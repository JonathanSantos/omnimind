//! Normalização de tags com facetas e vocabulário.

use crate::analyzer::fold;
use crate::model::MemoryType;
use unicode_normalization::UnicodeNormalization;

const FACETS: &[(&str, &str)] = &[
    ("project", "project"),
    ("projeto", "project"),
    ("projetos", "project"),
    ("area", "area"),
    ("phase", "phase"),
    ("fase", "phase"),
    ("source", "source"),
    ("fonte", "source"),
];

const TYPE_WORDS: &[&str] = &[
    "decision",
    "decisions",
    "decisao",
    "decisoes",
    "procedure",
    "procedures",
    "procedimento",
    "procedimentos",
    "concept",
    "concepts",
    "conceito",
    "conceitos",
    "episode",
    "episodes",
    "episodio",
    "episodios",
    "preference",
    "preferences",
    "preferencia",
    "preferencias",
    "memory",
    "memoria",
    "memories",
    "memorias",
];

#[derive(Debug, Clone, PartialEq)]
pub enum TagResult {
    Ok(String),
    /// Repete o tipo da memória.
    RepeatsType,
    Invalid,
}

/// NFKC, minúsculas, sem acento, sem espaço, no máximo 2 níveis, faceta canônica.
pub fn normalize(raw: &str) -> TagResult {
    let t: String = raw.nfkc().collect();
    let t = t.trim().trim_start_matches('#').trim();
    if t.is_empty() || t.len() > 80 || t.contains("..") || t.starts_with('/') {
        return TagResult::Invalid;
    }
    let f = fold(t).replace([' ', '_'], "-");
    if !f
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '/')
    {
        return TagResult::Invalid;
    }
    let parts: Vec<&str> = f.split('/').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() || parts.len() > 2 {
        return TagResult::Invalid;
    }
    if parts.len() == 1 && TYPE_WORDS.contains(&parts[0]) {
        return TagResult::RepeatsType;
    }
    if parts.len() == 2 {
        let facet = FACETS
            .iter()
            .find(|(k, _)| *k == parts[0])
            .map_or(parts[0], |x| x.1);
        return TagResult::Ok(format!("{facet}/{}", parts[1]));
    }
    TagResult::Ok(parts[0].to_string())
}

/// Projeto de uma memória, se ela pertence a um.
pub fn project_of(tags: &[String]) -> Option<&str> {
    tags.iter().find_map(|t| t.strip_prefix("project/"))
}

/// `rota-certa` vira `project/rota-certa` quando o vocabulário já conhece a forma com faceta.
pub fn canonical(tag: &str, vocabulary: &[String]) -> String {
    if tag.contains('/') {
        return tag.to_string();
    }
    let suffix = format!("/{tag}");
    let mut hits: Vec<&String> = vocabulary.iter().filter(|v| v.ends_with(&suffix)).collect();
    hits.sort();
    hits.dedup();
    if hits.len() == 1 {
        return hits[0].clone();
    }
    tag.to_string()
}

/// Tags mais próximas no vocabulário, por palavras em comum.
pub fn nearest(wanted: &[String], title: &str, vocabulary: &[String], limit: usize) -> Vec<String> {
    let needles: Vec<String> = wanted
        .iter()
        .flat_map(|t| t.split(['/', '-']).map(str::to_string).collect::<Vec<_>>())
        .chain(crate::simhash::words(title))
        .filter(|w| w.len() > 2)
        .collect();
    let mut scored: Vec<(usize, &String)> = vocabulary
        .iter()
        .map(|v| {
            let score = v
                .split(['/', '-'])
                .filter(|p| {
                    needles
                        .iter()
                        .any(|n| n == p || n.starts_with(p) || p.starts_with(n.as_str()))
                })
                .count();
            (score, v)
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    scored
        .into_iter()
        .take(limit)
        .map(|x| x.1.clone())
        .collect()
}

pub fn repeats_type(tag: &str, t: MemoryType) -> bool {
    tag == t.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization() {
        assert_eq!(
            normalize("Projeto/Rota Certa"),
            TagResult::Ok("project/rota-certa".into())
        );
        assert_eq!(normalize("#área/git"), TagResult::Ok("area/git".into()));
        assert_eq!(normalize("decisao"), TagResult::RepeatsType);
        assert_eq!(normalize("Procedimento"), TagResult::RepeatsType);
        for t in ["../../x", "a/b/c", "", "#", "a$b"] {
            assert_eq!(normalize(t), TagResult::Invalid, "{t}");
        }
    }

    #[test]
    fn facet_unification() {
        let v = vec!["project/rota-certa".to_string()];
        assert_eq!(canonical("rota-certa", &v), "project/rota-certa");
        assert_eq!(canonical("outra", &v), "outra");
    }
}
