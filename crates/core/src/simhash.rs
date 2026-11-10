//! SimHash de 64 bits sobre palavras dobradas, e Jaccard de tokens.

use crate::analyzer::fold;
use crate::util::fnv1a;
use std::collections::HashSet;

pub fn words(text: &str) -> Vec<String> {
    fold(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn simhash(text: &str) -> u64 {
    let mut v = [0i32; 64];
    let toks = words(text);
    for t in &toks {
        let h = fnv1a(t.as_bytes());
        for (i, slot) in v.iter_mut().enumerate() {
            if h >> i & 1 == 1 {
                *slot += 1;
            } else {
                *slot -= 1;
            }
        }
    }
    let mut out = 0u64;
    for (i, slot) in v.iter().enumerate() {
        if *slot > 0 {
            out |= 1 << i;
        }
    }
    out
}

pub fn distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

fn set(text: &str) -> HashSet<String> {
    words(text)
        .into_iter()
        .filter(|w| w.len() > 2 && !crate::analyzer::is_stopword(w))
        .collect()
}

pub fn jaccard(a: &str, b: &str) -> f32 {
    let (x, y) = (set(a), set(b));
    if x.is_empty() || y.is_empty() {
        return 0.0;
    }
    let i = x.intersection(&y).count() as f32;
    i / (x.len() + y.len()) as f32 / (1.0 - i / (x.len() + y.len()) as f32)
}

/// Palavras relevantes como hashes ordenados e únicos: comparar é barato.
pub fn word_ids(text: &str) -> Vec<u32> {
    let mut v: Vec<u32> = words(text)
        .into_iter()
        .filter(|w| w.len() > 2 && !crate::analyzer::is_stopword(w))
        .map(|w| fnv1a(w.as_bytes()) as u32)
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

pub fn jaccard_ids(a: &[u32], b: &[u32]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let (mut i, mut j, mut common) = (0, 0, 0usize);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                common += 1;
                i += 1;
                j += 1;
            }
        }
    }
    common as f32 / (a.len() + b.len() - common) as f32
}

/// Impressão de uma memória, calculada na leitura e reaproveitada nas comparações.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Fingerprint {
    pub sim_body: u64,
    pub sim_whole: u64,
    pub word_count: u32,
    pub title: Vec<u32>,
    pub head: Vec<u32>,
    pub body: Vec<u32>,
    pub whole: Vec<u32>,
}

impl Fingerprint {
    pub fn of(title: &str, summary: &str, body: &str) -> Self {
        let text = format!("{summary} {body}");
        let whole = format!("{title} {text}");
        Fingerprint {
            sim_body: simhash(&text),
            sim_whole: simhash(&whole),
            word_count: whole.split_whitespace().count() as u32,
            title: word_ids(title),
            head: word_ids(&format!("{title} {summary}")),
            body: word_ids(&text),
            whole: word_ids(&whole),
        }
    }
}

/// Fração das palavras de `a` que aparecem em `b`.
pub fn containment(a: &str, b: &str) -> f32 {
    let (x, y) = (set(a), set(b));
    if x.is_empty() {
        return 0.0;
    }
    x.intersection(&y).count() as f32 / x.len() as f32
}

const NEGATIONS: &[&str] = &["nao", "nunca", "jamais", "not", "never", "dont"];
const OPPOSITES: &[(&str, &str)] = &[
    ("sempre", "nunca"),
    ("always", "never"),
    ("usa", "nao"),
    ("sim", "nao"),
    ("ligado", "desligado"),
    ("habilitado", "desabilitado"),
    ("enable", "disable"),
];

fn numbers(text: &str) -> Vec<String> {
    let mut out: Vec<String> = text
        .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == ','))
        .map(|s| s.trim_matches(['.', ',']).to_string())
        .filter(|s| s.chars().any(|c| c.is_ascii_digit()))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Heurística: mesma pauta com polaridade ou números diferentes. Conflitos implícitos ficam de fora.
pub fn contradicts(a: &str, b: &str) -> bool {
    let wa = words(a);
    let wb = words(b);
    let has = |w: &[String], t: &str| w.iter().any(|x| x == t);
    let neg_a = NEGATIONS.iter().any(|n| has(&wa, n));
    let neg_b = NEGATIONS.iter().any(|n| has(&wb, n));
    if neg_a != neg_b {
        return true;
    }
    for (x, y) in OPPOSITES {
        if (has(&wa, x) && has(&wb, y)) || (has(&wa, y) && has(&wb, x)) {
            return true;
        }
    }
    let (na, nb) = (numbers(a), numbers(b));
    !na.is_empty() && !nb.is_empty() && na != nb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_texts_have_near_hashes() {
        let a =
            "o deploy do otimizador usa canary com vinte e cinco por cento dos pedidos de ontem";
        let b = "o deploy do otimizador usa canary com vinte e cinco por cento dos pedidos de hoje";
        let c = "receita de bolo de fubá com goiabada e queijo minas para o café da tarde";
        assert!(distance(simhash(a), simhash(b)) < distance(simhash(a), simhash(c)));
    }

    #[test]
    fn jaccard_bounds() {
        assert!((jaccard("cache redis matriz", "cache redis matriz") - 1.0).abs() < 0.01);
        assert_eq!(jaccard("cache redis", "bolo fuba"), 0.0);
        let j = jaccard("cache redis matriz", "cache redis bolo");
        assert!(j > 0.3 && j < 0.7, "{j}");
    }

    #[test]
    fn contradiction_heuristics() {
        assert!(contradicts(
            "canary de 10% dos pedidos",
            "canary de 25% dos pedidos"
        ));
        assert!(contradicts("sempre usa tabs", "nunca usa tabs"));
        assert!(!contradicts(
            "usa tabs no projeto",
            "usa tabs no projeto todo"
        ));
    }
}
