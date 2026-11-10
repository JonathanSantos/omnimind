//! O único caminho de escrita de memória. Toda regra mora aqui.

use crate::analyzer::fold;
use crate::analyzer::Analyzer;
use crate::brain::{
    content_fingerprint, make_rec, quarantine_reason, Brain, MemRec, Plan, PlanOp, State,
};
use crate::error::{Error, Result};
use crate::inspect::{self, Findings};
use crate::journal::{self, Event, EventKind, Mutation};
use crate::model::*;
use crate::paths;
use crate::simhash::{self, containment, contradicts};
use crate::tags::{self, TagResult};
use crate::util;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

const MAX_TITLE: usize = 200;
const MAX_SUMMARY: usize = 400;
const MAX_BODY: usize = 20_000;
const MAX_ALIASES: usize = 24;
const MAX_TAGS: usize = 12;
pub const SESSION_GROWTH_CAP: u32 = 40;
const MIN_QUOTE: usize = 20;

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn nfc(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    s.nfc().collect()
}

pub fn norm_title(s: &str) -> String {
    let f = fold(&inspect::strip_invisible(s));
    f.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn rejected(reason: &str, hint: &str, rule: &str) -> Applied {
    Applied {
        outcome: Outcome::Rejected {
            reason: reason.to_string(),
            hint: hint.to_string(),
        },
        id: None,
        path: None,
        rule: rule.to_string(),
        hash: None,
        event: None,
        warnings: Vec::new(),
        neighbors: Vec::new(),
        tags: Vec::new(),
        tag_vocabulary: Vec::new(),
        current_text: None,
    }
}

fn base(outcome: Outcome, id: &str, rule: &str) -> Applied {
    Applied {
        id: Some(id.to_string()),
        ..rejected_like(outcome, rule)
    }
}

fn rejected_like(outcome: Outcome, rule: &str) -> Applied {
    Applied {
        outcome,
        ..rejected("", "", rule)
    }
}

/// Memórias vizinhas: mesma nota, depois mesmo tema.
pub(crate) fn neighbors_of(st: &State, m: &Memory, limit: usize, now: i64) -> Vec<Neighbor> {
    let mut out: Vec<(u8, f32, Neighbor)> = Vec::new();
    let me = simhash::Fingerprint::of(&m.title, &m.summary, "");
    for r in st.mems.values() {
        if r.memory.id == m.id {
            continue;
        }
        let state = st.state_of(r, now);
        if state == MemoryState::Quarantined {
            continue;
        }
        let same_note = m.origin.note.is_some() && m.origin.note == r.memory.origin.note;
        let j = simhash::jaccard_ids(&me.head, &r.fp.head);
        if !same_note && j < 0.12 {
            continue;
        }
        let relation = if same_note { "same_note" } else { "same_topic" };
        out.push((
            u8::from(!same_note),
            -j,
            Neighbor {
                id: r.memory.id.clone(),
                title: inspect::strip_invisible(&r.memory.title),
                state,
                relation: relation.to_string(),
            },
        ));
    }
    out.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then_with(|| a.2.id.cmp(&b.2.id))
    });
    out.into_iter().take(limit).map(|x| x.2).collect()
}

/// Vocabulário servido ao agente: nada de memória retida, nada que o detector barraria.
fn vocabulary(st: &State) -> Vec<String> {
    let mut set = BTreeSet::new();
    for r in st.mems.values().filter(|r| r.quarantine.is_none()) {
        for t in &r.memory.tags {
            let f = inspect::inspect_tag(t);
            if !f.suspicious() && f.secret.is_none() {
                set.insert(t.clone());
            }
        }
    }
    set.into_iter().collect()
}

/// Fusão automática só com conteúdo idêntico depois de normalizar.
fn same_content(m: &Memory, title: &str, summary: &str, body: &str) -> bool {
    if norm_title(&m.title) != norm_title(title) || norm_title(&m.summary) != norm_title(summary) {
        return false;
    }
    norm_title(body).is_empty() || norm_title(&m.body) == norm_title(body)
}

pub(crate) fn merge_key(a: &str, b: &str) -> String {
    if a < b {
        format!("merge:{a}:{b}")
    } else {
        format!("merge:{b}:{a}")
    }
}

pub(crate) fn identical(a: &Memory, b: &Memory) -> bool {
    norm_title(&a.title) == norm_title(&b.title)
        && norm_title(&a.summary) == norm_title(&b.summary)
        && norm_title(&a.body) == norm_title(&b.body)
}

fn meaningful_forms(an: &mut Analyzer, text: &str) -> Vec<Vec<String>> {
    an.query_words(text)
        .into_iter()
        .filter(|w| !w.bonus)
        .map(|w| w.forms.into_iter().map(|f| f.0).collect())
        .collect()
}

/// Data e atribuição são o formato que o MCP pede, não conteúdo a sustentar.
fn is_attribution(word: &str) -> bool {
    let w = fold(word.trim_matches(|c: char| !c.is_alphanumeric()));
    let date = w.len() == 10 && w.chars().all(|c| c.is_ascii_digit() || c == '-');
    date || [
        "pessoa", "person", "usuario", "user", "informou", "disse", "pediu", "afirmou", "said",
        "told", "stated", "asked",
    ]
    .contains(&w.as_str())
}

/// Sustentação: ao menos metade das palavras de `title + summary` está na citação.
pub(crate) fn quote_sustains(quote: &str, title: &str, summary: &str) -> bool {
    if quote.chars().count() < MIN_QUOTE {
        return false;
    }
    let (hit, total) = word_coverage(quote, title, summary);
    total > 0 && hit * 2 >= total
}

/// Quantas palavras significativas de `title + summary` aparecem em `quote`, e de quantas.
fn word_coverage(quote: &str, title: &str, summary: &str) -> (usize, usize) {
    let mut an = Analyzer::new();
    // O analisador corta em 16 palavras por consulta; aqui valem todas.
    let mut quoted: Vec<String> = Vec::new();
    an.index_terms(quote, &mut |t| quoted.push(t.to_string()));
    let mut words: Vec<Vec<String>> = Vec::new();
    for part in format!("{title} {summary}")
        .split_whitespace()
        .filter(|w| !is_attribution(w))
        .collect::<Vec<_>>()
        .chunks(12)
    {
        words.extend(meaningful_forms(&mut an, &part.join(" ")));
    }
    let hit = words
        .iter()
        .filter(|forms| forms.iter().any(|f| quoted.contains(f)))
        .count();
    (hit, words.len())
}

/// Três palavras a menos já não são ajuste de texto: pode haver outro fato indo embora junto.
const LOST_WORDS: usize = 3;

/// Substituir apaga a memória inteira, e a IA nem sempre lembra o que mais estava nela.
fn lost_facts(old: &Memory, title: &str, summary: &str) -> Option<String> {
    let new = format!("{title} {summary}");
    let (hit, total) = word_coverage(&new, "", &old.summary);
    (total - hit >= LOST_WORDS).then(|| {
        let was: String = old.summary.chars().take(200).collect();
        format!("the replaced memory also said: \"{was}\". A fact that still holds needs its own memory")
    })
}

const RECENT_SIGNALS: usize = 20;
const UNSUSTAINED: &str = "quote does not sustain this text: stored as agent, not as the person. Pass their literal words in quote and keep title and summary close to them; added context goes in body";

fn claims_person(d: &MemoryDraft) -> bool {
    matches!(
        d.origin_kind,
        Some(OriginKind::Correction | OriginKind::Request)
    )
}

