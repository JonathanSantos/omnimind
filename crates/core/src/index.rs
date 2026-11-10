//! Índice invertido em RAM com BM25F. Postagem compacta: doc:24 | campo:3 | tf:5.

use crate::analyzer::{Analyzer, Chunks, QueryWord, EN_MARK, PT_MARK};
use crate::util::FastMap;
use std::collections::HashMap;

pub const FIELD_COUNT: usize = 6;
pub const F_TITLE: usize = 0;
pub const F_ALIAS: usize = 1;
pub const F_FILE: usize = 2;
pub const F_TAG: usize = 3;
pub const F_SUMMARY: usize = 4;
pub const F_BODY: usize = 5;
pub const WEIGHTS: [f32; FIELD_COUNT] = [5.0, 4.0, 3.5, 3.0, 2.0, 1.0];

const K1: f32 = 1.2;
const B: f32 = 0.75;
const MAX_TF: u32 = 31;
const NO_TERM: u32 = u32::MAX;
pub const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;

#[derive(Default, Clone, Copy)]
pub struct DocFields<'a> {
    pub title: &'a str,
    pub aliases: &'a [String],
    pub file: &'a str,
    pub tags: &'a [String],
    pub summary: &'a str,
    pub body: &'a str,
}

struct DocEntry {
    key: String,
    lens: [u32; FIELD_COUNT],
    term_ids: Vec<u32>,
}

#[derive(Default)]
pub struct Index {
    terms: FastMap<Box<str>, u32>,
    /// Palavra minúscula já analisada: evita refazer dobra e stemmer. Limpa ao fim do lote.
    word_cache: FastMap<Box<str>, [u32; 3]>,
    term_list: Vec<Box<str>>,
    postings: Vec<Vec<u32>>,
    df: Vec<u32>,
    docs: Vec<Option<DocEntry>>,
    free: Vec<u32>,
    by_key: HashMap<String, u32>,
    total: [u64; FIELD_COUNT],
    live: u32,
}

#[derive(Debug, Clone)]
pub struct Scored {
    pub key: String,
    pub score: f32,
    pub ratio: f32,
    pub all_words: bool,
}

fn encode(doc: u32, field: usize, tf: u32) -> u32 {
    (doc << 8) | ((field as u32) << 5) | tf.min(MAX_TF)
}

fn idf(df: u32, n: u32) -> f32 {
    let n = n.max(1) as f32;
    let df = df as f32;
    (1.0 + (n - df + 0.5) / (df + 0.5)).ln().max(0.01)
}

