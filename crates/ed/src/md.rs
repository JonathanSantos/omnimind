//! Leitura de Markdown orientada a linhas: rápida o bastante para rodar em toda tecla.

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Body,
    Blank,
    Heading(u8),
    Bullet { indent: u8, task: Option<bool> },
    Ordered { indent: u8 },
    Quote,
    Fence,
    Code,
    Rule,
    Front,
    Table,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Carry {
    pub in_code: bool,
    pub in_front: bool,
}

/// Classifica uma linha dado o estado da anterior; devolve o estado seguinte.
pub fn classify(line: &str, index: usize, carry: Carry) -> (Kind, Carry) {
    let t = line.trim_start();
    let indent = (line.len() - t.len()).min(255) as u8;
    if carry.in_front {
        let close = line.trim_end() == "---" || line.trim_end() == "...";
        return (Kind::Front, Carry { in_front: !close, ..carry });
    }
    if index == 0 && line.trim_end() == "---" {
        return (Kind::Front, Carry { in_front: true, ..carry });
    }
    if t.starts_with("```") || t.starts_with("~~~") {
        return (Kind::Fence, Carry { in_code: !carry.in_code, ..carry });
    }
    if carry.in_code {
        return (Kind::Code, carry);
    }
    if t.is_empty() {
        return (Kind::Blank, carry);
    }
    let hashes = t.bytes().take_while(|b| *b == b'#').count();
    if (1..=6).contains(&hashes) && t.as_bytes().get(hashes) == Some(&b' ') {
        return (Kind::Heading(hashes as u8), carry);
    }
    if is_rule(t) {
        return (Kind::Rule, carry);
    }
    if t.starts_with("> ") || t == ">" {
        return (Kind::Quote, carry);
    }
    for m in ["- ", "* ", "+ "] {
        if let Some(r) = t.strip_prefix(m) {
            let task = if r.starts_with("[ ] ") || r == "[ ]" {
                Some(false)
            } else if r.starts_with("[x] ") || r.starts_with("[X] ") {
                Some(true)
            } else {
                None
            };
            return (Kind::Bullet { indent, task }, carry);
        }
    }
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 && digits < 10 && t[digits..].starts_with(". ") {
        return (Kind::Ordered { indent }, carry);
    }
    if t.starts_with('|') && t.trim_end().ends_with('|') && t.len() > 1 {
        return (Kind::Table, carry);
    }
    (Kind::Body, carry)
}

fn is_rule(t: &str) -> bool {
    let s = t.trim_end();
    s.len() >= 3
        && (s.bytes().all(|b| b == b'-') || s.bytes().all(|b| b == b'*') || s.bytes().all(|b| b == b'_'))
}

/// Tamanho em bytes do prefixo de bloco (`## `, `- `, `> `), que some fora da linha ativa.
pub fn prefix_len(line: &str, kind: Kind) -> usize {
    let t = line.trim_start();
    let indent = line.len() - t.len();
    match kind {
        Kind::Heading(n) => indent + n as usize + 1,
        Kind::Quote => indent + if t.starts_with("> ") { 2 } else { 1 },
        Kind::Bullet { task: Some(_), .. } => indent + 6.min(t.len()),
        Kind::Bullet { .. } => indent + 2,
        Kind::Ordered { .. } => indent + t.bytes().take_while(u8::is_ascii_digit).count() + 2,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Bold,
    Italic,
    Strike,
    Highlight,
    Code,
    Link,
    Wiki,
    Tag,
    Url,
    /// Caractere invisível. É sempre revelado: texto escondido não passa calado.
    Hidden,
}

/// Trecho estilizado. `marks` são as faixas de sintaxe que somem fora da linha ativa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inline {
    pub mark: Mark,
    pub range: Range<usize>,
    pub syntax: Vec<Range<usize>>,
    /// Alvo de link ou wikilink.
    pub target: Option<Range<usize>>,
}

pub fn is_hidden_char(c: char) -> bool {
    matches!(c,
        '\u{00AD}' | '\u{034F}' | '\u{061C}' | '\u{180E}'
        | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}'
        | '\u{2066}'..='\u{206F}' | '\u{FE00}'..='\u{FE0F}' | '\u{FEFF}'
        | '\u{E0000}'..='\u{E0FFF}')
}

