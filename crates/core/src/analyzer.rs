//! Analisador igual no índice e na consulta: dobra, identificadores, plurais PT, stemmers PT e EN.

use rust_stemmers::{Algorithm, Stemmer};
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

/// Sobe quando muda a análise; descarta instantâneos antigos.
pub const ANALYZER_VERSION: u32 = 2;

pub const PT_MARK: char = '\u{1}';
pub const EN_MARK: char = '\u{2}';

const MAX_WORD: usize = 64;
const MAX_COMPOUND: usize = 160;

/// NFKD, sem marcas, minúsculas e ligaduras em ASCII.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.nfkd() {
        if unicode_normalization::char::is_combining_mark(c) {
            continue;
        }
        match c {
            'ß' => out.push_str("ss"),
            'æ' | 'Æ' => out.push_str("ae"),
            'œ' | 'Œ' => out.push_str("oe"),
            'ø' | 'Ø' => out.push('o'),
            'đ' | 'Đ' => out.push('d'),
            'ł' | 'Ł' => out.push('l'),
            c => out.extend(c.to_lowercase()),
        }
    }
    out
}

fn is_connector(c: char) -> bool {
    matches!(c, '_' | '-' | '.' | ':' | '/' | '\\' | '@')
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF)
}

/// Plurais irregulares que o Snowball PT deixa separados, já sobre texto dobrado.
pub fn singularize(w: &str) -> String {
    let n = w.len();
    let rep = |suffix_len: usize, new: &str| format!("{}{new}", &w[..n - suffix_len]);
    if n >= 5 && w.ends_with("oes") {
        return rep(3, "ao");
    }
    if n >= 4 && w.ends_with("aes") {
        return rep(3, "ao");
    }
    if n >= 4 && w.ends_with("aos") {
        return rep(3, "ao");
    }
    if n >= 5 && w.ends_with("ais") {
        return rep(3, "al");
    }
    if n >= 5 && w.ends_with("eis") {
        return rep(3, "el");
    }
    if n >= 6 && w.ends_with("ois") {
        return rep(3, "ol");
    }
    if n >= 5 && w.ends_with("ns") {
        return rep(2, "m");
    }
    w.to_string()
}

const STOP: &[&str] = &[
    // português
    "a", "o", "as", "os", "um", "uma", "uns", "umas", "de", "do", "da", "dos", "das", "em", "no",
    "na", "nos", "nas", "por", "pelo", "pela", "pelos", "pelas", "para", "pra", "pro", "com",
    "sem", "e", "ou", "que", "se", "ao", "aos", "qual", "quais", "quem", "como", "onde", "quando",
    "quanto", "porque", "por que", "eu", "tu", "ele", "ela", "nos", "voce", "meu", "minha", "seu",
    "sua", "esse", "essa", "este", "esta", "isso", "isto", "foi", "ser", "sao", "tem", "ter", "ha",
    "mais", "muito", "mas", "ja", "tambem", "so", "entre", "sobre", "num", "numa", "dum", "duma",
    "lhe", "fica", "ficam", "posso", "pode", "faz", "fazer", "depois", "antes", "ate",
    // inglês
    "the", "an", "of", "to", "in", "on", "at", "for", "and", "or", "is", "are", "was", "were", "be",
    "it", "this", "that", "with", "as", "by", "what", "which", "who", "how", "where", "when",
    "why", "do", "does", "did", "can", "i", "you", "we", "they", "my", "your",
];

pub fn is_stopword(w: &str) -> bool {
    STOP.contains(&w)
}

/// Um trecho contínuo com identificador preservado.
pub(crate) struct Chunks<'a> {
    pub text: &'a str,
    pub pos: usize,
}

impl<'a> Iterator for Chunks<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        let bytes = self.text;
        let mut start = None;
        let mut end = self.pos;
        let iter = bytes[self.pos..].char_indices();
        for (i, c) in iter {
            let abs = self.pos + i;
            if c.is_alphanumeric() {
                if start.is_none() {
                    start = Some(abs);
                }
                end = abs + c.len_utf8();
            } else if start.is_some() && is_connector(c) {
                // Conectores em sequência (`brain::open`) só unem se vier letra depois.
                let next = bytes[abs..].chars().find(|n| !is_connector(*n));
                if !next.is_some_and(char::is_alphanumeric) {
                    self.pos = abs + c.len_utf8();
                    return Some(&bytes[start?..end]);
                }
            } else if let Some(s) = start {
                self.pos = abs + c.len_utf8();
                return Some(&bytes[s..end]);
            }
        }
        self.pos = bytes.len();
        start.map(|s| &bytes[s..end])
    }
}

