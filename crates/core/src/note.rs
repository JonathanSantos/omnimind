//! Leitura de notas: título, tags, wikilinks fora de código, e resolução de links.

use crate::analyzer::fold;
use crate::frontmatter::{self, get_list, get_str};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct LinkSpan {
    /// Faixa do alvo dentro do texto, sem `#seção` nem `|apelido`.
    pub start: usize,
    pub end: usize,
    pub target: String,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedNote {
    pub title: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub summary: String,
    pub body_offset: usize,
}

/// Faixas de código (cercado e em linha), que não têm link nem tag.
fn code_mask(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut pos = 0;
    let mut fence: Option<(String, usize)> = None;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        let marker: String = t.chars().take_while(|c| *c == '`' || *c == '~').collect();
        if let Some((f, start)) = &fence {
            if marker.len() >= f.len() && marker.starts_with(&f[..1]) && t.trim_end() == marker {
                ranges.push((*start, pos + line.len()));
                fence = None;
            }
        } else if marker.len() >= 3 && (marker.starts_with("```") || marker.starts_with("~~~")) {
            fence = Some((marker, pos));
        } else {
            // código em linha
            let b = line.as_bytes();
            let mut i = 0;
            while i < b.len() {
                if b[i] == b'`' {
                    let mut n = 0;
                    while i + n < b.len() && b[i + n] == b'`' {
                        n += 1;
                    }
                    let open = i;
                    i += n;
                    let mut j = i;
                    let mut closed = false;
                    while j < b.len() {
                        if b[j] == b'`' {
                            let mut m = 0;
                            while j + m < b.len() && b[j + m] == b'`' {
                                m += 1;
                            }
                            if m == n {
                                ranges.push((pos + open, pos + j + m));
                                i = j + m;
                                closed = true;
                                break;
                            }
                            j += m;
                        } else {
                            j += 1;
                        }
                    }
                    if !closed {
                        i = open + n;
                    }
                } else {
                    i += 1;
                }
            }
        }
        pos += line.len();
    }
    if let Some((_, start)) = fence {
        ranges.push((start, text.len()));
    }
    ranges
}

fn in_ranges(r: &[(usize, usize)], pos: usize) -> bool {
    r.iter().any(|(a, b)| pos >= *a && pos < *b)
}

/// Wikilinks fora de código. Aceita `[[alvo]]`, `[[alvo#seção]]`, `[[alvo|apelido]]`, `[[alvo\|apelido]]`.
pub fn wikilinks(text: &str) -> Vec<LinkSpan> {
    let mask = code_mask(text);
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(p) = text[from..].find("[[") {
        let open = from + p;
        let inner_start = open + 2;
        let Some(q) = text[inner_start..].find("]]") else {
            break;
        };
        let inner_end = inner_start + q;
        from = inner_start;
        let inner = &text[inner_start..inner_end];
        if inner.contains('\n') || inner.contains("[[") || in_ranges(&mask, open) {
            continue;
        }
        let mut end = inner.len();
        for sep in ["\\|", "|", "#"] {
            if let Some(i) = inner.find(sep) {
                end = end.min(i);
            }
        }
        let raw = &inner[..end];
        let target = raw.trim();
        if target.is_empty() {
            continue;
        }
        let lead = raw.len() - raw.trim_start().len();
        let start = inner_start + lead;
        out.push(LinkSpan {
            start,
            end: start + target.len(),
            target: target.to_string(),
        });
        from = inner_end + 2;
    }
    out
}

/// Tags em linha (`#projeto/rota-certa`), fora de código e de URL.
pub fn inline_tags(text: &str) -> Vec<String> {
    let mask = code_mask(text);
    let mut out = Vec::new();
    let b = text.as_bytes();
    for (i, c) in text.char_indices() {
        if c != '#' {
            continue;
        }
        let prev_ok = i == 0 || matches!(b[i - 1], b' ' | b'\n' | b'\t' | b'(');
        if !prev_ok || in_ranges(&mask, i) {
            continue;
        }
        let tag: String = text[i + 1..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '/'))
            .collect();
        let tag = tag.trim_end_matches(['/', '-']);
        if !tag.is_empty() && tag.chars().any(char::is_alphabetic) && !out.iter().any(|t| t == tag)
        {
            out.push(tag.to_string());
        }
    }
    out
}

fn file_stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md")
        .or_else(|| name.strip_suffix(".markdown"))
        .unwrap_or(name)
}

pub fn parse_note(path: &str, text: &str) -> ParsedNote {
    let (yaml, body) = frontmatter::split(text);
    let offset = text.len() - body.len();
    let mut n = ParsedNote {
        body_offset: offset,
        ..Default::default()
    };
    let meta = yaml.and_then(frontmatter::parse_yaml);
    if let Some(v) = &meta {
        n.title = get_str(v, "title").unwrap_or_default();
        n.aliases = get_list(v, "aliases");
        n.tags = get_list(v, "tags");
        n.summary = get_str(v, "summary").unwrap_or_default();
    }
    if n.title.trim().is_empty() {
        n.title = body
            .lines()
            .find_map(|l| l.strip_prefix("# ").map(|t| t.trim().to_string()))
            .unwrap_or_else(|| file_stem(path).to_string());
    }
    for t in inline_tags(body) {
        if !n.tags.contains(&t) {
            n.tags.push(t);
        }
    }
    n.links = wikilinks(body).into_iter().map(|l| l.target).collect();
    n
}

