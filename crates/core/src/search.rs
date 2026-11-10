//! Busca e leitura. A exclusão de substituídas e quarentenadas mora aqui, em código.

use crate::analyzer::{fold, is_stopword, QueryWord};
use crate::brain::{Brain, MemRec, State};
use crate::error::{Error, Result};
use crate::inspect;
use crate::journal::EventKind;
use crate::model::*;
use crate::paths;
use crate::util;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

const RRF_K: f32 = 60.0;
const WEAK_RATIO: f32 = 0.5;
const SNIPPET_CHARS: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Audience {
    /// Agente: quarentena nunca aparece.
    #[default]
    Agent,
    /// Pessoa, no app: vê tudo, marcado.
    Person,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    /// De 1 a 3 reformulações.
    pub queries: Vec<String>,
    pub limit: usize,
    pub offset: usize,
    /// `note` ou tipo de memória. Vazio: tudo.
    pub types: Vec<String>,
    pub tags: Vec<String>,
    pub project: Option<String>,
    pub include_old: bool,
    pub audience: Audience,
    /// Grava evento `searched`.
    pub record: bool,
    /// Tempo máximo esperando a indexação. `None` espera até o fim.
    pub wait_ms: Option<u64>,
}

impl SearchQuery {
    pub fn new(q: &str) -> Self {
        SearchQuery {
            queries: vec![q.to_string()],
            limit: 5,
            ..Default::default()
        }
    }
    pub fn person(q: &str) -> Self {
        SearchQuery {
            audience: Audience::Person,
            ..SearchQuery::new(q)
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Coverage {
    pub memories: usize,
    pub notes: usize,
    pub hidden: BTreeMap<String, usize>,
    pub unread: usize,
    pub indexing: usize,
    pub scanned_ms_ago: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchStatus {
    Ok,
    Weak,
    NoMatch,
    Incomplete,
}

impl SearchStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SearchStatus::Ok => "ok",
            SearchStatus::Weak => "weak",
            SearchStatus::NoMatch => "no_match",
            SearchStatus::Incomplete => "incomplete",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub id: String,
    pub kind: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub mem_type: Option<MemoryType>,
    pub title: String,
    pub path: String,
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<MemoryState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<Authority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    pub date: String,
    pub score: f32,
    pub match_ratio: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResponse {
    pub status: SearchStatus,
    pub hits: Vec<Hit>,
    pub total: usize,
    pub next_offset: Option<usize>,
    pub coverage: Coverage,
    pub reason: Option<String>,
    pub corrected: Option<String>,
    pub took_ms: u64,
}

fn activation(st: &State, rec: &MemRec) -> f32 {
    let Some(u) = st.view.uses.get(&rec.memory.id) else {
        return 1.0;
    };
    let useful = 1.0 + 0.15 * (1.0 + u.helped as f32).ln();
    let hurt = 1.0 / (1.0 + 0.25 * u.hurt as f32);
    let decays = matches!(
        rec.memory.mem_type,
        MemoryType::Episode | MemoryType::Concept
    );
    let idle = if decays && u.helped > 0 {
        let now = st.view.sessions.len() as u32;
        let gap = now.saturating_sub(u.last_useful_session).min(40);
        0.99f32.powi(gap as i32).max(0.7)
    } else {
        1.0
    };
    useful * hurt * idle
}

fn state_weight(s: MemoryState) -> f32 {
    match s {
        MemoryState::Active => 1.0,
        MemoryState::Stale => 0.5,
        _ => 0.4,
    }
}

fn authority_weight(a: Authority) -> f32 {
    match a {
        Authority::Person => 1.3,
        Authority::Agent => 1.0,
        Authority::External => 0.8,
    }
}

fn origin_label(m: &Memory) -> String {
    let mut s = m.origin.kind.as_str().to_string();
    if let Some(c) = &m.origin.client {
        s.push_str(" via ");
        s.push_str(c);
    }
    if let Some(n) = &m.origin.note {
        s.push_str(", ");
        s.push_str(n);
    }
    s
}

fn word_matches(line_folded: &str, words: &[QueryWord]) -> usize {
    let mut n = 0;
    for w in words {
        let t: &str = &w.text;
        let probe = if t.len() > 5 {
            &t[..t.char_indices().nth(5).map_or(t.len(), |x| x.0)]
        } else {
            t
        };
        if line_folded.contains(probe) {
            n += 1;
        }
    }
    n
}

/// Trecho que mais casa: linha, ou frase se a linha for longa. Sem invisível e sem segredo.
pub(crate) fn make_snippet(body: &str, fallback: &str, words: &[QueryWord]) -> String {
    let mut best: Option<(usize, &str)> = None;
    let mut in_fence = false;
    for line in body.lines() {
        let t = line.trim();
        if t.starts_with("```") {
            in_fence = !in_fence;
        }
        if t.is_empty() || t == "---" {
            continue;
        }
        let parts: Vec<&str> = if t.chars().count() > SNIPPET_CHARS {
            t.split_inclusive(". ").collect()
        } else {
            vec![t]
        };
        for p in parts {
            let n = word_matches(&fold(p), words);
            if n > 0 && best.is_none_or(|b| n > b.0) {
                best = Some((n, p));
            }
        }
    }
    let raw = best.map_or_else(
        || {
            if fallback.is_empty() {
                body.lines()
                    .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
                    .unwrap_or("")
            } else {
                fallback
            }
        },
        |b| b.1,
    );
    let clean = inspect::redact(&inspect::strip_invisible(raw));
    let t = clean.trim();
    let cut: String = t.chars().take(SNIPPET_CHARS).collect();
    if cut.len() < t.len() {
        format!("{cut}…")
    } else {
        cut
    }
}

impl Brain {
    /// Espera a indexação até o prazo, soltando o estado entre fatias.
    pub(crate) fn wait_ready(&self, wait_ms: Option<u64>) -> Result<()> {
        let deadline = wait_ms.map(|m| Instant::now() + Duration::from_millis(m));
        loop {
            let slice = wait_ms.map_or(40, |m| m.min(40));
            let pending = self.reconcile_slice(Duration::from_millis(slice))?;
            if pending == 0 {
                return Ok(());
            }
            if deadline.is_some_and(|d| Instant::now() >= d) {
                return Ok(());
            }
        }
    }

    pub fn search(&self, q: &SearchQuery) -> Result<SearchResponse> {
        let started = Instant::now();
        if q.queries.is_empty() {
            return Err(Error::Invalid("at least one query".into()));
        }
        if q.queries.len() > 3 {
            return Err(Error::Invalid("at most 3 queries".into()));
        }
        self.wait_ready(q.wait_ms)?;
        let mut st = self.lock_state();
        self.refresh(
            &mut st,
            q.wait_ms.map(|m| Duration::from_millis(m.min(40))),
            false,
        )?;
        let mut resp = self.search_locked(&mut st, q);
        resp.took_ms = started.elapsed().as_millis() as u64;
        let results: Vec<String> = resp.hits.iter().map(|h| h.id.clone()).collect();
        drop(st);
        if q.record {
            let mut e = self.event(EventKind::Searched, None);
            e.queries = Some(q.queries.iter().map(|s| inspect::redact(s)).collect());
            e.results = Some(results);
            e.status = Some(resp.status.as_str().to_string());
            let _ = self.append(&mut e);
        }
        Ok(resp)
    }

    fn visible(st: &State, rec: &MemRec, state: MemoryState, q: &SearchQuery) -> bool {
        let _ = (st, rec);
        match state {
            MemoryState::Active | MemoryState::Stale => true,
            MemoryState::Quarantined => q.audience == Audience::Person,
            _ => q.include_old,
        }
    }

    fn passes_filters(st: &State, key: &str, q: &SearchQuery) -> bool {
        let (ty, tags): (&str, &[String]) = if let Some(path) = key.strip_prefix("note:") {
            (
                "note",
                st.notes.get(path).map_or(&[], |n| n.tags.as_slice()),
            )
        } else {
            match st.mems.get(key) {
                Some(r) => (r.memory.mem_type.as_str(), r.memory.tags.as_slice()),
                None => return false,
            }
        };
        if !q.types.is_empty() && !q.types.iter().any(|t| t == ty) {
            return false;
        }
        let matches_tag = |want: &str| {
            tags.iter()
                .any(|t| t == want || t.ends_with(&format!("/{want}")))
        };
        if !q.tags.iter().all(|t| matches_tag(t)) {
            return false;
        }
        if let Some(p) = &q.project {
            if ty != "note" && !matches_tag(&format!("project/{p}")) {
                return false;
            }
        }
        true
    }

    fn search_locked(&self, st: &mut State, q: &SearchQuery) -> SearchResponse {
        let now = util::now_micros();
        st.ensure_resolver();
        let mut hidden_ids: HashMap<String, MemoryState> = HashMap::new();
        let mut ranked_lists: Vec<Vec<(String, f32, f32)>> = Vec::new();
        let mut words_all: Vec<QueryWord> = Vec::new();
        let mut corrected = None;

        for text in &q.queries {
            let words = st.analyzer.query_words(text);
            let list = self.rank(st, &words, q, now, &mut hidden_ids);
            ranked_lists.push(list);
            words_all.extend(words);
        }

        if ranked_lists.iter().all(Vec::is_empty) {
            // Correção de typo só quando nada voltou.
            let mut fixed_any = false;
            let mut new_words = Vec::new();
            for w in words_all
                .iter()
                .filter(|w| !w.bonus && !is_stopword(&w.text))
            {
                match st.index.fuzzy_term(&w.text) {
                    Some(t) if !st.index.is_empty() => {
                        fixed_any = true;
                        new_words.push(t);
                    }
                    _ => new_words.push(w.text.clone()),
                }
            }
            if fixed_any {
                let fixed = new_words.join(" ");
                let words = st.analyzer.query_words(&fixed);
                let list = self.rank(st, &words, q, now, &mut hidden_ids);
                if !list.is_empty() {
                    ranked_lists = vec![list];
                    words_all = words;
                    corrected = Some(fixed);
                }
            }
        }

        // Fusão por RRF. Uma consulta só preserva a ordem do BM25.
        let mut fused: HashMap<String, (f32, f32)> = HashMap::new();
        for list in &ranked_lists {
            for (rank, (key, score, ratio)) in list.iter().enumerate() {
                let e = fused.entry(key.clone()).or_insert((0.0, 0.0));
                e.0 += if ranked_lists.len() == 1 {
                    *score
                } else {
                    1.0 / (RRF_K + rank as f32 + 1.0)
                };
                e.1 = e.1.max(*ratio);
            }
        }
        let mut order: Vec<(String, f32, f32)> =
            fused.into_iter().map(|(k, (s, r))| (k, s, r)).collect();
        order.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut coverage = self.coverage(st);
        for s in hidden_ids.values() {
            *coverage.hidden.entry(s.as_str().to_string()).or_insert(0) += 1;
        }

        let total = order.len();
        let limit = if q.limit == 0 { 5 } else { q.limit.min(50) };
        let mut hits = Vec::new();
        let mut consumed = q.offset.min(total);
        for (key, score, ratio) in order.iter().skip(q.offset) {
            if hits.len() >= limit {
                break;
            }
            consumed += 1;
            if let Some(h) = self.make_hit(
                st,
                key,
                *score,
                *ratio,
                &words_all,
                now,
                corrected.is_some(),
            ) {
                hits.push(h);
            } else {
                // Arquivo sumiu entre a varredura e a leitura: nunca mostra o que o brain não tem mais.
                coverage.unread += 1;
            }
        }
        let next_offset = (consumed < total).then_some(consumed);

        let best_ratio = order.iter().map(|x| x.2).fold(0.0, f32::max);
        let weak = corrected.is_some() || best_ratio < WEAK_RATIO;
        let (status, reason) = if coverage.indexing > 0 {
            (
                SearchStatus::Incomplete,
                Some(format!("{} files still being indexed", coverage.indexing)),
            )
        } else if hits.is_empty() {
            if coverage.unread > 0 {
                (
                    SearchStatus::Incomplete,
                    Some(format!("{} files could not be read", coverage.unread)),
                )
            } else {
                (SearchStatus::NoMatch, None)
            }
        } else if weak {
            (
                SearchStatus::Weak,
                Some("results share only part of the query".to_string()),
            )
        } else {
            (SearchStatus::Ok, None)
        };
        SearchResponse {
            status,
            hits,
            total,
            next_offset,
            coverage,
            reason,
            corrected,
            took_ms: 0,
        }
    }

    /// (chave, pontuação, cobertura) em ordem, com pesos de memória e bônus de grafo.
    fn rank(
        &self,
        st: &State,
        words: &[QueryWord],
        q: &SearchQuery,
        now: i64,
        hidden: &mut HashMap<String, MemoryState>,
    ) -> Vec<(String, f32, f32)> {
        let mut out = Vec::new();
        for s in st.index.search(words) {
            if !Self::passes_filters(st, &s.key, q) {
                continue;
            }
            let score = if let Some(rec) = st.mems.get(&s.key) {
                // Na carência de 300 ms o arquivo sumiu: não mostra, e a cobertura declara.
                if st.missing.contains_key(&rec.path) {
                    continue;
                }
                let state = st.state_of(rec, now);
                if !Self::visible(st, rec, state, q) {
                    hidden.insert(s.key.clone(), state);
                    continue;
                }
                s.score
                    * state_weight(state)
                    * authority_weight(rec.effective_authority())
                    * activation(st, rec)
            } else if s.key.starts_with("note:") {
                s.score
            } else {
                continue;
            };
            out.push((s.key, score, s.ratio));
        }
        out.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        // Bônus para vizinhos de wikilink a um salto do melhor resultado.
        if let Some(best) = out.first().map(|x| x.0.clone()) {
            if let Some(path) = best.strip_prefix("note:") {
                if let (Some(n), Some(resolver)) = (st.notes.get(path), &st.resolver) {
                    let neigh: HashSet<String> = n
                        .links
                        .iter()
                        .filter_map(|l| resolver.resolve(l))
                        .map(|p| format!("note:{p}"))
                        .collect();
                    if !neigh.is_empty() {
                        for x in out.iter_mut().skip(1) {
                            if neigh.contains(&x.0) {
                                x.1 *= 1.15;
                            }
                        }
                        out.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                    }
                }
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn make_hit(
        &self,
        st: &State,
        key: &str,
        score: f32,
        ratio: f32,
        words: &[QueryWord],
        now: i64,
        corrected: bool,
    ) -> Option<Hit> {
        let ratio = if corrected { ratio.min(0.49) } else { ratio };
        if let Some(rec) = st.mems.get(key) {
            let m = &rec.memory;
            let state = st.state_of(rec, now);
            let warning = match state {
                MemoryState::Stale => {
                    Some("source changed since this was written; check it".to_string())
                }
                MemoryState::Superseded => st
                    .superseded_by
                    .get(&m.id)
                    .map(|s| format!("superseded by {s}")),
                MemoryState::Expired => Some("expired".to_string()),
                MemoryState::Archived => Some("archived".to_string()),
                MemoryState::Quarantined => {
                    rec.quarantine.as_ref().map(|r| format!("quarantined: {r}"))
                }
                MemoryState::Active => None,
            };
            let snippet = if state == MemoryState::Quarantined {
                inspect::reveal(&m.summary)
            } else {
                make_snippet(&m.body, &m.summary, words)
            };
            return Some(Hit {
                id: m.id.clone(),
                kind: "memory".into(),
                mem_type: Some(m.mem_type),
                title: inspect::strip_invisible(&m.title),
                path: rec.path.clone(),
                snippet,
                project: crate::tags::project_of(&m.tags).map(str::to_string),
                state: Some(state),
                authority: Some(rec.effective_authority()),
                origin: Some(origin_label(m)),
                date: m.updated.chars().take(10).collect(),
                score,
                match_ratio: ratio,
                warning,
                flags: rec.flags(),
            });
        }
        let path = key.strip_prefix("note:")?;
        let n = st.notes.get(path)?;
        let full = self.inner.root.join(path);
        let bytes = std::fs::read(&full).ok()?;
        let cap = bytes.len().min(256 * 1024);
        let text = String::from_utf8_lossy(&bytes[..cap]);
        let (_, body) = crate::frontmatter::split(&text);
        let mut flags = crate::brain::note_flags(path, &n.title);
        if inspect::has_invisible(body) {
            flags.push("invisible_text".to_string());
        }
        if !flags.iter().any(|f| f == "instruction_like")
            && inspect::instruction_like(body).is_some()
        {
            flags.push("instruction_like".to_string());
        }
        let clean_path = inspect::strip_invisible(path);
        Some(Hit {
            id: format!("note:{clean_path}"),
            kind: "note".into(),
            mem_type: None,
            title: inspect::strip_invisible(&n.title),
            path: clean_path,
            snippet: make_snippet(body, "", words),
            project: None,
            state: None,
            authority: None,
            origin: None,
            date: util::format_ts_seconds(n.sig.mtime_ns / 1000)
                .chars()
                .take(10)
                .collect(),
            score,
            match_ratio: ratio,
            warning: None,
            flags,
        })
    }

    pub(crate) fn coverage(&self, st: &State) -> Coverage {
        Coverage {
            memories: st.mems.len(),
            notes: st.notes.len(),
            hidden: BTreeMap::new(),
            unread: st.unread.len() + st.invalid.len() + st.missing.len(),
            indexing: st.pending.len(),
            scanned_ms_ago: st.last_scan.map_or(0, |t| t.elapsed().as_millis() as u64),
        }
    }

    /// Lista para o app: todas as memórias, com estado derivado.
    pub fn memories(&self) -> Result<Vec<MemoryMeta>> {
        let mut st = self.lock_state();
        self.refresh(&mut st, None, false)?;
        while !st.pending.is_empty() {
            self.refresh(&mut st, None, true)?;
        }
        let now = util::now_micros();
        let mut v: Vec<MemoryMeta> = st.mems.values().map(|r| st.meta_of(r, now)).collect();
        v.sort_by(|a, b| b.updated.cmp(&a.updated).then_with(|| a.id.cmp(&b.id)));
        Ok(v)
    }

    /// Detalhe para a pessoa. Agente usa `read_item`, que aplica as exclusões.
    pub fn memory(&self, id: &str) -> Result<Option<MemoryDetail>> {
        let mut st = self.lock_state();
        self.refresh(&mut st, None, false)?;
        let now = util::now_micros();
        let Some(rec) = st.mems.get(id) else {
            return Ok(None);
        };
        let full = self.inner.root.join(&rec.path);
        let text = std::fs::read_to_string(&full).unwrap_or_default();
        // Só os campos: o frontmatter tem `authority: person` e `---` em toda memória.
        let mut found = inspect::Findings::default();
        let mem = &rec.memory;
        for field in [&mem.title, &mem.summary, &mem.body] {
            found.merge(inspect::inspect(field));
        }
        for field in &mem.aliases {
            found.merge(inspect::inspect(field));
        }
        for field in &mem.tags {
            found.merge(inspect::inspect_tag(field));
        }
        let mut flags = found.labels();
        flags.extend(rec.flags());
        for code in rec.suspect.iter().flat_map(|s| s.split(", ")) {
            if !flags.iter().any(|f| f == code) {
                flags.push(code.to_string());
            }
        }
        Ok(Some(MemoryDetail {
            meta: st.meta_of(rec, now),
            memory: rec.memory.clone(),
            revealed: inspect::reveal(&text),
            text,
            flags,
        }))
    }

    fn resolve_target(&self, st: &State, target: &str) -> Option<String> {
        let t = util::plain_id(target);
        if st.mems.contains_key(t) {
            return Some(t.to_string());
        }
        if let Some(p) = t.strip_prefix("note:") {
            if st.notes.contains_key(p) {
                return Some(format!("note:{p}"));
            }
            let with = format!("notes/{p}");
            if st.notes.contains_key(&with) {
                return Some(format!("note:{with}"));
            }
            return self.note_by_name(st, p);
        }
        if let Some(id) = st.path_ids.get(t) {
            return Some(id.clone());
        }
        if st.notes.contains_key(t) {
            return Some(format!("note:{t}"));
        }
        // Caminho parcial, como o catálogo mostra.
        let name = t.rsplit('/').next().unwrap_or(t);
        let hits: Vec<&String> = st
            .path_ids
            .keys()
            .filter(|p| p.rsplit('/').next() == Some(name))
            .collect();
        if hits.len() == 1 {
            return st.path_ids.get(hits[0]).cloned();
        }
        let notes: Vec<&String> = st.notes.keys().filter(|p| p.ends_with(t)).collect();
        if notes.len() == 1 {
            return Some(format!("note:{}", notes[0]));
        }
        self.note_by_name(st, t)
    }

    /// Nota pelo nome do arquivo, sem pasta nem extensão, como a IA a cita. Só se for única.
    fn note_by_name(&self, st: &State, target: &str) -> Option<String> {
        let last = target.rsplit('/').next().unwrap_or(target);
        let name = fold(last.strip_suffix(".md").unwrap_or(last));
        let hits: Vec<&String> = st
            .notes
            .keys()
            .filter(|p| {
                let file = p.rsplit('/').next().unwrap_or(p);
                fold(file.strip_suffix(".md").unwrap_or(file)) == name
            })
            .collect();
        (hits.len() == 1).then(|| format!("note:{}", hits[0]))
    }

    /// A busca entrega caminho sem invisíveis; aqui ele volta ao nome real do arquivo.
    fn resolve_clean(&self, st: &State, target: &str) -> Option<String> {
        let t = target.strip_prefix("note:").unwrap_or(target).trim();
        let hits: Vec<&String> = st
            .notes
            .keys()
            .filter(|p| inspect::has_invisible(p) && inspect::strip_invisible(p) == t)
            .collect();
        (hits.len() == 1).then(|| format!("note:{}", hits[0]))
    }

    /// Leitura para o agente: paginada, com origem, e sem nada oculto.
    pub fn read_item(&self, req: &ReadRequest) -> Result<ReadResponse> {
        if let Some(rel) = req.target.strip_prefix("note:") {
            paths::check_rel(rel)?;
        }
        self.wait_ready(req.wait_ms)?;
        let mut st = self.lock_state();
        self.refresh(&mut st, None, false)?;
        let now = util::now_micros();
        let coverage = self.coverage(&st);
        let target = self
            .resolve_target(&st, &req.target)
            .or_else(|| self.resolve_clean(&st, &req.target));
        let Some(target) = target else {
            return Ok(ReadResponse::missing(&req.target, coverage));
        };
        let max = if req.max_chars == 0 {
            6000
        } else {
            req.max_chars.min(self.inner.config.max_read_chars)
        };
        let page = |text: &str| -> (String, usize, Option<usize>) {
            let chars: Vec<char> = text.chars().collect();
            let total = chars.len();
            let start = req.offset.min(total);
            let end = (start + max).min(total);
            let chunk: String = chars[start..end].iter().collect();
            (chunk, total, (end < total).then_some(end))
        };
        if let Some(path) = target.strip_prefix("note:") {
            let path = path.to_string();
            let full = paths::safe_join(&self.inner.root, &path)?;
            // Sumiu e a varredura ainda não viu: responde como não achada.
            let Ok(bytes) = std::fs::read(&full) else {
                return Ok(ReadResponse::missing(&req.target, coverage));
            };
            let hash = util::hash_bytes(&bytes);
            let raw = String::from_utf8_lossy(&bytes).into_owned();
            let parsed_title = st.notes.get(&path).map_or("", |n| n.title.as_str());
            let mut flags = crate::brain::note_flags(&path, parsed_title);
            if inspect::has_invisible(&raw) {
                flags.push("invisible_text".to_string());
            }
            if !flags.iter().any(|f| f == "instruction_like")
                && inspect::instruction_like(&raw).is_some()
            {
                flags.push("instruction_like".to_string());
            }
            if inspect::find_secret(&raw).is_some() {
                flags.push("secret_redacted".to_string());
            }
            let clean = inspect::redact(&inspect::strip_invisible(&raw));
            let (text, total, next_offset) = page(&clean);
            let title = st
                .notes
                .get(&path)
                .map_or_else(|| path.clone(), |n| inspect::strip_invisible(&n.title));
            let session = self.session().to_string();
            let mut covered = false;
            if req.distill {
                let key = (session.clone(), path.clone());
                let contiguous = req.offset == 0
                    || st
                        .reading
                        .get(&key)
                        .is_some_and(|(h, off)| h == &hash && *off == req.offset);
                match (contiguous, next_offset) {
                    (true, None) => {
                        covered = true;
                        st.reading.remove(&key);
                    }
                    (true, Some(off)) => {
                        st.reading.insert(key, (hash.clone(), off));
                    }
                    _ => {}
                }
            }
            drop(st);
            let mut e = self.event(EventKind::Read, None);
            e.target = Some(target.clone());
            e.path = Some(path.clone());
            e.after_hash = Some(hash.clone());
            let _ = self.append(&mut e);
            if covered {
                let mut e = self.event(EventKind::Covered, None);
                e.path = Some(path.clone());
                e.after_hash = Some(hash.clone());
                let _ = self.append(&mut e);
            }
            let clean_path = inspect::strip_invisible(&path);
            return Ok(ReadResponse {
                status: "ok".into(),
                id: format!("note:{clean_path}"),
                kind: "note".into(),
                title,
                path: clean_path,
                project: None,
                text,
                total_chars: total,
                next_offset,
                hash: Some(hash),
                state: None,
                authority: None,
                origin: None,
                date: None,
                superseded_by: None,
                neighbors: Vec::new(),
                flags,
                covered,
                coverage,
            });
        }
        let Some(rec) = st.mems.get(&target) else {
            return Ok(ReadResponse::missing(&req.target, coverage));
        };
        if st.missing.contains_key(&rec.path) || !self.inner.root.join(&rec.path).exists() {
            return Ok(ReadResponse::missing(&req.target, coverage));
        }
        let state = st.state_of(rec, now);
        let m = &rec.memory;
        let visible = match state {
            MemoryState::Active | MemoryState::Stale => true,
            MemoryState::Quarantined => false,
            _ => req.include_old,
        };
        let superseded_by = st.superseded_by.get(&m.id).cloned();
        if !visible {
            let mut r = ReadResponse::missing(&req.target, coverage);
            r.status = "hidden".into();
            r.id = m.id.clone();
            r.kind = "memory".into();
            r.state = Some(state);
            r.superseded_by = superseded_by;
            return Ok(r);
        }
        let header = if m.summary.is_empty() {
            String::new()
        } else {
            format!("{}\n\n", m.summary)
        };
        let body = format!("{header}{}", m.body);
        let (text, total, next_offset) = page(&inspect::redact(&inspect::strip_invisible(&body)));
        let neighbors = crate::apply::neighbors_of(&st, m, 5, now);
        let resp = ReadResponse {
            status: "ok".into(),
            id: m.id.clone(),
            kind: "memory".into(),
            title: inspect::strip_invisible(&m.title),
            path: rec.path.clone(),
            project: crate::tags::project_of(&m.tags).map(str::to_string),
            text,
            total_chars: total,
            next_offset,
            hash: Some(rec.hash.clone()),
            state: Some(state),
            authority: Some(rec.effective_authority()),
            origin: Some(origin_label(m)),
            date: Some(m.updated.chars().take(10).collect()),
            superseded_by,
            neighbors,
            flags: rec.flags(),
            covered: false,
            coverage,
        };
        drop(st);
        let mut e = self.event(EventKind::Read, None);
        e.target = Some(resp.id.clone());
        let _ = self.append(&mut e);
        Ok(resp)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReadRequest {
    pub target: String,
    pub offset: usize,
    pub max_chars: usize,
    pub include_old: bool,
    /// Nota lida por inteiro conta como coberta, por hash.
    pub distill: bool,
    pub wait_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadResponse {
    pub status: String,
    pub id: String,
    pub kind: String,
    pub title: String,
    pub path: String,
    pub text: String,
    pub total_chars: usize,
    pub next_offset: Option<usize>,
    pub hash: Option<String>,
    pub project: Option<String>,
    pub state: Option<MemoryState>,
    pub authority: Option<Authority>,
    pub origin: Option<String>,
    pub date: Option<String>,
    pub superseded_by: Option<String>,
    pub neighbors: Vec<Neighbor>,
    pub flags: Vec<String>,
    pub covered: bool,
    pub coverage: Coverage,
}

impl ReadResponse {
    fn missing(target: &str, coverage: Coverage) -> Self {
        ReadResponse {
            status: "not_found".into(),
            id: target.chars().take(200).collect(),
            kind: String::new(),
            project: None,
            title: String::new(),
            path: String::new(),
            text: String::new(),
            total_chars: 0,
            next_offset: None,
            hash: None,
            state: None,
            authority: None,
            origin: None,
            date: None,
            superseded_by: None,
            neighbors: Vec::new(),
            flags: Vec::new(),
            covered: false,
            coverage,
        }
    }
}