impl Index {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.live as usize
    }

    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    pub fn contains(&self, key: &str) -> bool {
        self.by_key.contains_key(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.by_key.keys()
    }

    pub fn clear_word_cache(&mut self) {
        self.word_cache = FastMap::default();
    }

    pub fn vocabulary_size(&self) -> usize {
        self.term_list.len()
    }

    fn term_id(&mut self, t: &str) -> u32 {
        if let Some(id) = self.terms.get(t) {
            return *id;
        }
        let id = self.term_list.len() as u32;
        self.terms.insert(t.into(), id);
        self.term_list.push(t.into());
        self.postings.push(Vec::new());
        self.df.push(0);
        id
    }

    pub fn remove(&mut self, key: &str) -> bool {
        let Some(doc) = self.by_key.remove(key) else {
            return false;
        };
        let Some(entry) = self.docs[doc as usize].take() else {
            return false;
        };
        for t in &entry.term_ids {
            let p = &mut self.postings[*t as usize];
            p.retain(|e| e >> 8 != doc);
            self.df[*t as usize] = self.df[*t as usize].saturating_sub(1);
        }
        for (i, l) in entry.lens.iter().enumerate() {
            self.total[i] = self.total[i].saturating_sub(u64::from(*l));
        }
        self.free.push(doc);
        self.live -= 1;
        true
    }

    /// Indexa, substituindo a versão anterior da mesma chave.
    pub fn add(&mut self, key: &str, f: &DocFields<'_>, an: &mut Analyzer) {
        self.remove(key);
        let doc = match self.free.pop() {
            Some(d) => d,
            None => {
                self.docs.push(None);
                (self.docs.len() - 1) as u32
            }
        };
        let mut counts: FastMap<u32, [u16; FIELD_COUNT]> = FastMap::default();
        let mut lens = [0u32; FIELD_COUNT];
        let mut feed = |this: &mut Index, field: usize, text: &str, an: &mut Analyzer| {
            let mut len = 0u32;
            for chunk in (Chunks { text, pos: 0 }) {
                let plain = chunk.len() <= 32
                    && chunk
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit());
                if plain {
                    let ids = match this.word_cache.get(chunk) {
                        Some(ids) => *ids,
                        None => {
                            let mut ids = [NO_TERM; 3];
                            let mut n = 0;
                            an.index_chunk(chunk, &mut |t| {
                                if n < 3 {
                                    ids[n] = this.term_id(t);
                                    n += 1;
                                }
                            });
                            this.word_cache.insert(chunk.into(), ids);
                            ids
                        }
                    };
                    for id in ids {
                        if id != NO_TERM {
                            let e = counts.entry(id).or_insert([0; FIELD_COUNT]);
                            e[field] = e[field].saturating_add(1);
                        }
                    }
                    len += 1;
                } else {
                    an.index_chunk(chunk, &mut |t| {
                        let id = this.term_id(t);
                        let e = counts.entry(id).or_insert([0; FIELD_COUNT]);
                        e[field] = e[field].saturating_add(1);
                        if !t.starts_with(PT_MARK) && !t.starts_with(EN_MARK) {
                            len += 1;
                        }
                    });
                }
            }
            len
        };
        lens[F_TITLE] = feed(self, F_TITLE, f.title, an);
        for a in f.aliases {
            lens[F_ALIAS] += feed(self, F_ALIAS, a, an);
        }
        lens[F_FILE] = feed(self, F_FILE, f.file, an);
        for t in f.tags {
            lens[F_TAG] += feed(self, F_TAG, t, an);
        }
        lens[F_SUMMARY] = feed(self, F_SUMMARY, f.summary, an);
        let body = crate::util::truncate_chars(f.body, MAX_BODY_BYTES);
        lens[F_BODY] = feed(self, F_BODY, body, an);

        let mut term_ids = Vec::with_capacity(counts.len());
        for (t, fields) in counts {
            term_ids.push(t);
            let p = &mut self.postings[t as usize];
            for (fi, tf) in fields.iter().enumerate() {
                if *tf > 0 {
                    p.push(encode(doc, fi, u32::from(*tf)));
                }
            }
            self.df[t as usize] += 1;
        }
        for (i, l) in lens.iter().enumerate() {
            self.total[i] += u64::from(*l);
        }
        self.docs[doc as usize] = Some(DocEntry {
            key: key.to_string(),
            lens,
            term_ids,
        });
        self.by_key.insert(key.to_string(), doc);
        self.live += 1;
    }

    /// Pares de documentos que falam do mesmo assunto: cosseno sobre os termos raros que dividem.
    /// Cada documento fica com os `per_doc` mais parecidos acima de `floor`. Chaves em ordem, sem repetir par.
    /// Cofre grande demais fica de fora: o custo cresce com o quadrado da frequência dos termos.
    pub fn similar(&self, per_doc: usize, floor: f32) -> Vec<(String, String, f32)> {
        let n = self.live;
        if !(3..=3000).contains(&n) {
            return Vec::new();
        }
        // Termo em quase todo documento não distingue assunto; termo de um só não liga ninguém.
        let max_df = ((n as f32).sqrt() * 1.5).ceil().max(3.0) as u32;
        let weight = |t: u32| {
            let d = self.df[t as usize];
            if (2..=max_df).contains(&d) { idf(d, n) } else { 0.0 }
        };
        let mut by_term: FastMap<u32, Vec<u32>> = FastMap::default();
        let mut norm = vec![0f32; self.docs.len()];
        for (i, d) in self.docs.iter().enumerate() {
            let Some(d) = d else { continue };
            for &t in &d.term_ids {
                let w = weight(t);
                if w > 0.0 {
                    norm[i] += w * w;
                    by_term.entry(t).or_default().push(i as u32);
                }
            }
            norm[i] = norm[i].sqrt();
        }
        let key = |i: u32| self.docs[i as usize].as_ref().map_or("", |d| d.key.as_str());
        let mut pairs: Vec<(String, String, f32)> = Vec::new();
        for (a, d) in self.docs.iter().enumerate() {
            let Some(d) = d else { continue };
            if norm[a] == 0.0 {
                continue;
            }
            let mut shared: FastMap<u32, f32> = FastMap::default();
            for &t in &d.term_ids {
                let w = weight(t);
                if w == 0.0 {
                    continue;
                }
                for &b in &by_term[&t] {
                    if b as usize != a {
                        *shared.entry(b).or_insert(0.0) += w * w;
                    }
                }
            }
            let mut best: Vec<(f32, u32)> = shared.into_iter().map(|(b, s)| (s / (norm[a] * norm[b as usize]), b)).filter(|x| x.0 >= floor).collect();
            best.sort_by(|x, y| y.0.total_cmp(&x.0).then(key(x.1).cmp(key(y.1))));
            for (score, b) in best.into_iter().take(per_doc) {
                let (x, y) = (key(a as u32), key(b));
                pairs.push(if x < y { (x.to_string(), y.to_string(), score) } else { (y.to_string(), x.to_string(), score) });
            }
        }
        pairs.sort_by(|x, y| (&x.0, &x.1).cmp(&(&y.0, &y.1)));
        pairs.dedup_by(|b, a| a.0 == b.0 && a.1 == b.1);
        pairs
    }

    fn avg(&self) -> [f32; FIELD_COUNT] {
        let n = self.live.max(1) as f32;
        let mut a = [1.0f32; FIELD_COUNT];
        for (slot, total) in a.iter_mut().zip(self.total.iter()) {
            *slot = (*total as f32 / n).max(1.0);
        }
        a
    }

    /// Todos os documentos que casam com ao menos uma palavra. OR com bônus de todas as palavras.
    pub fn search(&self, words: &[QueryWord]) -> Vec<Scored> {
        let n_docs = self.docs.len();
        if n_docs == 0 || words.is_empty() {
            return Vec::new();
        }
        let avg = self.avg();
        let mut score = vec![0f32; n_docs];
        let mut matched = vec![0f32; n_docs];
        let mut count = vec![0u16; n_docs];
        let mut touched: Vec<u32> = Vec::new();
        let unknown_idf = idf(0, self.live);
        let mut total_idf = 0f32;
        let mut required = 0u16;
        let mut seen_in_word = vec![u32::MAX; n_docs];

        for (wi, w) in words.iter().enumerate() {
            let mut word_idf = 0f32;
            for (term, form_weight) in &w.forms {
                let Some(&t) = self.terms.get(term.as_str()) else {
                    continue;
                };
                let df = self.df[t as usize];
                if df == 0 {
                    continue;
                }
                let term_idf = idf(df, self.live);
                word_idf = word_idf.max(term_idf);
                let post = &self.postings[t as usize];
                let mut i = 0;
                while i < post.len() {
                    let doc = post[i] >> 8;
                    let mut tf_tilde = 0f32;
                    while i < post.len() && post[i] >> 8 == doc {
                        let e = post[i];
                        let field = ((e >> 5) & 7) as usize;
                        let tf = (e & 31) as f32;
                        let len = self.docs[doc as usize]
                            .as_ref()
                            .map_or(1, |d| d.lens[field]) as f32;
                        tf_tilde += WEIGHTS[field] * tf / ((1.0 - B) + B * len / avg[field]);
                        i += 1;
                    }
                    let d = doc as usize;
                    if count[d] == 0 && score[d] == 0.0 {
                        touched.push(doc);
                    }
                    score[d] += form_weight * term_idf * tf_tilde / (K1 + tf_tilde);
                    if seen_in_word[d] != wi as u32 {
                        seen_in_word[d] = wi as u32;
                        if !w.bonus {
                            count[d] += 1;
                        }
                    }
                }
            }
            if !w.bonus {
                required += 1;
                let e = if word_idf > 0.0 {
                    word_idf
                } else {
                    unknown_idf
                };
                total_idf += e;
                // idf ganho por documento é somado depois de varrer a palavra.
                for &doc in &touched {
                    if seen_in_word[doc as usize] == wi as u32 {
                        matched[doc as usize] += e;
                    }
                }
            }
        }

        let mut out = Vec::with_capacity(touched.len());
        for doc in touched {
            let d = doc as usize;
            let Some(entry) = &self.docs[d] else { continue };
            let all = required > 0 && count[d] == required;
            let mut s = score[d];
            if all && required > 1 {
                s *= 1.5;
            }
            let ratio = if total_idf > 0.0 {
                matched[d] / total_idf
            } else {
                1.0
            };
            out.push(Scored {
                key: entry.key.clone(),
                score: s,
                ratio,
                all_words: all,
            });
        }
        out
    }

    /// Melhor termo a distância 1 no vocabulário literal. Só para o caso de nada voltar.
    pub fn fuzzy_term(&self, word: &str) -> Option<String> {
        if word.chars().count() < 4 {
            return None;
        }
        let target: Vec<char> = word.chars().collect();
        let mut best: Option<(u32, &str)> = None;
        for (i, t) in self.term_list.iter().enumerate() {
            let df = self.df[i];
            if df == 0 || t.starts_with(PT_MARK) || t.starts_with(EN_MARK) {
                continue;
            }
            let cand: Vec<char> = t.chars().collect();
            if cand.len().abs_diff(target.len()) > 1 || !within_one(&target, &cand) {
                continue;
            }
            if best.is_none_or(|b| df > b.0 || (df == b.0 && &**t < b.1)) {
                best = Some((df, t));
            }
        }
        best.map(|b| b.1.to_string())
    }

    // Instantâneo binário: tudo validado com limites, nunca panica com arquivo ruim.

    pub fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.term_list.len() as u32);
        for (i, t) in self.term_list.iter().enumerate() {
            put_u32(out, t.len() as u32);
            out.extend_from_slice(t.as_bytes());
            let p = &self.postings[i];
            put_u32(out, p.len() as u32);
            for e in p {
                out.extend_from_slice(&e.to_le_bytes());
            }
        }
        put_u32(out, self.docs.len() as u32);
        for d in &self.docs {
            match d {
                None => out.push(0),
                Some(d) => {
                    out.push(1);
                    put_u32(out, d.key.len() as u32);
                    out.extend_from_slice(d.key.as_bytes());
                    for l in d.lens {
                        put_u32(out, l);
                    }
                }
            }
        }
    }

    pub fn decode(r: &mut Reader<'_>) -> Option<Index> {
        let nterms = r.u32()? as usize;
        let mut idx = Index::default();
        idx.term_list.reserve(nterms);
        idx.terms.reserve(nterms);
        let mut raw_postings = Vec::with_capacity(nterms);
        for i in 0..nterms {
            let len = r.u32()? as usize;
            let t = std::str::from_utf8(r.bytes(len)?).ok()?;
            idx.term_list.push(t.into());
            idx.terms.insert(t.into(), i as u32);
            let plen = r.u32()? as usize;
            let bytes = r.bytes(plen.checked_mul(4)?)?;
            let mut p = Vec::with_capacity(plen);
            for c in bytes.as_chunks::<4>().0 {
                p.push(u32::from_le_bytes(*c));
            }
            raw_postings.push(p);
        }
        let ndocs = r.u32()? as usize;
        idx.docs.reserve(ndocs);
        for i in 0..ndocs {
            match r.u8()? {
                0 => {
                    idx.docs.push(None);
                    idx.free.push(i as u32);
                }
                1 => {
                    let len = r.u32()? as usize;
                    let key = std::str::from_utf8(r.bytes(len)?).ok()?.to_string();
                    let mut lens = [0u32; FIELD_COUNT];
                    for (fi, l) in lens.iter_mut().enumerate() {
                        *l = r.u32()?;
                        idx.total[fi] += u64::from(*l);
                    }
                    idx.by_key.insert(key.clone(), i as u32);
                    idx.docs.push(Some(DocEntry {
                        key,
                        lens,
                        term_ids: Vec::new(),
                    }));
                    idx.live += 1;
                }
                _ => return None,
            }
        }
        idx.df = vec![0; nterms];
        for (t, p) in raw_postings.iter().enumerate() {
            let mut last = u32::MAX;
            for e in p {
                let doc = e >> 8;
                let slot = idx.docs.get_mut(doc as usize)?.as_mut()?;
                if doc != last {
                    slot.term_ids.push(t as u32);
                    idx.df[t] += 1;
                    last = doc;
                }
            }
        }
        idx.postings = raw_postings;
        Some(idx)
    }
}