pub fn norm_link(s: &str) -> String {
    let f = fold(s.trim());
    f.strip_suffix(".md").unwrap_or(&f).to_string()
}

/// Dados de uma nota para resolver links.
#[derive(Debug, Clone)]
pub struct LinkEntry<'a> {
    pub path: &'a str,
    pub title: &'a str,
    pub aliases: &'a [String],
}

/// Ordem: caminho > nome do arquivo > título > alias. Empate: caminho mais curto, depois alfabético.
#[derive(Default)]
pub struct Resolver {
    by_path: HashMap<String, Vec<String>>,
    by_name: HashMap<String, Vec<String>>,
    by_title: HashMap<String, Vec<String>>,
    by_alias: HashMap<String, Vec<String>>,
}

impl Resolver {
    pub fn remove(&mut self, e: &LinkEntry<'_>) {
        fn drop_from(map: &mut HashMap<String, Vec<String>>, key: &str, path: &str) {
            if let Some(v) = map.get_mut(key) {
                v.retain(|p| p != path);
                if v.is_empty() {
                    map.remove(key);
                }
            }
        }
        let full = norm_link(e.path);
        drop_from(&mut self.by_path, &full, e.path);
        if let Some(rest) = full.strip_prefix("notes/") {
            drop_from(&mut self.by_path, rest, e.path);
        }
        drop_from(&mut self.by_name, &norm_link(file_stem(e.path)), e.path);
        drop_from(&mut self.by_title, &norm_link(e.title), e.path);
        for a in e.aliases {
            drop_from(&mut self.by_alias, &norm_link(a), e.path);
        }
    }

    pub fn add(&mut self, e: &LinkEntry<'_>) {
        let p = e.path.to_string();
        let full = norm_link(e.path);
        self.by_path
            .entry(full.clone())
            .or_default()
            .push(p.clone());
        if let Some(rest) = full.strip_prefix("notes/") {
            self.by_path
                .entry(rest.to_string())
                .or_default()
                .push(p.clone());
        }
        self.by_name
            .entry(norm_link(file_stem(e.path)))
            .or_default()
            .push(p.clone());
        self.by_title
            .entry(norm_link(e.title))
            .or_default()
            .push(p.clone());
        for a in e.aliases {
            self.by_alias
                .entry(norm_link(a))
                .or_default()
                .push(p.clone());
        }
    }

    pub fn build<'a>(entries: impl Iterator<Item = LinkEntry<'a>>) -> Resolver {
        let mut r = Resolver::default();
        for e in entries {
            r.add(&e);
        }
        r
    }

    pub fn resolve(&self, target: &str) -> Option<String> {
        let t = norm_link(target);
        for map in [&self.by_path, &self.by_name, &self.by_title, &self.by_alias] {
            if let Some(v) = map.get(&t) {
                return v.iter().min_by_key(|p| (p.len(), (*p).clone())).cloned();
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_outside_code_only() {
        let t = "a [[Um]] `[[Dois]]`\n```\n[[Tres]]\n```\n[[Quatro|apelido]] [[Cinco#sec]] | [[Seis\\|x]] |";
        let l: Vec<_> = wikilinks(t).into_iter().map(|s| s.target).collect();
        assert_eq!(l, vec!["Um", "Quatro", "Cinco", "Seis"]);
    }

    #[test]
    fn spans_point_at_target() {
        let t = "x [[ Alvo |y]] z";
        let s = &wikilinks(t)[0];
        assert_eq!(&t[s.start..s.end], "Alvo");
    }

    #[test]
    fn tags_skip_urls_and_code() {
        let t = "#projeto/rota-certa e https://x.y/#frag `#code` #123 fim #ok";
        assert_eq!(inline_tags(t), vec!["projeto/rota-certa", "ok"]);
    }

    #[test]
    fn title_priority() {
        assert_eq!(
            parse_note("notes/a.md", "---\ntitle: T\n---\n# H\n").title,
            "T"
        );
        assert_eq!(parse_note("notes/a.md", "# H\ncorpo").title, "H");
        assert_eq!(parse_note("notes/a b.md", "corpo").title, "a b");
    }

    #[test]
    fn resolver_order() {
        let mut r = Resolver::default();
        let a = vec!["Apelido".to_string()];
        r.add(&LinkEntry {
            path: "notes/x/Foo.md",
            title: "Bar",
            aliases: &a,
        });
        r.add(&LinkEntry {
            path: "notes/y/Bar.md",
            title: "Baz",
            aliases: &[],
        });
        r.add(&LinkEntry {
            path: "notes/z/Foo.md",
            title: "Qux",
            aliases: &[],
        });
        assert_eq!(r.resolve("Bar").as_deref(), Some("notes/y/Bar.md"));
        assert_eq!(r.resolve("foo").as_deref(), Some("notes/x/Foo.md"));
        assert_eq!(r.resolve("z/foo").as_deref(), Some("notes/z/Foo.md"));
        assert_eq!(r.resolve("apélido").as_deref(), Some("notes/x/Foo.md"));
        assert_eq!(r.resolve("nada"), None);
    }
}