/// Onde a memória está agora: editar o título renomeia o arquivo depois do evento.
fn file_of(st: &State, e: &Event) -> String {
    e.target
        .as_ref()
        .and_then(|t| st.mems.get(t))
        .map(|r| r.path.clone())
        .or_else(|| e.path.clone())
        .unwrap_or_default()
}

struct Prepared {
    title: String,
    summary: String,
    body: String,
    aliases: Vec<String>,
    tags: Vec<String>,
    quote: Option<String>,
    warnings: Vec<String>,
}

impl Brain {
    /// Aplica uma mudança. Único caminho para app, MCP, CLI e hooks.
    pub fn apply(&self, change: Change, actor: Actor) -> Result<Applied> {
        let _lock = self.write_lock()?;
        let mut st = self.lock_state();
        self.refresh(&mut st, None, false)?;
        while !st.pending.is_empty() {
            self.refresh(&mut st, None, true)?;
        }
        self.refresh_memories(&mut st)?;
        match change {
            Change::Write(d) => self.apply_write(&mut st, d, &actor),
            Change::Archive {
                id,
                base_hash,
                reason,
            } => self.set_flag(&mut st, &id, base_hash, reason, &actor, Flag::Archive),
            Change::Restore {
                id,
                base_hash,
                reason,
            } => self.set_flag(&mut st, &id, base_hash, reason, &actor, Flag::Restore),
            Change::Promote {
                id,
                base_hash,
                reason,
            } => {
                if actor.kind != ActorKind::Person {
                    return Ok(rejected(
                        "promotion_requires_person",
                        "Only the person can lift a quarantine, from the app or the CLI.",
                        "promote",
                    ));
                }
                self.set_flag(&mut st, &id, base_hash, reason, &actor, Flag::Promote)
            }
            Change::CorrectLine {
                id,
                base_hash,
                text,
            } => self.correct_line(&mut st, &id, base_hash, &text, &actor),
            Change::Merge {
                into,
                from,
                base_hash,
                reason,
            } => self.merge(&mut st, &into, &from, base_hash, reason, &actor),
            Change::AddAlias { id, alias, reason } => {
                self.add_alias(&mut st, &id, &alias, reason, &actor)
            }
        }
    }

    /// Sob a trava: relê `memories/` e o diário. Notas ficam com a última varredura.
    fn refresh_memories(&self, st: &mut State) -> Result<()> {
        self.refresh(st, None, true)?;
        st.view.catch_up(self.root());
        Ok(())
    }

    fn inspect_draft(&self, d: &MemoryDraft) -> Findings {
        let mut f = Findings::default();
        for field in [&d.title, &d.summary, &d.body] {
            f.merge(inspect::inspect(field));
        }
        for field in &d.aliases {
            f.merge(inspect::inspect(field));
        }
        for field in &d.tags {
            f.merge(inspect::inspect_tag(field));
        }
        for field in [
            &d.key,
            &d.quote,
            &d.note,
            &d.reason,
            &d.expires,
            &d.base_hash,
        ]
        .into_iter()
        .flatten()
        {
            f.merge(inspect::inspect(field));
        }
        for field in &d.supersedes {
            f.merge(inspect::inspect(field));
        }
        f
    }

    fn journal_decision(
        &self,
        kind: EventKind,
        target: Option<&str>,
        rule: &str,
        reason: &str,
        actor: &Actor,
    ) {
        let mut e = self.event(kind, Some(actor));
        e.target = target.map(str::to_string);
        e.rule = Some(rule.to_string());
        e.reason = Some(inspect::redact(&inspect::strip_invisible(reason)));
        let _ = self.append(&mut e);
    }

    fn reject(&self, st: &State, r: Applied, actor: &Actor) -> Result<Applied> {
        let _ = st;
        let reason = match &r.outcome {
            Outcome::Rejected { reason, .. } => reason.clone(),
            _ => String::new(),
        };
        self.journal_decision(
            EventKind::Rejected,
            r.id.as_deref(),
            &r.rule,
            &reason,
            actor,
        );
        Ok(r)
    }

    fn prepare(
        &self,
        st: &State,
        d: &MemoryDraft,
        strip: bool,
        tag_warnings: &mut Vec<String>,
        actor: &Actor,
    ) -> Prepared {
        let clean = |s: &str| {
            let s = if strip {
                inspect::strip_invisible(s)
            } else {
                s.to_string()
            };
            nfc(&s)
        };
        let vocab = vocabulary(st);
        let mut aliases = Vec::new();
        for a in &d.aliases {
            let a = collapse(&clean(a));
            if !a.is_empty() && !aliases.contains(&a) && aliases.len() < MAX_ALIASES {
                aliases.push(a);
            }
        }
        let mut tag_list = Vec::new();
        for t in &d.tags {
            match tags::normalize(t) {
                TagResult::Ok(n) => {
                    let n = tags::canonical(&n, &vocab);
                    if !tag_list.contains(&n) && tag_list.len() < MAX_TAGS {
                        tag_list.push(n);
                    }
                }
                TagResult::RepeatsType => {
                    tag_warnings.push(format!(
                        "tag refused, repeats the type: {}",
                        collapse(&clean(t))
                    ));
                }
                TagResult::Invalid => {
                    tag_warnings.push("tag refused, invalid".to_string());
                }
            }
        }
        if let Some(cwd) = actor.cwd.as_deref() {
            let project = crate::brain::project_from_cwd(cwd);
            let mem_type = d
                .mem_type
                .or_else(|| d.id.as_ref().and_then(|i| st.mems.get(i)).map(|r| r.memory.mem_type));
            // Preferência é da pessoa, não da pasta: só fica no projeto se o texto o citar.
            let said = fold(&format!(
                "{} {} {}",
                d.title,
                d.summary,
                d.quote.as_deref().unwrap_or("")
            ));
            let named = said.contains(&project) || said.contains(&project.replace('-', " "));
            let p = format!("project/{project}");
            if (mem_type != Some(MemoryType::Preference) || named) && !tag_list.contains(&p) {
                tag_list.push(p);
            }
        }
        Prepared {
            title: collapse(&clean(&d.title)),
            summary: collapse(&clean(&d.summary)),
            body: clean(&d.body).trim().to_string(),
            aliases,
            tags: tag_list,
            quote: d
                .quote
                .as_ref()
                .map(|q| collapse(&clean(q)))
                .filter(|q| !q.is_empty()),
            warnings: Vec::new(),
        }
    }

    /// Citação literal de sinal da pessoa. O detector olha o trecho citado: uma colagem suspeita
    /// no resto da mensagem não pode calar o que a pessoa disse ao lado dela.
    fn quote_is_signal(&self, st: &State, quote: &str) -> bool {
        let q = fold(quote);
        q.chars().count() >= MIN_QUOTE
            && inspect::instruction_like(quote).is_none()
            && st
                .view
                .signals
                .iter()
                .any(|s| !s.rejected && fold(&s.text).contains(&q))
    }

    /// Sinal literal e sustentação: só assim um agente escreve em nome da pessoa.
    fn person_backs(&self, st: &State, quote: Option<&str>, title: &str, summary: &str) -> bool {
        quote.is_some_and(|q| self.quote_is_signal(st, q) && quote_sustains(q, title, summary))
    }