pub fn inlines(s: &str) -> Vec<Inline> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let rest = &s[i..];
        if b[i] == b'`' {
            if let Some(end) = rest[1..].find('`') {
                let close = i + 1 + end;
                out.push(Inline {
                    mark: Mark::Code,
                    range: i..close + 1,
                    syntax: vec![i..i + 1, close..close + 1],
                    target: None,
                });
                i = close + 1;
                continue;
            }
        }
        if rest.starts_with("[[") {
            if let Some(end) = rest.find("]]") {
                let inner = i + 2..i + end;
                if !inner.is_empty() && !s[inner.clone()].contains('[') {
                    let (target, mut syntax) = match s[inner.clone()].find('|') {
                        Some(p) => (inner.start..inner.start + p, vec![i..inner.start + p + 1]),
                        None => (inner.clone(), vec![i..i + 2]),
                    };
                    syntax.push(i + end..i + end + 2);
                    out.push(Inline { mark: Mark::Wiki, range: i..i + end + 2, syntax, target: Some(target) });
                    i += end + 2;
                    continue;
                }
            }
        }
        if b[i] == b'[' {
            if let Some(close) = rest.find("](") {
                if let Some(paren) = rest[close..].find(')') {
                    let end = close + paren;
                    if !rest[1..close].contains('\n') && !rest[1..close].contains(']') {
                        out.push(Inline {
                            mark: Mark::Link,
                            range: i..i + end + 1,
                            syntax: vec![i..i + 1, i + close..i + end + 1],
                            target: Some(i + close + 2..i + end),
                        });
                        i += end + 1;
                        continue;
                    }
                }
            }
        }
        let pair = [("**", Mark::Bold), ("__", Mark::Bold), ("~~", Mark::Strike), ("==", Mark::Highlight)];
        let mut hit = false;
        for (m, mark) in pair {
            if rest.starts_with(m) && rest.len() > 4 && !rest[2..].starts_with(' ') {
                if let Some(end) = rest[2..].find(m) {
                    if end > 0 {
                        let close = i + 2 + end;
                        out.push(Inline {
                            mark,
                            range: i..close + 2,
                            syntax: vec![i..i + 2, close..close + 2],
                            target: None,
                        });
                        i += 2;
                        hit = true;
                        break;
                    }
                }
            }
        }
        if hit {
            continue;
        }
        if (b[i] == b'*' || b[i] == b'_') && rest.len() > 2 && !rest[1..].starts_with(' ') {
            let before_ok = i == 0 || !s[..i].chars().next_back().is_some_and(char::is_alphanumeric);
            if before_ok || b[i] == b'*' {
                let ch = &rest[..1];
                if let Some(end) = rest[1..].find(ch) {
                    let close = i + 1 + end;
                    let after_ok = !s[close + 1..].chars().next().is_some_and(char::is_alphanumeric);
                    if end > 0 && b[close - 1] != b' ' && (b[i] == b'*' || after_ok) {
                        out.push(Inline {
                            mark: Mark::Italic,
                            range: i..close + 1,
                            syntax: vec![i..i + 1, close..close + 1],
                            target: None,
                        });
                        i += 1;
                        continue;
                    }
                }
            }
        }
        if b[i] == b'#' {
            let boundary = i == 0 || s[..i].ends_with(' ');
            let n = rest[1..]
                .char_indices()
                .find(|(_, c)| !(c.is_alphanumeric() || matches!(c, '-' | '_' | '/')))
                .map(|(p, _)| p)
                .unwrap_or(rest.len() - 1);
            if boundary && n > 0 && !rest[1..].starts_with(|c: char| c.is_ascii_digit()) {
                out.push(Inline { mark: Mark::Tag, range: i..i + 1 + n, syntax: vec![], target: None });
                i += 1 + n;
                continue;
            }
        }
        if rest.starts_with("http://") || rest.starts_with("https://") {
            let n = rest.find(|c: char| c.is_whitespace() || c == ')').unwrap_or(rest.len());
            out.push(Inline { mark: Mark::Url, range: i..i + n, syntax: vec![], target: Some(i..i + n) });
            i += n;
            continue;
        }
        let c = rest.chars().next().unwrap_or(' ');
        if is_hidden_char(c) {
            out.push(Inline { mark: Mark::Hidden, range: i..i + c.len_utf8(), syntax: vec![], target: None });
        }
        i += c.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(t: &str) -> Vec<Kind> {
        let mut c = Carry::default();
        t.lines()
            .enumerate()
            .map(|(i, l)| {
                let (k, n) = classify(l, i, c);
                c = n;
                k
            })
            .collect()
    }

    #[test]
    fn blocks() {
        let k = kinds("---\ntitulo: x\n---\n# Um\n\n- [x] feito\n1. dois\n> cita\n```\n# não\n```\n---\n| a | b |");
        assert_eq!(k[0], Kind::Front);
        assert_eq!(k[2], Kind::Front);
        assert_eq!(k[3], Kind::Heading(1));
        assert_eq!(k[5], Kind::Bullet { indent: 0, task: Some(true) });
        assert_eq!(k[9], Kind::Code);
        assert_eq!(k[11], Kind::Rule);
        assert_eq!(k[12], Kind::Table);
    }

    #[test]
    fn inline_marks_and_acentos() {
        let i = inlines("ação **forte** e *leve* `cód` [[Nota|alias]] #tag/sub [a](b)");
        let marks: Vec<_> = i.iter().map(|x| x.mark).collect();
        assert_eq!(marks, [Mark::Bold, Mark::Italic, Mark::Code, Mark::Wiki, Mark::Tag, Mark::Link]);
    }

    #[test]
    fn hidden_chars_are_flagged() {
        let i = inlines("ok\u{200B}x\u{202E}y\u{E0041}");
        assert_eq!(i.iter().filter(|x| x.mark == Mark::Hidden).count(), 3);
    }

    #[test]
    fn never_panics_on_multibyte_edges() {
        for s in ["*é", "**é**", "[é](", "`é", "#é", "é*é*é", "_a_é", "__", "**", "*", "=é=", "[[é", "[[]]"] {
            let _ = inlines(s);
        }
    }
}
