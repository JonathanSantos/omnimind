//! Separação e leitura do frontmatter. O serializador é nosso, para ordem estável e cotação segura.

use serde_json::Value;

/// Divide `---\nyaml\n---\ncorpo`. Sem cabeçalho fechado, tudo é corpo.
pub fn split(text: &str) -> (Option<&str>, &str) {
    let t = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    let rest = match t
        .strip_prefix("---\n")
        .or_else(|| t.strip_prefix("---\r\n"))
    {
        Some(r) => r,
        None => return (None, text),
    };
    let mut pos = 0;
    for line in rest.split_inclusive('\n') {
        let l = line.trim_end_matches(['\n', '\r']);
        if l == "---" || l == "..." {
            let body = &rest[pos + line.len()..];
            return (Some(&rest[..pos]), body);
        }
        pos += line.len();
    }
    (None, text)
}

pub fn parse_yaml(yaml: &str) -> Option<Value> {
    if yaml.trim().is_empty() {
        return Some(Value::Object(serde_json::Map::new()));
    }
    match serde_saphyr::from_str::<Value>(yaml) {
        Ok(v @ Value::Object(_)) => Some(v),
        _ => None,
    }
}

pub fn get_str(v: &Value, key: &str) -> Option<String> {
    match v.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Lista YAML, ou string com vírgulas.
pub fn get_list(v: &Value, key: &str) -> Vec<String> {
    match v.get(key) {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| match x {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        Some(Value::String(s)) => s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn plain_safe(s: &str) -> bool {
    if s.is_empty() || s.len() > 200 {
        return false;
    }
    let first = s.chars().next().unwrap_or(' ');
    if !(first.is_alphanumeric()) {
        return false;
    }
    if first.is_ascii_digit() {
        // Data e ULID ficam legíveis; o resto que começa com dígito vai com aspas.
        return crate::util::is_ulid(s)
            || (crate::util::parse_ts(s).is_some()
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'-' | b':' | b'T' | b'Z')));
    }
    if s.ends_with(' ') || s.ends_with(':') {
        return false;
    }
    let lower = s.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "null" | "true" | "false" | "yes" | "no" | "on" | "off" | "y" | "n"
    ) {
        return false;
    }
    if s.parse::<f64>().is_ok()
        || (first.is_ascii_digit()
            && s.bytes()
                .all(|b| b.is_ascii_hexdigit() || b == b'_' || b == b'x'))
    {
        return false;
    }
    for c in s.chars() {
        if matches!(
            c,
            ':' | '#'
                | '\''
                | '"'
                | '['
                | ']'
                | '{'
                | '}'
                | ','
                | '&'
                | '*'
                | '!'
                | '|'
                | '>'
                | '%'
                | '@'
                | '`'
                | '\\'
        ) || c.is_control()
            || !(c.is_ascii() || c.is_alphanumeric() || c == ' ')
        {
            return false;
        }
    }
    true
}

/// Escalar YAML: simples quando seguro, senão aspas duplas com escapes JSON.
pub fn yaml_str(s: &str) -> String {
    if plain_safe(s) {
        return s.to_string();
    }
    yaml_quoted(s)
}

fn yaml_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            '\u{85}' | '\u{a0}' | '\u{2028}' | '\u{2029}' | '\u{feff}' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c if crate::inspect::has_invisible(c.encode_utf8(&mut [0; 4])) => {
                let u = c as u32;
                if u <= 0xFFFF {
                    out.push_str(&format!("\\u{u:04x}"));
                } else {
                    out.push_str(&format!("\\U{u:08x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn yaml_list(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|s| yaml_str(s)).collect();
    format!("[{}]", parts.join(", "))
}

/// Chaves de topo fora de `known`, com as linhas originais, para não destruir nada no round-trip.
pub fn unknown_blocks(yaml: &str, known: &[&str]) -> String {
    let mut out = String::new();
    let mut keep = false;
    for line in yaml.split_inclusive('\n') {
        let top = !line.starts_with([' ', '\t', '-', '#']) && !line.trim().is_empty();
        if top {
            let key = line.split(':').next().unwrap_or("").trim();
            keep = !known.contains(&key);
        }
        if keep {
            out.push_str(line);
        }
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_variants() {
        let (y, b) = split("---\na: 1\n---\ncorpo\n---\nmais");
        assert_eq!(y, Some("a: 1\n"));
        assert_eq!(b, "corpo\n---\nmais");
        assert_eq!(split("sem cabeçalho").0, None);
        assert_eq!(split("---\nnunca fecha\n").0, None);
    }

    #[test]
    fn scalar_round_trip() {
        for s in [
            "simples",
            "com: dois pontos",
            "'aspas'",
            "\"duplas\"",
            "a\nb\n---\nc",
            "null",
            "true",
            "123",
            "1e5",
            "2026-10-31",
            "- item",
            "# comentário",
            "[x, y]",
            "{a: 1}",
            "  espaços  ",
            "ação ñ 漢字 🍲",
            "a\u{200B}b\u{202E}c\u{E0041}",
            "back\\slash",
            "tab\there",
            "",
            "~",
            "&anchor",
            "*alias",
            "!tag",
            "%dir",
            "@at",
            "`tick",
            "0x1f",
            "0o7",
            ".5",
            "-",
            "?",
        ] {
            let y = format!(
                "v: {}\nl: {}\n",
                yaml_str(s),
                yaml_list(&[s.to_string(), "b".into()])
            );
            let v = parse_yaml(&y).unwrap_or_else(|| panic!("parse failed for {s:?}: {y}"));
            assert_eq!(get_str(&v, "v").as_deref(), Some(s), "{y}");
            assert_eq!(get_list(&v, "l")[0], s, "{y}");
        }
    }

    #[test]
    fn hostile_yaml_does_not_panic() {
        assert!(parse_yaml("a: [unterminated").is_none());
        assert!(parse_yaml(": : :").is_none() || parse_yaml(": : :").is_some());
        let bomb = "a: &a [x,x]\nb: &b [*a,*a]\nc: &c [*b,*b]\nd: &d [*c,*c]\ne: [*d,*d]\n";
        let _ = parse_yaml(bomb);
        let deep = "a: ".to_string() + &"[".repeat(5000);
        let _ = parse_yaml(&deep);
    }

    #[test]
    fn unknown_blocks_preserved() {
        let y = "id: x\nfoo:\n  bar: 1\ntags:\n- a\nbaz: 2\n";
        assert_eq!(
            unknown_blocks(y, &["id", "tags"]),
            "foo:\n  bar: 1\nbaz: 2\n"
        );
    }
}