    /// Frase da pessoa que este texto registra, entre os sinais recentes.
    fn signal_behind(&self, st: &State, title: &str, summary: &str) -> Option<String> {
        let text = format!("{title} {summary}");
        st.view
            .signals
            .iter()
            .rev()
            .take(RECENT_SIGNALS)
            .find(|s| {
                !s.rejected
                    && inspect::instruction_like(&s.text).is_none()
                    && quote_sustains(&text, &s.text, "")
            })
            .map(|s| s.text.clone())
    }

    /// A autoridade vem do transporte e do diário, nunca de campo da requisição.
    fn authority_for(
        &self,
        st: &State,
        d: &MemoryDraft,
        actor: &Actor,
        quote: Option<&str>,
        title: &str,
        summary: &str,
    ) -> Authority {
        match actor.kind {
            ActorKind::Person => Authority::Person,
            ActorKind::External => Authority::External,
            ActorKind::Import => d
                .import
                .as_ref()
                .and_then(|i| i.authority)
                .unwrap_or(Authority::Agent),
            ActorKind::System => {
                if d.origin_kind == Some(OriginKind::Correction) {
                    Authority::Person
                } else {
                    Authority::Agent
                }
            }
            ActorKind::Agent | ActorKind::Hook => {
                if d.origin_kind == Some(OriginKind::Tool) {
                    return Authority::External;
                }
                let spoken = matches!(
                    d.origin_kind,
                    Some(OriginKind::Correction | OriginKind::Request)
                );
                if spoken && self.person_backs(st, quote, title, summary) {
                    Authority::Person
                } else {
                    Authority::Agent
                }
            }
        }
    }

