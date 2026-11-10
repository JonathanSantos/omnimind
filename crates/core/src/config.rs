//! `brain.toml`: leitor pequeno de um subconjunto de TOML (chave = string, inteiro ou lista de strings).

use crate::error::{Error, Result};
use unicode_normalization::UnicodeNormalization;

pub const FORMAT: u32 = 1;
pub const DEFAULT_MAX_READ_CHARS: usize = 20_000;

#[derive(Debug, Clone, PartialEq)]
pub struct BrainConfig {
    pub format: u32,
    pub id: String,
    pub name: String,
    pub language: Vec<String>,
    pub created_at: String,
    pub allowed_clients: Vec<String>,
    pub max_read_chars: usize,
}

fn parse_string(v: &str) -> Option<String> {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        return serde_json::from_str::<String>(v).ok();
    }
    if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
        return Some(v[1..v.len() - 1].to_string());
    }
    None
}

fn parse_list(v: &str) -> Option<Vec<String>> {
    let v = v.trim();
    let inner = v.strip_prefix('[')?.strip_suffix(']')?;
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote = None;
    let mut esc = false;
    for c in inner.chars() {
        match quote {
            Some(q) => {
                cur.push(c);
                if esc {
                    esc = false;
                } else if c == '\\' && q == '"' {
                    esc = true;
                } else if c == q {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' => {
                    quote = Some(c);
                    cur.push(c);
                }
                ',' => {
                    if !cur.trim().is_empty() {
                        out.push(parse_string(&cur)?);
                    }
                    cur.clear();
                }
                c => cur.push(c),
            },
        }
    }
    if !cur.trim().is_empty() {
        out.push(parse_string(&cur)?);
    }
    Some(out)
}

impl BrainConfig {
    pub fn new(id: String, name: String, created_at: String) -> Self {
        BrainConfig {
            format: FORMAT,
            id,
            name,
            language: vec!["pt-BR".to_string(), "en".to_string()],
            created_at,
            allowed_clients: Vec::new(),
            max_read_chars: DEFAULT_MAX_READ_CHARS,
        }
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut c = BrainConfig::new(String::new(), String::new(), String::new());
        let mut section = String::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') && !line.contains('=') {
                section = line[1..line.len() - 1].trim().to_string();
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                return Err(Error::Config(format!("bad line: {line}")));
            };
            let k = k.trim();
            let v = v.trim();
            match (section.as_str(), k) {
                ("", "format") => {
                    c.format = v.parse().map_err(|_| Error::Config("bad format".into()))?;
                }
                ("", "id") => c.id = parse_string(v).unwrap_or_default(),
                ("", "name") => c.name = parse_string(v).unwrap_or_default(),
                ("", "created_at") => c.created_at = parse_string(v).unwrap_or_default(),
                ("", "language") => {
                    c.language = parse_list(v)
                        .or_else(|| parse_string(v).map(|s| vec![s]))
                        .ok_or_else(|| Error::Config("bad language".into()))?;
                }
                ("" | "ai", "allowed_clients") => {
                    c.allowed_clients =
                        parse_list(v).ok_or_else(|| Error::Config("bad allowed_clients".into()))?;
                }
                ("" | "ai", "max_read_chars") => {
                    c.max_read_chars = v
                        .parse()
                        .map_err(|_| Error::Config("bad max_read_chars".into()))?;
                }
                _ => {}
            }
        }
        if c.id.is_empty() {
            return Err(Error::Config("missing id".into()));
        }
        if c.format != FORMAT {
            return Err(Error::Config(format!("unsupported format {}", c.format)));
        }
        Ok(c)
    }

    pub fn render(&self) -> String {
        let q = |s: &str| serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into());
        let list = |l: &[String]| l.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ");
        format!(
            "format = {}\nid = {}\nname = {}\nlanguage = [{}]\ncreated_at = {}\n\n[ai]\nallowed_clients = [{}]\nmax_read_chars = {}\n",
            self.format,
            q(&self.id),
            q(&self.name),
            list(&self.language),
            q(&self.created_at),
            list(&self.allowed_clients),
            self.max_read_chars
        )
    }

    /// Comparação exata após NFKC. Lista vazia libera qualquer cliente válido.
    pub fn client_allowed(&self, client: &str) -> bool {
        if self.allowed_clients.is_empty() {
            return true;
        }
        let want: String = client.nfkc().collect();
        self.allowed_clients
            .iter()
            .any(|c| c.nfkc().collect::<String>() == want)
    }
}

/// Nome de cliente vem do protocolo. Depois de NFKC só aceita `[a-z0-9._-]`, e nomes reservados caem.
pub fn normalize_client(raw: &str) -> Result<String> {
    if raw.len() > 256 {
        return Err(Error::ClientNotAllowed("name too long".into()));
    }
    let nfkc: String = raw.nfkc().collect::<String>().trim().to_lowercase();
    if crate::inspect::has_invisible(&nfkc) {
        return Err(Error::ClientNotAllowed(
            "invisible characters in client name".into(),
        ));
    }
    let mapped = nfkc.replace(' ', "-");
    let mapped = match mapped.as_str() {
        "visual-studio-code" | "vscode" | "github-copilot" | "copilot-cli" => "copilot".to_string(),
        "claude" | "claudecode" | "claude-code" => "claude-code".to_string(),
        _ => mapped,
    };
    if mapped.is_empty()
        || mapped.len() > 64
        || !mapped
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(Error::ClientNotAllowed("client name not accepted".into()));
    }
    // Compara só as letras: `human.`, `_person_` e `h-u-m-a-n` não passam.
    let letters: String = mapped.chars().filter(char::is_ascii_alphabetic).collect();
    if matches!(
        letters.as_str(),
        "human" | "person" | "pessoa" | "user" | "system" | "omnimind" | "varzea"
    ) {
        return Err(Error::ClientNotAllowed(format!(
            "reserved client name: {mapped}"
        )));
    }
    Ok(mapped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_edgemind_toml() {
        let t = "format = 1\nid = \"01M\"\nname = \"Pessoal\"\nlanguage = [\"pt-BR\", \"en\"]\ncreated_at = \"x\"\n\n[ai]\nallowed_clients = [\"claude-code\"]\nmax_read_chars = 20000\n";
        let c = BrainConfig::parse(t).unwrap();
        assert_eq!(c.name, "Pessoal");
        assert_eq!(c.allowed_clients, vec!["claude-code"]);
        assert_eq!(BrainConfig::parse(&c.render()).unwrap(), c);
    }

    #[test]
    fn client_rules() {
        let mut c = BrainConfig::new("x".into(), "n".into(), "t".into());
        assert!(c.client_allowed("any"));
        c.allowed_clients = vec!["claude-code".into()];
        assert!(c.client_allowed("claude-code"));
        assert!(!c.client_allowed("xclaude-codex"));
        assert!(normalize_client("ｈｕｍａｎ").is_err());
        assert!(normalize_client("human").is_err());
        assert!(normalize_client("human.").is_err());
        assert!(normalize_client("_person_").is_err());
        assert!(normalize_client("hu\u{200B}man").is_err());
        assert!(normalize_client(&"a".repeat(100_000)).is_err());
        assert_eq!(normalize_client("Claude Code").unwrap(), "claude-code");
        assert_eq!(normalize_client("Visual Studio Code").unwrap(), "copilot");
    }
}