fn within_one(a: &[char], b: &[char]) -> bool {
    if a == b {
        return false;
    }
    let mut i = 0;
    while i < a.len() && i < b.len() && a[i] == b[i] {
        i += 1;
    }
    if a.len() == b.len() {
        // substituição ou transposição
        if a[i + 1..] == b[i + 1..] {
            return true;
        }
        i + 1 < a.len() && a[i] == b[i + 1] && a[i + 1] == b[i] && a[i + 2..] == b[i + 2..]
    } else if a.len() > b.len() {
        a[i + 1..] == b[i..]
    } else {
        a[i..] == b[i + 1..]
    }
}

pub fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub struct Reader<'a> {
    pub data: &'a [u8],
    pub pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }
    pub fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let s = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(s)
    }
    pub fn u8(&mut self) -> Option<u8> {
        Some(self.bytes(1)?[0])
    }
    pub fn u32(&mut self) -> Option<u32> {
        let b = self.bytes(4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(idx: &mut Index, an: &mut Analyzer, key: &str, title: &str, body: &str) {
        idx.add(
            key,
            &DocFields {
                title,
                body,
                ..Default::default()
            },
            an,
        );
    }

    fn q(idx: &Index, an: &mut Analyzer, s: &str) -> Vec<String> {
        let words = an.query_words(s);
        let mut r = idx.search(&words);
        r.sort_by(|a, b| b.score.total_cmp(&a.score));
        r.into_iter().map(|x| x.key).collect()
    }

    #[test]
    fn title_outranks_body_and_removal_works() {
        let mut idx = Index::new();
        let mut an = Analyzer::new();
        doc(
            &mut idx,
            &mut an,
            "a",
            "outra coisa",
            "cache cache de matriz",
        );
        doc(
            &mut idx,
            &mut an,
            "b",
            "cache da matriz",
            "texto sem relação",
        );
        assert_eq!(q(&idx, &mut an, "cache")[0], "b");
        assert!(idx.remove("b"));
        assert_eq!(q(&idx, &mut an, "cache"), vec!["a"]);
        doc(&mut idx, &mut an, "a", "novo", "sem nada");
        assert!(q(&idx, &mut an, "cache").is_empty());
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn snapshot_round_trip() {
        let mut idx = Index::new();
        let mut an = Analyzer::new();
        for i in 0..50 {
            doc(
                &mut idx,
                &mut an,
                &format!("k{i}"),
                &format!("titulo {i}"),
                "palavras repetidas aqui",
            );
        }
        idx.remove("k3");
        let mut buf = Vec::new();
        idx.encode(&mut buf);
        let mut r = Reader::new(&buf);
        let back = Index::decode(&mut r).unwrap();
        assert_eq!(back.len(), idx.len());
        assert_eq!(q(&back, &mut an, "titulo 7")[0], "k7");
        // arquivo truncado devolve None, não panica
        for cut in [0, 1, 5, buf.len() / 2, buf.len() - 1] {
            let mut r = Reader::new(&buf[..cut]);
            assert!(Index::decode(&mut r).is_none());
        }
    }

    #[test]
    fn fuzzy_within_one() {
        let mut idx = Index::new();
        let mut an = Analyzer::new();
        doc(&mut idx, &mut an, "a", "roteirização", "matriz");
        assert_eq!(idx.fuzzy_term("matirz").as_deref(), Some("matriz"));
        assert_eq!(idx.fuzzy_term("matrizz").as_deref(), Some("matriz"));
        assert_eq!(idx.fuzzy_term("mariz").as_deref(), Some("matriz"));
        assert_eq!(idx.fuzzy_term("banana"), None);
    }

    #[test]
    fn ratio_counts_unknown_words() {
        let mut idx = Index::new();
        let mut an = Analyzer::new();
        doc(&mut idx, &mut an, "a", "projeto rota", "corpo");
        doc(&mut idx, &mut an, "b", "outra nota", "corpo");
        let r = idx.search(&an.query_words("projeto orcamento"));
        assert_eq!(r.len(), 1);
        assert!(r[0].ratio < 0.6, "{}", r[0].ratio);
        let r = idx.search(&an.query_words("projeto rota"));
        assert!((r[0].ratio - 1.0).abs() < 1e-3);
        assert!(r[0].all_words);
    }
}