    fn apply_write(&self, st: &mut State, d: MemoryDraft, actor: &Actor) -> Result<Applied> {
        let now = util::now_micros();
        // Passo 2: texto bruto, em todos os campos. Segredo nunca vai nem para o diário.
        let findings = self.inspect_draft(&d);
        if let Some(rule) = &findings.secret {
            return self.reject(
                st,
                rejected(
                    "secret",
                    "Remove the credential. Secrets are never stored.",
                    &format!("secret:{rule}"),
                ),
                actor,
            );
        }
        // Passo 1: chave de idempotência.
        let key = d
            .key
            .clone()
            .map(|k| format!("k:{}", util::hash_str(&k)))
            .unwrap_or_else(|| {
                format!(
                    "c:{}",
                    util::hash_str(&format!(
                        "{}\u{0}{}\u{0}{}",
                        norm_title(&d.title),
                        collapse(&d.summary),
                        collapse(&d.body)
                    ))
                )
            });
        if d.id.is_none() {
            if let Some(id) = st.view.keys.get(&key).cloned() {
                if let Some(rec) = st.mems.get(&id) {
                    let same = same_content(&rec.memory, &d.title, &d.summary, &d.body);
                    if d.key.is_some() && !same {
                        return self.reject(
                            st,
                            rejected(
                                "key_reused",
                                "This key already belongs to another text. Use a new key.",
                                "idempotency",
                            ),
                            actor,
                        );
                    }
                    let state = st.state_of(rec, now);
                    let new_alias = d
                        .aliases
                        .iter()
                        .any(|x| !rec.memory.aliases.iter().any(|a| fold(a) == fold(x)));
                    // Alias novo segue até a fusão: repetir a escrita não pode perdê-lo.
                    if !new_alias {
                        let mut a = base(Outcome::Unchanged, &id, "idempotency");
                        a.hash = Some(rec.hash.clone());
                        a.tag_vocabulary = vocabulary(st);
                        // Fato já substituído não renasce, mas quem escreveu precisa saber.
                        if !matches!(state, MemoryState::Active | MemoryState::Stale) {
                            a.warnings.push(format!(
                                "this exact text exists as {}: nothing was stored",
                                state.as_str()
                            ));
                        }
                        return Ok(a);
                    }
                }
            }
        }

        let is_person = actor.kind == ActorKind::Person;
        let mut warnings = Vec::new();
        let p_strip = is_person;
        let mut prepared = self.prepare(st, &d, p_strip, &mut warnings, actor);
        // Sem citação, o diário ainda sabe a frase: assim o sinal fecha e a origem fica registrada.
        if prepared.quote.is_none()
            && matches!(actor.kind, ActorKind::Agent | ActorKind::Hook)
            && (claims_person(&d) || d.mem_type == Some(MemoryType::Preference))
        {
            prepared.quote = self.signal_behind(st, &prepared.title, &prepared.summary);
        }
        if prepared.title.is_empty() {
            return self.reject(
                st,
                rejected(
                    "title_required",
                    "Give the memory a short title.",
                    "validate",
                ),
                actor,
            );
        }
        if prepared.summary.is_empty() && prepared.body.is_empty() {
            return self.reject(
                st,
                rejected(
                    "content_required",
                    "Write a summary or a body with the fact, its date and its origin.",
                    "validate",
                ),
                actor,
            );
        }
        for (name, len, max) in [
            ("title", prepared.title.chars().count(), MAX_TITLE),
            ("summary", prepared.summary.chars().count(), MAX_SUMMARY),
            ("body", prepared.body.chars().count(), MAX_BODY),
        ] {
            if len > max {
                return self.reject(
                    st,
                    rejected(
                        &format!("{name}_too_long"),
                        &format!("Shorten {name} to at most {max} characters."),
                        "validate",
                    ),
                    actor,
                );
            }
        }
        if let Some(e) = &d.expires {
            if util::parse_ts(e).is_none() {
                return self.reject(
                    st,
                    rejected("bad_expires", "Use YYYY-MM-DD.", "validate"),
                    actor,
                );
            }
        }

        // Alteração: compara e troca.
        if let Some(id) = &d.id {
            return self.update_memory(
                st,
                id.clone(),
                &d,
                prepared,
                findings,
                actor,
                key,
                warnings,
            );
        }

        let Some(mem_type) = d.mem_type else {
            return self.reject(
                st,
                rejected(
                    "type_required",
                    "Use one of decision, procedure, concept, episode, preference.",
                    "validate",
                ),
                actor,
            );
        };
        for s in &d.supersedes {
            if !st.mems.contains_key(s) {
                return self.reject(
                    st,
                    rejected(
                        "unknown_supersedes",
                        "Use ids returned by memory_search or memory_write neighbors.",
                        "validate",
                    ),
                    actor,
                );
            }
        }

        // Fonte: caminho seguro, nota lida nesta sessão, hash calculado aqui.
        let mut note_hash = None;
        let mut note_text = None;
        if let Some(n) = &d.note {
            let rel = match paths::check_note_rel(n) {
                Ok(r) => r,
                Err(_) => {
                    return self.reject(
                        st,
                        rejected(
                            "bad_note_path",
                            "Use the note path as returned by the search.",
                            "source",
                        ),
                        actor,
                    )
                }
            };
            let full = match paths::safe_join(self.root(), &rel) {
                Ok(f) => f,
                Err(_) => {
                    return self.reject(
                        st,
                        rejected("bad_note_path", "Symlinks are refused.", "source"),
                        actor,
                    )
                }
            };
            let Ok(bytes) = fs::read(&full) else {
                return self.reject(
                    st,
                    rejected(
                        "note_not_found",
                        "The source note does not exist.",
                        "source",
                    ),
                    actor,
                );
            };
            let h = util::hash_bytes(&bytes);
            if matches!(actor.kind, ActorKind::Agent | ActorKind::Hook) {
                let session = actor
                    .session
                    .clone()
                    .unwrap_or_else(|| self.session().to_string());
                let was_read = st
                    .view
                    .session_reads
                    .get(&session)
                    .and_then(|m| m.get(&rel))
                    == Some(&h);
                if !was_read {
                    return self.reject(
                        st,
                        rejected("note_not_read", "Open the whole note with memory_read first. A search snippet is not enough.", "coverage"),
                        actor,
                    );
                }
            }
            note_hash = Some(h);
            note_text = Some(String::from_utf8_lossy(&bytes).into_owned());
        }

        // Preferência exige frase explícita da pessoa.
        if mem_type == MemoryType::Preference
            && !matches!(
                actor.kind,
                ActorKind::Person | ActorKind::Import | ActorKind::System
            )
        {
            let in_signal = self.person_backs(
                st,
                prepared.quote.as_deref(),
                &prepared.title,
                &prepared.summary,
            );
            let in_note = match (&prepared.quote, &note_text) {
                (Some(q), Some(t)) => {
                    q.chars().count() >= MIN_QUOTE
                        && fold(t).contains(&fold(q))
                        && quote_sustains(q, &prepared.title, &prepared.summary)
                }
                _ => false,
            };
            if !in_signal && !in_note {
                return self.reject(
                    st,
                    rejected(
                        "preference_needs_quote",
                        "No signal in the journal sustains this quote. Keep title and summary close to the person's literal words, or store it now as concept or episode so it is not lost, and tell the person.",
                        "preference",
                    ),
                    actor,
                );
            }
        }

        // Passo 6: taint.
        let mut authority = self.authority_for(
            st,
            &d,
            actor,
            prepared.quote.as_deref(),
            &prepared.title,
            &prepared.summary,
        );
        if claims_person(&d)
            && authority != Authority::Person
            && matches!(actor.kind, ActorKind::Agent | ActorKind::Hook)
        {
            warnings.push(UNSUSTAINED.to_string());
        }
        for s in &d.supersedes {
            if let Some(old) = st.mems.get(s) {
                if matches!(actor.kind, ActorKind::Agent | ActorKind::Hook) {
                    warnings.extend(lost_facts(&old.memory, &prepared.title, &prepared.summary));
                }
                // Vale para `consolidation` também: nenhum campo da requisição supera a pessoa.
                if old.memory.authority > authority {
                    return self.reject(
                        st,
                        rejected(
                            "cannot_supersede_higher_authority",
                            "This memory came from the person. Replacing it needs a quote of the person that sustains the new title and summary; added context goes in body. Otherwise write a new memory: the agenda opens the conflict.",
                            "authority",
                        ),
                        actor,
                    );
                }
                if d.origin_kind == Some(OriginKind::Consolidation) {
                    authority = authority.min(old.memory.authority);
                }
            }
        }

        // Passo 3: decide.
        let suspicious = findings.suspicious();
        let mut quarantine: Option<String> = None;
        if suspicious {
            let labels = findings.labels().join(", ");
            if is_person {
                // Invisível já saiu em `prepare`. Instrução fica, com aviso.
                warnings.push(format!("stored with warnings: {labels}"));
            } else {
                quarantine = Some(labels);
            }
        }
        if authority == Authority::External && quarantine.is_none() {
            quarantine = Some("external_authority".into());
        }

        // Só repete a nota: sem síntese.
        if let Some(t) = &note_text {
            let mine = format!("{} {}", prepared.summary, prepared.body);
            let long = mine.chars().count() > 200;
            if long
                && actor.kind != ActorKind::Import
                && containment(&mine, t) > 0.9
                && mine.chars().count() as f32 > t.chars().count() as f32 * 0.6
            {
                return self.reject(
                    st,
                    rejected(
                        "repeats_note",
                        "The note is already read by the search. Write the synthesis, not a copy.",
                        "gate",
                    ),
                    actor,
                );
            }
        }

        // Teto de crescimento por sessão.
        if matches!(actor.kind, ActorKind::Agent | ActorKind::Hook) && d.supersedes.is_empty() {
            let session = actor
                .session
                .clone()
                .unwrap_or_else(|| self.session().to_string());
            let n = st.view.sessions.get(&session).map_or(0, |s| s.new_memories);
            if n >= SESSION_GROWTH_CAP {
                return self.reject(
                    st,
                    rejected(
                        "session_growth_limit",
                        "Over 40 new memories in this session. Write one summary instead of a list.",
                        "growth",
                    ),
                    actor,
                );
            }
        }

        // Passo 5: colisão, só para o que não está em quarentena. Só conteúdo idêntico funde.
        let mut conflicts = Vec::new();
        let mut near_dups = Vec::new();
        if quarantine.is_none() {
            let body_text = format!("{} {}", prepared.summary, prepared.body);
            let fp = simhash::Fingerprint::of(&prepared.title, &prepared.summary, &prepared.body);
            let long_enough = fp.word_count >= 6;
            let new_title = norm_title(&prepared.title);
            let mut ids: Vec<String> = st.mems.keys().cloned().collect();
            ids.sort();
            for id in ids {
                let rec = &st.mems[&id];
                if d.supersedes.contains(&id) {
                    continue;
                }
                let state = st.state_of(rec, now);
                if !matches!(state, MemoryState::Active | MemoryState::Stale) {
                    continue;
                }
                let m = &rec.memory;
                let same_title = norm_title(&m.title) == new_title;
                let same_note =
                    d.note.is_some() && m.origin.note == d.note && m.mem_type == mem_type;
                let hash_near =
                    long_enough && simhash::distance(fp.sim_whole, rec.fp.sim_whole) <= 3;
                let j = simhash::jaccard_ids(&fp.whole, &rec.fp.whole);
                let same_type = m.mem_type == mem_type;
                let old_text = format!("{} {}", m.summary, m.body);
                if same_content(m, &prepared.title, &prepared.summary, &prepared.body) {
                    return self.merge_duplicate(st, &id, &prepared, actor, "exact_duplicate");
                }
                if same_note && state == MemoryState::Stale && (j >= 0.2 || same_title) {
                    let mut r = rejected(
                        "must_supersede",
                        &format!("The note changed after memory {id} was written. Send supersedes with that id."),
                        "supersedes",
                    );
                    r.neighbors = neighbors_of(st, m, 3, now);
                    r.neighbors.insert(
                        0,
                        Neighbor {
                            id: id.clone(),
                            title: inspect::strip_invisible(&m.title),
                            state,
                            relation: "same_note".into(),
                        },
                    );
                    return self.reject(st, r, actor);
                }
                if same_type && (same_title || hash_near || j >= 0.7) {
                    if contradicts(&old_text, &body_text) {
                        conflicts.push(id.clone());
                    } else {
                        near_dups.push(id.clone());
                    }
                }
            }
        }

        // Passo 7: grava.
        let id = match actor.kind {
            ActorKind::Import => d
                .import
                .as_ref()
                .and_then(|i| i.id.clone())
                .filter(|i| util::is_ulid(i) && !st.mems.contains_key(i))
                .unwrap_or_else(util::new_ulid),
            _ => util::new_ulid(),
        };
        let ts = util::now_ts_seconds();
        let imp = d.import.clone().unwrap_or_default();
        let mut memory = Memory {
            id: id.clone(),
            mem_type,
            title: prepared.title.clone(),
            summary: prepared.summary.clone(),
            aliases: std::mem::take(&mut prepared.aliases),
            tags: prepared.tags.clone(),
            authority,
            origin: Origin {
                kind: d.origin_kind.unwrap_or(if d.note.is_some() {
                    OriginKind::Note
                } else {
                    OriginKind::Session
                }),
                client: Some(imp.client.clone().unwrap_or_else(|| actor.client.clone())),
                session: imp.session.clone().or_else(|| {
                    Some(
                        actor
                            .session
                            .clone()
                            .unwrap_or_else(|| self.session().to_string()),
                    )
                }),
                quote: prepared.quote.take(),
                note: d.note.as_ref().and_then(|n| paths::check_note_rel(n).ok()),
                hash: imp.note_hash.clone().or(note_hash),
            },
            created: imp.created.clone().unwrap_or_else(|| ts.clone()),
            updated: imp.updated.clone().unwrap_or_else(|| ts.clone()),
            supersedes: d.supersedes.clone(),
            expires: d.expires.clone(),
            archived: imp.archived.clone(),
            quarantine: quarantine.clone(),
            promoted: None,
            promoted_hash: None,
            body: prepared.body.clone(),
            extra: String::new(),
        };
        // Texto da pessoa que o detector estranha fica com aviso, e a promoção é dela mesma.
        if is_person && quarantine_reason(&memory).is_some() {
            memory.promoted = Some(ts.clone());
            memory.promoted_hash = Some(content_fingerprint(&memory));
        }
        let rel = self.memory_path(st, &memory);
        let text = render_memory(&memory);
        let rule = if quarantine.is_some() {
            "quarantine"
        } else if !conflicts.is_empty() {
            "conflict"
        } else if !near_dups.is_empty() {
            "near_duplicate"
        } else {
            "direct"
        };
        let ev_kind = if quarantine.is_some() {
            EventKind::Quarantined
        } else {
            EventKind::Wrote
        };
        let event = self.commit(
            st,
            &rel,
            &memory.id,
            None,
            &text,
            ev_kind,
            rule,
            d.reason.as_deref(),
            Some(&key),
            None,
            actor,
        )?;
        for old in &d.supersedes {
            let mut e = self.event(EventKind::Superseded, Some(actor));
            e.target = Some(old.clone());
            e.reason = Some(format!("superseded by {id}"));
            e.undoes = None;
            let _ = self.append(&mut e);
        }
        st.view.catch_up(self.root());

        for c in &conflicts {
            warnings.push(format!(
                "conflict with {c}: stored, and the agenda lists it"
            ));
        }
        for c in &near_dups {
            warnings.push(format!(
                "close to {c}: stored apart, and the agenda lists it"
            ));
        }
        prepared.warnings = std::mem::take(&mut warnings);
        let outcome = match (&quarantine, d.supersedes.is_empty()) {
            (Some(r), _) => Outcome::Quarantined { reason: r.clone() },
            (None, false) => Outcome::Superseded {
                ids: d.supersedes.clone(),
            },
            (None, true) => Outcome::Written,
        };
        let mut a = base(outcome, &id, rule);
        a.path = st.mems.get(&id).map(|r| r.path.clone());
        a.event = Some(event);
        a.hash = st.mems.get(&id).map(|r| r.hash.clone());
        a.warnings = prepared.warnings;
        a.tags = prepared.tags;
        let vocab = vocabulary(st);
        a.tag_vocabulary = tags::nearest(&memory.tags, &memory.title, &vocab, 12);
        if quarantine.is_none() {
            a.neighbors = neighbors_of(st, &memory, 5, now);
        }
        Ok(a)
    }

