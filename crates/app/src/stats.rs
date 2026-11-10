//! Métricas tiradas do diário. Roda na thread de dados; a tela só lê o resultado pronto.

use crate::model::{Graph, Memory, MemState, Note};
use omnimind_core::journal::{Event, EventKind};
use rustc_hash::{FxHashMap, FxHashSet};

pub const PERIODS: [u32; 3] = [7, 30, 90];
const HISTORY: i64 = 180;
const HEAT_DAYS: i64 = 91;
const MONTHS: [&str; 12] = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];
const WEEKDAYS: [&str; 7] = ["domingo", "segunda", "terça", "quarta", "quinta", "sexta", "sábado"];

#[derive(Clone, Debug, Default)]
pub struct Ranked {
    pub id: String,
    pub title: String,
    pub is_note: bool,
    pub count: u32,
    pub by_ai: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ThemeStat {
    pub name: String,
    pub slot: u8,
    pub count: u32,
    pub before: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Gap {
    pub query: String,
    pub count: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Tool {
    pub name: String,
    pub sessions: u32,
    pub reads: u32,
    pub writes: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Bar {
    pub label: String,
    pub ai: u32,
    pub you: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Totals {
    pub reads: u32,
    pub ai_reads: u32,
    pub memories: u32,
    pub fixes: u32,
    pub asks: u32,
    pub misses: u32,
}

impl Totals {
    /// Parte das buscas que achou algo, de 0 a 1. Sem buscas não há taxa.
    pub fn hit_rate(&self) -> Option<f32> {
        (self.asks > 0).then(|| 1.0 - self.misses as f32 / self.asks as f32)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Period {
    pub days: u32,
    pub now: Totals,
    pub before: Totals,
    pub weekly: bool,
    pub bars: Vec<Bar>,
    pub top_notes: Vec<Ranked>,
    pub top_memories: Vec<Ranked>,
    pub themes: Vec<ThemeStat>,
    pub gaps: Vec<Gap>,
    pub tools: Vec<Tool>,
    pub unread: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Metrics {
    pub periods: Vec<Period>,
    /// Escritas por dia, da semana mais antiga para hoje. `heat_pad` casas vazias antes.
    pub heat: Vec<u32>,
    pub heat_pad: usize,
    pub streak: u32,
    pub active: u32,
    pub best_day: &'static str,
    pub orphans: u32,
    pub stale: u32,
    /// Leituras em 30 dias, por nó.
    pub reads_30: FxHashMap<String, u32>,
}

/// Dias desde 1970-01-01 (Hinnant).
pub fn day_number(date: &str) -> Option<i64> {
    let (y, m, d) = (date.get(..4)?.parse::<i64>().ok()?, date.get(5..7)?.parse::<i64>().ok()?, date.get(8..10)?.parse::<i64>().ok()?);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

fn civil(n: i64) -> (i64, u32, u32) {
    let z = n + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (yoe + era * 400 + (m <= 2) as i64, m, d)
}

pub fn day_label(n: i64) -> String {
    let (_, m, d) = civil(n);
    format!("{d} {}", MONTHS[(m - 1) as usize])
}

fn weekday(n: i64) -> usize {
    (n + 4).rem_euclid(7) as usize
}

#[derive(Clone, Copy, Default)]
struct Day {
    ai: u32,
    you: u32,
    memories: u32,
    fixes: u32,
    asks: u32,
    misses: u32,
    wrote: u32,
}

#[derive(Default, Clone, Copy)]
struct Window {
    now: [u32; 3],
    before: [u32; 3],
    ai: [u32; 3],
}

impl Window {
    fn add(&mut self, age: i64, ai: bool) {
        for (i, p) in PERIODS.iter().enumerate() {
            let p = *p as i64;
            if age < p {
                self.now[i] += 1;
                self.ai[i] += ai as u32;
            } else if age < p * 2 {
                self.before[i] += 1;
            }
        }
    }
}

pub fn is_person(client: &str) -> bool {
    matches!(client, "" | "person" | "omnimind" | "varzea")
}

fn clean(id: &str) -> &str {
    id.strip_prefix("note:").unwrap_or(id)
}

pub struct Sources<'a> {
    pub events: &'a [Event],
    pub notes: &'a [Note],
    pub memories: &'a [Memory],
    pub graph: &'a Graph,
    /// Hoje, em número de dia na hora local.
    pub today: i64,
    /// Segundos a somar ao UTC do diário para chegar à hora local.
    pub offset: i64,
}

/// Dia e minuto do dia, na hora local, de um carimbo UTC do diário.
pub fn local_time(ts: &str, offset: i64) -> Option<(i64, i64)> {
    let day = ts.get(..10).and_then(day_number)?;
    let (h, m) = (ts.get(11..13).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0), ts.get(14..16).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0));
    let total = day * 1440 + h * 60 + m + offset / 60;
    Some((total.div_euclid(1440), total.rem_euclid(1440)))
}

pub fn client_label(c: &str) -> String {
    match c {
        "claude-code" => "Claude Code".into(),
        "copilot" => "GitHub Copilot".into(),
        "" => "A IA".into(),
        c if is_person(c) => "Você".into(),
        other => other.replace(['-', '_'], " "),
    }
}

pub fn compute(s: &Sources<'_>) -> Metrics {
    let today = s.today;
    let mut days = vec![Day::default(); HISTORY as usize];
    let mut reads: FxHashMap<&str, Window> = FxHashMap::default();
    let mut themes: FxHashMap<u8, Window> = FxHashMap::default();
    let mut gaps: FxHashMap<String, (String, Window)> = FxHashMap::default();
    let mut tool_reads: FxHashMap<&str, Window> = FxHashMap::default();
    let mut tool_writes: FxHashMap<&str, Window> = FxHashMap::default();
    let mut tool_sessions: FxHashMap<&str, [FxHashSet<&str>; 3]> = FxHashMap::default();
    let (slots, names) = crate::graphview::theme_slots(s.graph);
    let node_theme: FxHashMap<&str, u8> = s.graph.nodes.iter().map(|n| (n.id.as_str(), slots.get(n.region as usize).copied().unwrap_or(0))).collect();
    for e in s.events {
        let Some((n, _)) = local_time(&e.ts, s.offset) else { continue };
        let age = today - n;
        if !(0..HISTORY).contains(&age) {
            continue;
        }
        let d = &mut days[age as usize];
        let person = is_person(&e.client);
        let mut read: Option<&str> = None;
        match e.ev {
            EventKind::Read => read = e.target.as_deref().or(e.path.as_deref()).map(crate::data::node_id),
            EventKind::Tool if e.kind.as_deref() == Some("note_opened") => read = e.path.as_deref(),
            EventKind::Wrote if e.before.is_none() => {
                d.memories += 1;
                if !person {
                    tool_writes.entry(e.client.as_str()).or_default().add(age, true);
                }
            }
            EventKind::Merged | EventKind::Superseded | EventKind::Wrote if !person => tool_writes.entry(e.client.as_str()).or_default().add(age, true),
            EventKind::Signal if e.kind.as_deref() == Some("correction") => d.fixes += 1,
            EventKind::Searched if !person => {
                d.asks += 1;
                if e.results.as_ref().is_none_or(Vec::is_empty) {
                    d.misses += 1;
                    if let Some(q) = e.queries.as_ref().and_then(|q| q.first()) {
                        let shown = q.trim();
                        if !shown.is_empty() {
                            gaps.entry(ed::fold(shown)).or_insert_with(|| (shown.to_string(), Window::default())).1.add(age, true);
                        }
                    }
                }
            }
            EventKind::Note if e.kind.as_deref() == Some("saved") => d.wrote += 1,
            _ => {}
        }
        if let Some(r) = read {
            let r = clean(r);
            if person {
                d.you += 1;
            } else {
                d.ai += 1;
                tool_reads.entry(e.client.as_str()).or_default().add(age, true);
            }
            reads.entry(r).or_default().add(age, !person);
            if let Some(&g) = node_theme.get(r) {
                themes.entry(g).or_default().add(age, !person);
            }
        }
        if !person && !e.session.is_empty() {
            let sets = tool_sessions.entry(e.client.as_str()).or_default();
            for (i, p) in PERIODS.iter().enumerate() {
                if age < *p as i64 {
                    sets[i].insert(e.session.as_str());
                }
            }
        }
    }
    let totals = |from: i64, to: i64| {
        let mut t = Totals::default();
        for d in &days[from as usize..(to as usize).min(days.len())] {
            t.reads += d.ai + d.you;
            t.ai_reads += d.ai;
            t.memories += d.memories;
            t.fixes += d.fixes;
            t.asks += d.asks;
            t.misses += d.misses;
        }
        t
    };
    let note_title: FxHashMap<&str, &str> = s.notes.iter().map(|n| (n.path.as_str(), n.title.as_str())).collect();
    let mem_title: FxHashMap<&str, &str> = s.memories.iter().map(|m| (m.id.as_str(), m.title.as_str())).collect();
    let mut periods = Vec::with_capacity(3);
    for (i, &p) in PERIODS.iter().enumerate() {
        let len = p as i64;
        let weekly = p > 30;
        let mut bars = vec![];
        if weekly {
            let mut end = 0i64;
            while end < len {
                let start = (end + 7).min(len);
                let part = &days[end as usize..start as usize];
                bars.push(Bar { label: format!("Semana de {}", day_label(today - start + 1)), ai: part.iter().map(|d| d.ai).sum(), you: part.iter().map(|d| d.you).sum() });
                end = start;
            }
            bars.reverse();
        } else {
            for age in (0..len).rev() {
                bars.push(Bar { label: day_label(today - age), ai: days[age as usize].ai, you: days[age as usize].you });
            }
        }
        let mut notes = vec![];
        let mut memories = vec![];
        for (id, w) in &reads {
            if w.now[i] == 0 {
                continue;
            }
            if let Some(t) = note_title.get(id) {
                notes.push(Ranked { id: id.to_string(), title: t.to_string(), is_note: true, count: w.now[i], by_ai: w.ai[i] });
            } else if let Some(t) = mem_title.get(id) {
                memories.push(Ranked { id: id.to_string(), title: t.to_string(), is_note: false, count: w.now[i], by_ai: w.ai[i] });
            }
        }
        let rank = |v: &mut Vec<Ranked>| {
            v.sort_by(|a, b| b.count.cmp(&a.count).then(a.title.cmp(&b.title)));
            v.truncate(6);
        };
        rank(&mut notes);
        rank(&mut memories);
        let mut theme_rows: Vec<ThemeStat> = names.iter().enumerate().map(|(g, name)| {
            let w = themes.get(&(g as u8)).copied().unwrap_or_default();
            ThemeStat { name: name.clone(), slot: g as u8, count: w.now[i], before: w.before[i] }
        }).collect();
        theme_rows.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
        let mut gap_rows: Vec<Gap> = gaps.values().filter(|(_, w)| w.now[i] > 0).map(|(q, w)| Gap { query: q.clone(), count: w.now[i] }).collect();
        gap_rows.sort_by(|a, b| b.count.cmp(&a.count).then(a.query.cmp(&b.query)));
        gap_rows.truncate(5);
        let mut tools: Vec<Tool> = tool_sessions
            .iter()
            .map(|(c, sets)| Tool { name: client_label(c), sessions: sets[i].len() as u32, reads: tool_reads.get(c).map_or(0, |w| w.now[i]), writes: tool_writes.get(c).map_or(0, |w| w.now[i]) })
            .filter(|t| t.sessions > 0)
            .collect();
        tools.sort_by(|a, b| b.reads.cmp(&a.reads).then(a.name.cmp(&b.name)));
        let live = s.memories.iter().filter(|m| m.state == MemState::Active);
        let unread = live.filter(|m| reads.get(m.id.as_str()).is_none_or(|w| w.now[i] == 0)).count() as u32;
        periods.push(Period { days: p, now: totals(0, len), before: totals(len, len * 2), weekly, bars, top_notes: notes, top_memories: memories, themes: theme_rows, gaps: gap_rows, tools, unread });
    }
    let reads_30 = reads.iter().filter(|(_, w)| w.now[1] > 0).map(|(k, w)| (k.to_string(), w.now[1])).collect();
    let mut heat = Vec::with_capacity(HEAT_DAYS as usize);
    let mut by_weekday = [0u32; 7];
    for age in (0..HEAT_DAYS).rev() {
        let w = days[age as usize].wrote;
        heat.push(w);
        by_weekday[weekday(today - age)] += w;
    }
    let active = heat.iter().filter(|w| **w > 0).count() as u32;
    let mut streak = 0;
    let start = if days[0].wrote > 0 { 0 } else { 1 };
    for d in &days[start..] {
        if d.wrote == 0 {
            break;
        }
        streak += 1;
    }
    let best = by_weekday.iter().enumerate().max_by_key(|(_, v)| **v).map_or(0, |(i, _)| i);
    let mut linked: FxHashSet<u32> = FxHashSet::default();
    for &(a, b, _) in &s.graph.edges {
        linked.insert(a);
        linked.insert(b);
    }
    let orphans = s.graph.nodes.iter().enumerate().filter(|(i, n)| matches!(n.kind, crate::model::NodeKind::Note) && !linked.contains(&(*i as u32))).count() as u32;
    Metrics {
        periods,
        heat_pad: weekday(today - HEAT_DAYS + 1),
        heat,
        streak,
        active,
        best_day: if active > 0 { WEEKDAYS[best] } else { "" },
        orphans,
        stale: s.memories.iter().filter(|m| matches!(m.state, MemState::Stale | MemState::Expired)).count() as u32,
        reads_30,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Authority, GraphNode, MemKind, NodeKind};

    fn ev(kind: EventKind, day: &str, client: &str) -> Event {
        let mut e = Event::new(kind);
        e.ts = format!("{day}T12:00:00Z");
        e.client = client.into();
        e.session = format!("s-{client}");
        e
    }

    fn memory(id: &str) -> Memory {
        Memory { id: id.into(), hash: String::new(), kind: MemKind::Decision, title: format!("T {id}"), summary: String::new(), body: String::new(), state: MemState::Active, authority: Authority::Agent, origin: String::new(), client: String::new(), session: String::new(), quote: String::new(), note: None, created: String::new(), tags: vec![], uses: 0, held: None, project: None }
    }

    #[test]
    fn dates_round_trip() {
        let n = day_number("2026-11-01").unwrap();
        assert_eq!(civil(n), (2026, 11, 1));
        assert_eq!(day_label(n), "1 nov");
        assert_eq!(WEEKDAYS[weekday(n)], "domingo");
        assert_eq!(day_number("1970-01-01"), Some(0));
    }

    #[test]
    fn windows_and_rankings_follow_the_journal() {
        let graph = Graph { nodes: vec![GraphNode { id: "m1".into(), kind: NodeKind::Memory(MemKind::Decision), title: "T".into(), weight: 1.0, region: 0, project: u32::MAX, hub: false }], edges: vec![], regions: vec!["Rota".into()], projects: vec![] };
        let memories = vec![memory("m1"), memory("m2")];
        let mut events = vec![];
        let read = |day: &str, target: &str, client: &str| {
            let mut e = ev(EventKind::Read, day, client);
            e.target = Some(target.into());
            e
        };
        events.push(read("2026-11-01", "m1", "claude-code"));
        events.push(read("2026-10-30", "m1", "claude-code"));
        events.push(read("2026-10-20", "m1", "copilot"));
        events.push(read("2026-08-20", "m2", "claude-code"));
        let mut opened = ev(EventKind::Tool, "2026-11-01", "person");
        opened.kind = Some("note_opened".into());
        opened.path = Some("notes/a.md".into());
        events.push(opened);
        let mut miss = ev(EventKind::Searched, "2026-10-31", "claude-code");
        miss.queries = Some(vec!["frete grátis".into()]);
        miss.results = Some(vec![]);
        events.push(miss);
        let mut hit = ev(EventKind::Searched, "2026-10-31", "claude-code");
        hit.queries = Some(vec!["rota".into()]);
        hit.results = Some(vec!["m1".into()]);
        events.push(hit);
        let mut saved = ev(EventKind::Note, "2026-11-01", "person");
        saved.kind = Some("saved".into());
        events.push(saved);
        let mut saved2 = ev(EventKind::Note, "2026-10-31", "person");
        saved2.kind = Some("saved".into());
        events.push(saved2);
        let m = compute(&Sources { events: &events, notes: &[], memories: &memories, graph: &graph, today: day_number("2026-11-01").unwrap(), offset: 0 });
        let week = &m.periods[0];
        assert_eq!(week.now.ai_reads, 2);
        assert_eq!(week.now.reads, 3);
        assert_eq!(week.before.ai_reads, 1);
        assert_eq!(week.top_memories[0].count, 2);
        assert_eq!(week.gaps[0].query, "frete grátis");
        assert_eq!(week.now.hit_rate(), Some(0.5));
        assert_eq!(week.unread, 1, "m2 só foi lida em agosto");
        assert_eq!(m.periods[2].unread, 0);
        assert_eq!(week.tools[0].name, "Claude Code");
        assert_eq!(week.themes[0].count, 2);
        assert_eq!(week.bars.len(), 7);
        assert_eq!(week.bars[6].ai, 1);
        assert_eq!(m.periods[2].bars.len(), 13);
        assert_eq!(m.periods[2].bars.iter().map(|b| b.ai + b.you).sum::<u32>(), m.periods[2].now.reads);
        assert_eq!(m.streak, 2);
        assert_eq!(m.active, 2);
        assert_eq!(m.heat.len(), 91);
        assert_eq!(m.reads_30["m1"], 3);
    }

    #[test]
    fn a_note_read_by_the_ai_counts_under_the_same_path_the_screens_use() {
        // O diário grava a nota como `note:<caminho>`; as telas procuram pelo caminho.
        let mut by_ai = ev(EventKind::Read, "2026-11-01", "claude-code");
        by_ai.target = Some("note:notes/a.md".into());
        by_ai.path = Some("notes/a.md".into());
        let mut by_person = ev(EventKind::Tool, "2026-11-01", "person");
        by_person.kind = Some("note_opened".into());
        by_person.path = Some("notes/a.md".into());
        let m = compute(&Sources { events: &[by_ai, by_person], notes: &[], memories: &[], graph: &Graph::default(), today: day_number("2026-11-01").unwrap(), offset: 0 });
        assert_eq!(m.reads_30.get("notes/a.md"), Some(&2), "{:?}", m.reads_30);
        assert!(!m.reads_30.contains_key("note:notes/a.md"));
    }

    #[test]
    fn days_follow_local_time() {
        let brasilia = -3 * 3600;
        let (day, minute) = local_time("2026-11-01T01:30:00Z", brasilia).unwrap();
        assert_eq!(day, day_number("2026-10-31").unwrap());
        assert_eq!(minute, 22 * 60 + 30);
        assert_eq!(local_time("2026-10-31T23:30:00Z", 2 * 3600).unwrap().0, day_number("2026-11-01").unwrap());
    }

    #[test]
    fn empty_journal_has_no_rate_and_no_panic() {
        let m = compute(&Sources { events: &[], notes: &[], memories: &[], graph: &Graph::default(), today: day_number("2026-11-01").unwrap(), offset: 0 });
        assert_eq!(m.periods.len(), 3);
        assert_eq!(m.periods[0].now.hit_rate(), None);
        assert_eq!(m.streak, 0);
        assert_eq!(m.best_day, "");
    }
}