/// Partes de camelCase. `rotaCerta` vira `rota`, `Certa`. Sem transição devolve o próprio.
fn camel_parts(w: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = w.char_indices().collect();
    for i in 1..chars.len() {
        let (pos, c) = chars[i];
        let (_, prev) = chars[i - 1];
        let next_lower = chars.get(i + 1).is_some_and(|x| x.1.is_lowercase());
        if c.is_uppercase() && (prev.is_lowercase() || (prev.is_uppercase() && next_lower)) {
            out.push(&w[start..pos]);
            start = pos;
        }
    }
    out.push(&w[start..]);
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueryWord {
    pub text: String,
    /// (termo, peso): literal 2, stem 1.
    pub forms: Vec<(String, f32)>,
    /// Forma composta opcional: pontua, mas não conta como palavra exigida.
    pub bonus: bool,
}

type StemPair = (Option<Box<str>>, Option<Box<str>>);

pub struct Analyzer {
    pt: Stemmer,
    en: Stemmer,
    cache: HashMap<String, StemPair>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer {
            pt: Stemmer::create(Algorithm::Portuguese),
            en: Stemmer::create(Algorithm::English),
            cache: HashMap::new(),
        }
    }

    pub fn clear_cache(&mut self) {
        self.cache = HashMap::new();
    }

    fn stems(&mut self, w: &str) -> StemPair {
        if let Some(v) = self.cache.get(w) {
            return v.clone();
        }
        let alpha = w.len() >= 3 && w.bytes().all(|b| b.is_ascii_lowercase());
        let mut res = (None, None);
        if alpha {
            res.1 = Some(format!("{EN_MARK}{}", self.en.stem(w)).into_boxed_str());
            if w.len() >= 4 || w.ends_with("ao") {
                let s = singularize(w);
                res.0 = Some(format!("{PT_MARK}{}", self.pt.stem(&s)).into_boxed_str());
            }
        }
        if self.cache.len() > 200_000 {
            self.cache.clear();
        }
        self.cache.insert(w.to_string(), res.clone());
        res
    }

    fn word(&mut self, folded: &str, emit: &mut dyn FnMut(&str)) {
        if folded.is_empty() || folded.len() > MAX_WORD {
            return;
        }
        emit(folded);
        let (pt, en) = self.stems(folded);
        if let Some(p) = pt {
            emit(&p);
        }
        if let Some(e) = en {
            emit(&e);
        }
    }

    /// Emite todos os termos do texto, um por ocorrência.
    pub fn index_terms(&mut self, text: &str, emit: &mut dyn FnMut(&str)) {
        let chunks = Chunks { text, pos: 0 };
        for chunk in chunks {
            self.index_chunk(chunk, emit);
        }
    }

    pub(crate) fn index_chunk(&mut self, chunk: &str, emit: &mut dyn FnMut(&str)) {
        if chunk.chars().any(is_cjk) {
            let chars: Vec<char> = chunk.chars().collect();
            let mut buf = String::new();
            for w in chars.windows(2) {
                buf.clear();
                buf.extend(w.iter());
                emit(&buf);
            }
            if chars.len() == 1 {
                emit(chunk);
            }
            return;
        }
        let compound = chunk.chars().any(is_connector);
        if compound && chunk.len() <= MAX_COMPOUND {
            emit(&fold(chunk));
        }
        for piece in chunk.split(is_connector) {
            if piece.is_empty() {
                continue;
            }
            let parts = camel_parts(piece);
            if parts.len() > 1 {
                emit(&fold(piece));
                for part in parts {
                    self.word(&fold(part), emit);
                }
            } else {
                self.word(&fold(piece), emit);
            }
        }
    }

    /// Palavras da consulta, com formas e pesos. Sem stopwords, salvo se só houver stopwords.
    pub fn query_words(&mut self, q: &str) -> Vec<QueryWord> {
        let mut out: Vec<QueryWord> = Vec::new();
        let chunks: Vec<&str> = Chunks { text: q, pos: 0 }.collect();
        for chunk in chunks {
            let compound = chunk.chars().any(is_connector);
            if compound && chunk.len() <= MAX_COMPOUND {
                let f = fold(chunk);
                out.push(QueryWord {
                    text: f.clone(),
                    forms: vec![(f, 2.0)],
                    bonus: true,
                });
            }
            for piece in chunk.split(is_connector).filter(|p| !p.is_empty()) {
                let parts = camel_parts(piece);
                if parts.len() > 1 {
                    let f = fold(piece);
                    out.push(QueryWord {
                        text: f.clone(),
                        forms: vec![(f, 2.0)],
                        bonus: true,
                    });
                }
                for part in parts {
                    let mut terms: Vec<String> = Vec::new();
                    let f = fold(part);
                    self.word(&f, &mut |t| terms.push(t.to_string()));
                    if terms.is_empty() {
                        continue;
                    }
                    let forms = terms
                        .into_iter()
                        .enumerate()
                        .map(|(i, t)| (t, if i == 0 { 2.0 } else { 1.0 }))
                        .collect();
                    out.push(QueryWord {
                        text: f,
                        forms,
                        bonus: false,
                    });
                }
            }
        }
        // Stopword pontua pelo IDF, mas não é exigida nem conta na cobertura da consulta.
        if out.iter().any(|w| !w.bonus && !is_stopword(&w.text)) {
            for w in &mut out {
                if is_stopword(&w.text) {
                    w.bonus = true;
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        out.retain(|w| seen.insert(w.text.clone()));
        out.truncate(16);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(s: &str) -> Vec<String> {
        let mut a = Analyzer::new();
        let mut v = Vec::new();
        a.index_terms(s, &mut |t| v.push(t.to_string()));
        v
    }

    fn shares_stem(a: &str, b: &str) -> bool {
        let ta = terms(a);
        let tb = terms(b);
        ta.iter().any(|t| tb.contains(t))
    }

    #[test]
    fn fold_removes_accents() {
        assert_eq!(fold("Decisões ﬁnal ÇÃO"), "decisoes final cao");
    }

    #[test]
    fn irregular_plurals_meet() {
        for (a, b) in [
            ("aviões", "avião"),
            ("decisões", "decisão"),
            ("revisões", "revisão"),
            ("pães", "pão"),
            ("mãos", "mão"),
            ("animais", "animal"),
            ("papéis", "papel"),
            ("viagens", "viagem"),
            ("corridas", "corrida"),
            ("escalando", "escala"),
            ("porções", "porção"),
        ] {
            assert!(shares_stem(a, b), "{a} {b}");
        }
    }

    #[test]
    fn identifiers_split() {
        let t = terms("rotaCerta rota_certa brain::open ERR_CONNECTION_RESET");
        for w in [
            "rotacerta",
            "rota",
            "certa",
            "rota_certa",
            "brain::open",
            "open",
            "err_connection_reset",
            "reset",
        ] {
            assert!(t.contains(&w.to_string()), "{w}: {t:?}");
        }
    }

    #[test]
    fn stem_markers_keep_languages_apart() {
        assert!(!shares_stem("note", "not"));
    }

    #[test]
    fn trailing_punctuation_ignored() {
        assert_eq!(terms("fim.")[0], "fim");
        assert!(terms("a: b").contains(&"a".to_string()));
    }

    #[test]
    fn query_drops_stopwords_but_not_all() {
        let mut a = Analyzer::new();
        let q = a.query_words("onde ficam guardados os pratos da família");
        let required: Vec<_> = q
            .iter()
            .filter(|w| !w.bonus)
            .map(|w| w.text.as_str())
            .collect();
        assert_eq!(required, vec!["guardados", "pratos", "familia"]);
        assert!(
            q.iter().any(|w| w.bonus && w.text == "onde"),
            "stopword continua pontuando"
        );
        assert!(
            a.query_words("o que").iter().all(|w| !w.bonus),
            "só stopwords: todas contam"
        );
    }

    #[test]
    fn cjk_bigrams() {
        assert_eq!(terms("漢字書").len(), 2);
    }
}