    fn memory_path(&self, st: &State, m: &Memory) -> String {
        let dir = m.mem_type.as_str();
        let slug = util::slug(&m.title);
        let candidate = format!("memories/{dir}/{slug}.md");
        if !st.path_ids.contains_key(&candidate) && !self.root().join(&candidate).exists() {
            return candidate;
        }
        let suffix = &m.id[m.id.len() - 6..];
        format!("memories/{dir}/{slug}-{}.md", suffix.to_lowercase())
    }

    /// Temporário, `sync_all`, `rename`, evento, índice. Devolve o id do evento.
    #[allow(clippy::too_many_arguments)]
    fn commit(
        &self,
        st: &mut State,
        rel: &str,
        id: &str,
        before: Option<(String, String)>,
        text: &str,
        kind: EventKind,
        rule: &str,
        reason: Option<&str>,
        key: Option<&str>,
        undoes: Option<&str>,
        actor: &Actor,
    ) -> Result<String> {
        let full = paths::safe_join(self.root(), rel)?;
        paths::create_parents(self.root(), &full)?;
        util::atomic_write(&self.tmp_dir()?, &full, text.as_bytes(), None)?;
        let hash = util::hash_str(text);
        let mut e = self.event(kind, Some(actor));
        e.target = Some(id.to_string());
        e.path = Some(rel.to_string());
        if let Some((text, h)) = before {
            e.before = Some(text);
            e.before_hash = Some(h);
        }
        e.after = Some(text.to_string());
        e.after_hash = Some(hash);
        e.rule = Some(rule.to_string());
        e.reason = reason.map(|r| inspect::redact(&inspect::strip_invisible(r)));
        e.key = key.map(str::to_string);
        e.undoes = undoes.map(str::to_string);
        if let Some(cwd) = &actor.cwd {
            e.project = Some(crate::brain::project_from_cwd(cwd));
        }
        self.append(&mut e)?;
        self.ingest_memory_now(st, rel, text)?;
        st.dirty = true;
        self.bump_generation(st);
        Ok(e.id)
    }

    fn ingest_memory_now(&self, st: &mut State, rel: &str, text: &str) -> Result<()> {
        let full = self.root().join(rel);
        let meta = fs::symlink_metadata(&full)?;
        let m = parse_memory(text).map_err(Error::Invalid)?;
        let id = m.id.clone();
        if let Some(prev) = st.mems.get(&id) {
            if prev.path != rel {
                let p = prev.path.clone();
                st.path_ids.remove(&p);
            }
        }
        let rec = make_rec(
            m,
            rel,
            util::hash_str(text),
            crate::brain::sig_for(&meta),
            crate::brain::now_nanos(),
            false,
        );
        self.index_memory(st, &rec);
        st.path_ids.insert(rel.to_string(), id.clone());
        st.view.last_hash.insert(id.clone(), rec.hash.clone());
        st.view.product_hash.insert(id.clone(), rec.hash.clone());
        self.ledger_append(&id, &rec.hash);
        st.mems.insert(id, rec);
        st.recompute_superseded(util::now_micros());
        Ok(())
    }

    fn ledger_append(&self, id: &str, hash: &str) {
        use std::io::Write;
        let path = self.root().join(".omnimind").join("ledger");
        if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(format!("{id} {hash}\n").as_bytes());
        }
    }

    fn current_text(&self, rec: &MemRec) -> Result<(String, String)> {
        let text = fs::read_to_string(self.root().join(&rec.path))?;
        let h = util::hash_str(&text);
        Ok((text, h))
    }

    /// Compara e troca: devolve o texto atual quando diverge.
    fn cas(
        &self,
        st: &State,
        id: &str,
        base_hash: Option<&str>,
    ) -> Result<std::result::Result<(String, String), Applied>> {
        let Some(rec) = st.mems.get(id) else {
            let mut r = rejected("not_found", "Unknown id.", "cas");
            r.id = Some(id.to_string());
            return Ok(Err(r));
        };
        let (text, h) = self.current_text(rec)?;
        if base_hash != Some(h.as_str()) {
            let mut r = rejected(
                "base_hash_mismatch",
                "The memory changed or base_hash is missing. Read current_text, then resend with its hash.",
                "cas",
            );
            r.id = Some(id.to_string());
            // Memória retida e segredo: só o hash, nunca o texto.
            if rec.quarantine.is_none() && inspect::find_secret(&text).is_none() {
                r.current_text = Some(inspect::strip_invisible(&text));
            }
            r.hash = Some(h);
            return Ok(Err(r));
        }
        Ok(Ok((text, h)))
    }

    #[allow(clippy::too_many_arguments)]
    fn update_memory(
        &self,
        st: &mut State,
        id: String,
        d: &MemoryDraft,
        prepared: Prepared,
        findings: Findings,
        actor: &Actor,
        key: String,
        mut warnings: Vec<String>,
    ) -> Result<Applied> {
        if let Some(r) = self.held_by_quarantine(st, &id, actor) {
            return self.reject(st, r, actor);
        }
        let (before_text, before_hash) = match self.cas(st, &id, d.base_hash.as_deref())? {
            Ok(x) => x,
            Err(r) => return self.reject(st, r, actor),
        };
        let rec = &st.mems[&id];
        let mut m = rec.memory.clone();
        let mut rel = rec.path.clone();
        let backed = self.person_backs(
            st,
            prepared.quote.as_deref(),
            &prepared.title,
            &prepared.summary,
        );
        if actor.kind != ActorKind::Person && m.authority == Authority::Person && !backed {
            return self.reject(
                st,
                rejected(
                    "cannot_edit_person_memory",
                    "This memory came from the person. Send a quote of the person that sustains the new title and summary, with added context in body, or write a new memory: the agenda opens the conflict.",
                    "authority",
                ),
                actor,
            );
        }
        let quarantined = findings.suspicious() && actor.kind != ActorKind::Person;
        if quarantined {
            m.quarantine = Some(findings.labels().join(", "));
        }
        if findings.suspicious() && actor.kind == ActorKind::Person {
            warnings.push(format!(
                "stored with warnings: {}",
                findings.labels().join(", ")
            ));
        }
        let renamed = util::slug(&m.title) != util::slug(&prepared.title);
        if matches!(actor.kind, ActorKind::Agent | ActorKind::Hook) {
            warnings.extend(lost_facts(&m, &prepared.title, &prepared.summary));
        }
        m.title = prepared.title;
        m.summary = prepared.summary;
        m.body = prepared.body;
        if !prepared.aliases.is_empty() {
            m.aliases = prepared.aliases;
        }
        if !prepared.tags.is_empty() {
            m.tags = prepared.tags;
        }
        if actor.kind == ActorKind::Person
            || (backed
                && claims_person(d)
                && m.authority == Authority::Agent
                && matches!(actor.kind, ActorKind::Agent | ActorKind::Hook))
        {
            m.authority = Authority::Person;
        }
        if claims_person(d)
            && m.authority != Authority::Person
            && matches!(actor.kind, ActorKind::Agent | ActorKind::Hook)
        {
            warnings.push(UNSUSTAINED.to_string());
        }
        if prepared.quote.is_some() {
            m.origin.quote = prepared.quote;
        }
        if d.expires.is_some() {
            m.expires = d.expires.clone();
        }
        m.updated = util::now_ts_seconds();
        m.promoted = None;
        m.promoted_hash = None;
        if actor.kind == ActorKind::Person {
            m.quarantine = None;
            if quarantine_reason(&m).is_some() {
                m.promoted = Some(m.updated.clone());
                m.promoted_hash = Some(content_fingerprint(&m));
            }
        }
        let text = render_memory(&m);
        if text == before_text {
            let mut a = base(Outcome::Unchanged, &id, "cas");
            a.hash = Some(before_hash);
            return Ok(a);
        }
        let kind = if quarantined {
            EventKind::Quarantined
        } else {
            EventKind::Wrote
        };
        let rule = if quarantined { "quarantine" } else { "edit" };
        // O arquivo acompanha o título. Renomeia antes do evento: uma queda no meio só deixa
        // o texto antigo no nome novo, nunca dois arquivos com o mesmo id.
        if renamed && !quarantined {
            let to = self.memory_path(st, &m);
            fs::rename(
                paths::safe_join(self.root(), &rel)?,
                paths::safe_join(self.root(), &to)?,
            )?;
            rel = to;
        }
        let event = self.commit(
            st,
            &rel,
            &id,
            Some((before_text, before_hash)),
            &text,
            kind,
            rule,
            d.reason.as_deref(),
            Some(&key),
            None,
            actor,
        )?;
        st.view.catch_up(self.root());
        let outcome = if quarantined {
            Outcome::Quarantined {
                reason: findings.labels().join(", "),
            }
        } else {
            Outcome::Written
        };
        let mut a = base(outcome, &id, rule);
        a.path = Some(rel);
        a.event = Some(event);
        a.hash = st.mems.get(&id).map(|r| r.hash.clone());
        a.warnings = warnings;
        a.tags = m.tags.clone();
        a.tag_vocabulary = tags::nearest(&m.tags, &m.title, &vocabulary(st), 12);
        a.neighbors = neighbors_of(st, &m, 5, util::now_micros());
        Ok(a)
    }

    /// Duplicata: mantém o texto existente e soma só aliases novos.
    fn merge_duplicate(
        &self,
        st: &mut State,
        into: &str,
        p: &Prepared,
        actor: &Actor,
        rule: &str,
    ) -> Result<Applied> {
        let rec = &st.mems[into];
        let mut m = rec.memory.clone();
        let rel = rec.path.clone();
        let (before, before_hash) = self.current_text(rec)?;
        let before_aliases = m.aliases.len();
        let before_tags = m.tags.len();
        for a in &p.aliases {
            if !m.aliases.iter().any(|x| fold(x) == fold(a)) && m.aliases.len() < MAX_ALIASES {
                m.aliases.push(a.clone());
            }
        }
        for t in &p.tags {
            if !m.tags.contains(t) && m.tags.len() < MAX_TAGS {
                m.tags.push(t.clone());
            }
        }
        let mut a = base(
            Outcome::Merged {
                into: into.to_string(),
            },
            into,
            rule,
        );
        if m.aliases.len() != before_aliases || m.tags.len() != before_tags {
            m.updated = util::now_ts_seconds();
            let text = render_memory(&m);
            let event = self.commit(
                st,
                &rel,
                into,
                Some((before, before_hash)),
                &text,
                EventKind::Merged,
                rule,
                None,
                None,
                None,
                actor,
            )?;
            st.view.catch_up(self.root());
            a.event = Some(event);
        } else {
            a.outcome = Outcome::Unchanged;
        }
        a.hash = st.mems.get(into).map(|r| r.hash.clone());
        a.tag_vocabulary = vocabulary(st);
        Ok(a)
    }

    /// Agente não mexe no que a pessoa ainda vai julgar. Resposta só com o hash.
    fn held_by_quarantine(&self, st: &State, id: &str, actor: &Actor) -> Option<Applied> {
        let rec = st.mems.get(id)?;
        if rec.quarantine.is_none() || actor.kind == ActorKind::Person {
            return None;
        }
        let mut r = rejected(
            "memory_held",
            "This memory is held for the person to review. Write a new memory instead.",
            "quarantine",
        );
        r.id = Some(id.to_string());
        r.hash = Some(rec.hash.clone());
        Some(r)
    }

    fn set_flag(
        &self,
        st: &mut State,
        id: &str,
        base_hash: Option<String>,
        reason: Option<String>,
        actor: &Actor,
        flag: Flag,
    ) -> Result<Applied> {
        if let Some(r) = self.held_by_quarantine(st, id, actor) {
            return self.reject(st, r, actor);
        }
        let (before, h) = match self.cas(st, id, base_hash.as_deref())? {
            Ok(x) => x,
            Err(r) => return self.reject(st, r, actor),
        };
        let rec = &st.mems[id];
        if flag != Flag::Promote
            && actor.kind != ActorKind::Person
            && rec.memory.authority == Authority::Person
        {
            return self.reject(
                st,
                rejected(
                    "cannot_edit_person_memory",
                    "Only the person can archive or restore this memory.",
                    "authority",
                ),
                actor,
            );
        }
        let mut m = rec.memory.clone();
        let rel = rec.path.clone();
        let (kind, rule) = match flag {
            Flag::Archive => {
                m.archived = Some(util::today());
                (EventKind::Archived, "archive")
            }
            Flag::Restore => {
                m.archived = None;
                (EventKind::Restored, "restore")
            }
            Flag::Promote => {
                m.quarantine = None;
                m.promoted = Some(util::now_ts_seconds());
                m.promoted_hash = Some(content_fingerprint(&m));
                (EventKind::Promoted, "promote")
            }
        };
        m.updated = util::now_ts_seconds();
        let text = render_memory(&m);
        let event = self.commit(
            st,
            &rel,
            id,
            Some((before, h)),
            &text,
            kind,
            rule,
            reason.as_deref(),
            None,
            None,
            actor,
        )?;
        st.view.catch_up(self.root());
        let mut a = base(Outcome::Written, id, rule);
        a.path = Some(rel);
        a.event = Some(event);
        a.hash = st.mems.get(id).map(|r| r.hash.clone());
        Ok(a)
    }

    fn correct_line(
        &self,
        st: &mut State,
        id: &str,
        base_hash: Option<String>,
        text: &str,
        actor: &Actor,
    ) -> Result<Applied> {
        if actor.kind != ActorKind::Person {
            return self.reject(
                st,
                rejected(
                    "correction_requires_person",
                    "Portrait lines are corrected by the person.",
                    "portrait",
                ),
                actor,
            );
        }
        if let Err(r) = self.cas(st, id, base_hash.as_deref())? {
            return self.reject(st, r, actor);
        }
        let old = st.mems[id].memory.clone();
        let draft = MemoryDraft {
            mem_type: Some(old.mem_type),
            title: old.title.clone(),
            summary: text.to_string(),
            body: format!("{} (a pessoa corrigiu em {})", text, util::today()),
            aliases: old.aliases.clone(),
            tags: old.tags.clone(),
            origin_kind: Some(OriginKind::Correction),
            quote: Some(text.to_string()),
            supersedes: vec![id.to_string()],
            reason: Some("portrait line edited".into()),
            ..Default::default()
        };
        self.apply_write(st, draft, actor)
    }

    fn merge(
        &self,
        st: &mut State,
        into: &str,
        from: &str,
        base_hash: Option<String>,
        reason: Option<String>,
        actor: &Actor,
    ) -> Result<Applied> {
        if !st.mems.contains_key(from) {
            return self.reject(st, rejected("not_found", "Unknown id.", "merge"), actor);
        }
        let (before, h) = match self.cas(st, into, base_hash.as_deref())? {
            Ok(x) => x,
            Err(r) => return self.reject(st, r, actor),
        };
        let other = st.mems[from].memory.clone();
        let rec = &st.mems[into];
        if other.authority > rec.memory.authority {
            return self.reject(
                st,
                rejected(
                    "authority_conflict",
                    "Merge into the memory with the highest authority.",
                    "merge",
                ),
                actor,
            );
        }
        let mut m = rec.memory.clone();
        let rel = rec.path.clone();
        for a in other.aliases.iter().chain(std::iter::once(&other.title)) {
            if !m.aliases.contains(a)
                && norm_title(a) != norm_title(&m.title)
                && m.aliases.len() < MAX_ALIASES
            {
                m.aliases.push(a.clone());
            }
        }
        if !m.supersedes.iter().any(|s| s == from) {
            m.supersedes.push(from.to_string());
        }
        m.updated = util::now_ts_seconds();
        let text = render_memory(&m);
        let event = self.commit(
            st,
            &rel,
            into,
            Some((before, h)),
            &text,
            EventKind::Merged,
            "merge",
            reason.as_deref(),
            Some(&merge_key(into, from)),
            None,
            actor,
        )?;
        st.view.catch_up(self.root());
        let mut a = base(
            Outcome::Merged {
                into: into.to_string(),
            },
            into,
            "merge",
        );
        a.event = Some(event);
        a.hash = st.mems.get(into).map(|r| r.hash.clone());
        Ok(a)
    }

    fn add_alias(
        &self,
        st: &mut State,
        id: &str,
        alias: &str,
        reason: Option<String>,
        actor: &Actor,
    ) -> Result<Applied> {
        let f = inspect::inspect(alias);
        if f.suspicious() || f.secret.is_some() {
            return self.reject(st, rejected("bad_alias", "Alias refused.", "alias"), actor);
        }
        let Some(rec) = st.mems.get(id) else {
            return self.reject(st, rejected("not_found", "Unknown id.", "alias"), actor);
        };
        let alias = collapse(alias);
        let mut m = rec.memory.clone();
        if m.aliases.iter().any(|a| fold(a) == fold(&alias)) || m.aliases.len() >= MAX_ALIASES {
            return Ok(base(Outcome::Unchanged, id, "alias"));
        }
        let rel = rec.path.clone();
        let (before, h) = self.current_text(rec)?;
        m.aliases.push(alias);
        m.updated = util::now_ts_seconds();
        let text = render_memory(&m);
        let event = self.commit(
            st,
            &rel,
            id,
            Some((before, h)),
            &text,
            EventKind::Wrote,
            "alias",
            reason.as_deref(),
            None,
            None,
            actor,
        )?;
        st.view.catch_up(self.root());
        let mut a = base(Outcome::Written, id, "alias");
        a.event = Some(event);
        a.hash = st.mems.get(id).map(|r| r.hash.clone());
        Ok(a)
    }

    /// Reforço explícito: `helped`, `hurt` ou `cited`. Exposição nunca reforça.
    pub fn reinforce(&self, id: &str, kind: EventKind) -> Result<()> {
        if !matches!(kind, EventKind::Helped | EventKind::Hurt | EventKind::Cited) {
            return Err(Error::Invalid(
                "reinforce takes helped, hurt or cited".into(),
            ));
        }
        let mut e = self.event(kind, None);
        e.target = Some(id.to_string());
        self.append(&mut e)
    }

    /// Mutações desta memória depois de `ev`, para oferecer desfazer em cadeia.
    fn later_mutations(st: &State, target: &str, after_event: &str) -> Vec<Mutation> {
        let mut seen = false;
        let mut out = Vec::new();
        for m in st.view.mutations.iter().filter(|m| m.target == target) {
            if seen && !st.view.undone.contains(&m.event) {
                out.push(m.clone());
            }
            if m.event == after_event {
                seen = true;
            }
        }
        out
    }

    /// Desfaz um evento. Gera `undid`; se a memória mudou depois, recusa e oferece a cadeia.
    pub fn undo(&self, event_id: &str) -> Result<UndoReport> {
        let _lock = self.write_lock()?;
        let mut st = self.lock_state();
        self.refresh(&mut st, None, true)?;
        st.view.catch_up(self.root());
        let (id, ev) = self.undo_target(&st, event_id)?;
        if ev.path.is_none() {
            return Err(Error::Invalid("event has no path".into()));
        }
        let path = file_of(&st, &ev);
        let cur = fs::read(self.root().join(&path))
            .ok()
            .map(|b| util::hash_bytes(&b));
        if cur != ev.after_hash {
            let chain: Vec<String> = Self::later_mutations(&st, &id, event_id)
                .into_iter()
                .rev()
                .map(|m| m.event)
                .collect();
            let reason = if chain.is_empty() {
                "the memory changed outside the product after this event".to_string()
            } else {
                "the memory changed after this event".to_string()
            };
            return Err(Error::UndoRefused { reason, chain });
        }
        let undid = self.apply_undo(&mut st, &ev, &id, &path)?;
        st.view.catch_up(self.root());
        Ok(UndoReport {
            undone: vec![event_id.to_string()],
            undid_events: vec![undid],
        })
    }

    fn undo_target(&self, st: &State, event_id: &str) -> Result<(String, Event)> {
        let ev = journal::find_event(self.root(), event_id)
            .ok_or_else(|| Error::NotFound(event_id.to_string()))?;
        if !ev.ev.is_mutation() {
            return Err(Error::UndoRefused {
                reason: format!("event of kind {:?} has nothing to undo", ev.ev).to_lowercase(),
                chain: Vec::new(),
            });
        }
        if st.view.undone.contains(event_id) {
            return Err(Error::UndoRefused {
                reason: "already undone".into(),
                chain: Vec::new(),
            });
        }
        let id = ev
            .target
            .clone()
            .ok_or_else(|| Error::Invalid("event has no target".into()))?;
        Ok((id, ev))
    }

    fn apply_undo(&self, st: &mut State, ev: &Event, id: &str, path: &str) -> Result<String> {
        let actor = Actor::person();
        let current = fs::read_to_string(self.root().join(path)).ok();
        let cur_hash = current.as_deref().map(util::hash_str);
        let reason = format!("undo of {}", ev.id);
        match &ev.before {
            Some(text) => {
                let before = current.zip(cur_hash);
                self.commit(
                    st,
                    path,
                    id,
                    before,
                    text,
                    EventKind::Undid,
                    "undo",
                    Some(&reason),
                    None,
                    Some(&ev.id),
                    &actor,
                )
            }
            None => {
                // Desfazer criação: o texto continua no diário.
                let full = paths::safe_join(self.root(), path)?;
                fs::remove_file(&full)?;
                let mut e = self.event(EventKind::Undid, Some(&actor));
                e.target = Some(id.to_string());
                e.path = Some(path.to_string());
                e.before = current;
                e.before_hash = cur_hash;
                e.rule = Some("undo".to_string());
                e.reason = Some(reason);
                e.undoes = Some(ev.id.clone());
                self.append(&mut e)?;
                self.drop_memory(st, id);
                st.view.last_hash.remove(id);
                st.recompute_superseded(util::now_micros());
                st.dirty = true;
                self.bump_generation(st);
                Ok(e.id)
            }
        }
    }

    /// Desfaz todo o estrato de uma sessão, em ordem reversa, tudo ou nada.
    pub fn undo_session(&self, session: &str) -> Result<UndoReport> {
        let _lock = self.write_lock()?;
        let mut st = self.lock_state();
        self.refresh(&mut st, None, true)?;
        st.view.catch_up(self.root());
        let muts: Vec<Mutation> = st
            .view
            .mutations
            .iter()
            .filter(|m| {
                m.session == session
                    && m.ev != EventKind::Undid
                    && !st.view.undone.contains(&m.event)
            })
            .cloned()
            .collect();
        if muts.is_empty() {
            return Err(Error::NotFound(format!(
                "no changes to undo in session {session}"
            )));
        }
        // Checa antes de tocar em qualquer arquivo.
        let mut last_by_target: BTreeMap<String, Mutation> = BTreeMap::new();
        for m in &muts {
            last_by_target.insert(m.target.clone(), m.clone());
        }
        let mut events = Vec::with_capacity(muts.len());
        for m in muts.iter().rev() {
            let e = journal::find_event(self.root(), &m.event)
                .ok_or_else(|| Error::NotFound(m.event.clone()))?;
            events.push(e);
        }
        let files: Vec<String> = events.iter().map(|e| file_of(&st, e)).collect();
        for (target, m) in &last_by_target {
            let e = events
                .iter()
                .find(|e| e.id == m.event)
                .ok_or_else(|| Error::NotFound(m.event.clone()))?;
            let path = file_of(&st, e);
            let cur = fs::read(self.root().join(&path))
                .ok()
                .map(|b| util::hash_bytes(&b));
            if cur != m.after_hash {
                let chain: Vec<String> = Self::later_mutations(&st, target, &m.event)
                    .into_iter()
                    .rev()
                    .map(|x| x.event)
                    .collect();
                return Err(Error::UndoRefused {
                    reason: format!(
                        "memory {target} changed after this session; nothing was undone"
                    ),
                    chain,
                });
            }
        }
        // Plano idempotente: um `kill -9` no meio é retomado na próxima trava.
        let mut ops = Vec::new();
        let mut sim: BTreeMap<String, Option<String>> = BTreeMap::new();
        for (e, path) in events.iter().zip(&files) {
            let path = path.clone();
            let cur_hash = sim
                .get(&path)
                .cloned()
                .unwrap_or_else(|| e.after_hash.clone());
            match (&e.before, cur_hash) {
                (Some(text), Some(expected)) => {
                    ops.push(PlanOp::Write {
                        path: path.clone(),
                        expected,
                        text: text.clone(),
                    });
                    sim.insert(path, Some(util::hash_str(text)));
                }
                (None, Some(expected)) => {
                    ops.push(PlanOp::Remove {
                        path: path.clone(),
                        expected,
                    });
                    sim.insert(path, None);
                }
                _ => {}
            }
        }
        let plan = Plan {
            id: util::new_ulid(),
            ops,
        };
        self.run_plan(&plan)?;
        let actor = Actor::person();
        let mut undid = Vec::new();
        let mut undone = Vec::new();
        for (ev, path) in events.iter().zip(&files) {
            let mut e = self.event(EventKind::Undid, Some(&actor));
            e.target = ev.target.clone();
            e.path = Some(path.clone());
            e.before = ev.after.clone();
            e.before_hash = ev.after_hash.clone();
            e.after = ev.before.clone();
            e.after_hash = ev.before_hash.clone();
            e.rule = Some("undo_session".into());
            e.reason = Some(format!("undo of session {session}"));
            e.undoes = Some(ev.id.clone());
            self.append(&mut e)?;
            undid.push(e.id);
            undone.push(ev.id.clone());
        }
        // Estado do disco manda.
        for (ev, path) in events.iter().zip(&files) {
            let id = ev.target.clone().unwrap_or_default();
            match fs::read_to_string(self.root().join(path)) {
                Ok(text) => self.ingest_memory_now(&mut st, path, &text)?,
                Err(_) => self.drop_memory(&mut st, &id),
            }
        }
        st.recompute_superseded(util::now_micros());
        st.dirty = true;
        self.bump_generation(&mut st);
        st.view.catch_up(self.root());
        Ok(UndoReport {
            undone,
            undid_events: undid,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flag {
    Archive,
    Restore,
    Promote,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UndoReport {
    pub undone: Vec<String>,
    pub undid_events: Vec<String>,
}
