//! Projeto como entidade. Quem diz que um projeto existe é uma nota, não o nome da pasta
//! onde a sessão rodou: a nota dá o grupo, as pastas e os apelidos.
//! Tudo aqui é puro: recebe texto e caminhos, devolve decisões.

use crate::util;
use serde::{Deserialize, Serialize};

/// O que uma nota declara sobre o projeto que descreve.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decl {
    pub group: String,
    pub folders: Vec<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub slug: String,
    pub title: String,
    /// Caminho da nota que declara o projeto.
    pub note: String,
    pub group: String,
    pub folders: Vec<String>,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Projects {
    pub list: Vec<Project>,
}

/// Onde uma sessão está rodando, do ponto de vista dos projetos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// Pasta de um projeto que tem nota.
    Project(String),
    /// Pasta de trabalho sem nota: vale como projeto provisório, com o nome da pasta.
    Unregistered(String),
    /// Pasta descartável: nunca é um projeto.
    Ephemeral(String),
}

const GROUP_KEYS: [&str; 2] = ["grupo", "group"];
const FOLDER_KEYS: [&str; 4] = ["pasta", "pastas", "folder", "folders"];
const ALIAS_KEYS: [&str; 3] = ["apelidos", "apelido", "aliases"];
/// O cabeçalho de uma nota de projeto fica no começo: não vale procurar no texto inteiro.
const HEADER_LINES: usize = 40;

fn clean(v: &str) -> String {
    v.trim().trim_matches(|c| c == '`' || c == '"' || c == '\'').trim().to_string()
}

fn list(v: &str) -> Vec<String> {
    v.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(clean)
        .filter(|s| !s.is_empty())
        .collect()
}

/// `- **Grupo:** Projetos` no corpo ou `group: Projetos` no frontmatter. Devolve (chave, valor).
fn field(line: &str) -> Option<(String, &str)> {
    let t = line.trim();
    let t = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).unwrap_or(t).trim_start();
    let (key, value) = if let Some(rest) = t.strip_prefix("**") {
        let (k, v) = rest.split_once("**")?;
        (k.trim_end_matches(':'), v.trim_start_matches(':'))
    } else {
        t.split_once(':')?
    };
    let key = key.trim().to_lowercase();
    if key.is_empty() || key.contains(' ') {
        return None;
    }
    Some((key, value.trim()))
}

/// Lê a declaração de projeto de uma nota. `None` quando a nota não é de projeto.
pub fn declare(text: &str) -> Option<Decl> {
    let mut d = Decl::default();
    let mut found = false;
    let mut fenced = false;
    for line in text.lines().take(HEADER_LINES) {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let Some((key, value)) = field(line) else { continue };
        if value.is_empty() {
            continue;
        }
        if GROUP_KEYS.contains(&key.as_str()) && d.group.is_empty() {
            d.group = clean(value);
            found = true;
        } else if FOLDER_KEYS.contains(&key.as_str()) && d.folders.is_empty() {
            d.folders = list(value);
            found = true;
        } else if ALIAS_KEYS.contains(&key.as_str()) && d.aliases.is_empty() {
            d.aliases = list(value);
        }
    }
    found.then_some(d)
}

fn norm_path(p: &str) -> String {
    let p = p.trim().replace('\\', "/");
    let p = p.trim_end_matches('/');
    p.strip_prefix('~').unwrap_or(p).to_string()
}

fn base_name(cwd: &str) -> String {
    let p = norm_path(cwd);
    util::slug(p.rsplit('/').next().unwrap_or(&p))
}

/// Pasta descartável: temporária do sistema ou com nome de rascunho.
pub fn is_ephemeral(cwd: &str) -> bool {
    let p = norm_path(cwd).to_lowercase();
    let name = base_name(cwd);
    const TEMP: [&str; 4] = ["/tmp/", "/private/tmp/", "/var/folders/", "/var/tmp/"];
    let dir = format!("{p}/");
    let inside_temp = TEMP.iter().any(|t| dir.starts_with(t)) || dir.contains("/appdata/local/temp/");
    inside_temp || name.starts_with("scratch") || name == "tmp" || name == "temp" || name.starts_with("tmp-")
}

impl Projects {
    /// Monta a lista a partir das notas que declaram projeto: (caminho, título, declaração).
    pub fn build<'a>(notes: impl Iterator<Item = (&'a str, &'a str, &'a Decl)>) -> Self {
        let mut list: Vec<Project> = notes
            .map(|(path, title, d)| {
                let stem = path.rsplit('/').next().unwrap_or(path).trim_end_matches(".md");
                let slug = util::slug(stem);
                let mut aliases: Vec<String> = d.aliases.iter().map(|a| util::slug(a)).filter(|a| !a.is_empty() && *a != slug).collect();
                // Pasta escrita só com o nome vale como apelido: `Pasta: toca-discos`.
                let mut folders = Vec::new();
                for f in &d.folders {
                    let n = norm_path(f);
                    if n.contains('/') {
                        folders.push(n);
                    } else if !n.is_empty() {
                        aliases.push(util::slug(&n));
                    }
                }
                for f in &folders {
                    let last = util::slug(f.rsplit('/').next().unwrap_or(f));
                    if last != slug && !last.is_empty() {
                        aliases.push(last);
                    }
                }
                aliases.sort();
                aliases.dedup();
                Project { slug, title: title.to_string(), note: path.to_string(), group: d.group.clone(), folders, aliases }
            })
            .filter(|p| !p.slug.is_empty())
            .collect();
        list.sort_by(|a, b| a.slug.cmp(&b.slug).then(a.note.cmp(&b.note)));
        list.dedup_by(|b, a| a.slug == b.slug);
        Projects { list }
    }

    /// Projeto por slug ou apelido.
    pub fn get(&self, name: &str) -> Option<&Project> {
        let n = util::slug(name);
        self.list.iter().find(|p| p.slug == n).or_else(|| self.list.iter().find(|p| p.aliases.contains(&n)))
    }

    /// Nome canônico de um projeto: o apelido vira o slug da nota; o que não tem nota fica como está.
    pub fn canon(&self, name: &str) -> String {
        self.get(name).map_or_else(|| util::slug(name), |p| p.slug.clone())
    }

    pub fn of_note(&self, path: &str) -> Option<&Project> {
        self.list.iter().find(|p| p.note == path)
    }

    /// Título no formato `trilho: índice das decisões`: o começo nomeia o projeto.
    pub fn named_in_title(&self, title: &str) -> Option<&Project> {
        let (head, rest) = title.split_once(':')?;
        if rest.trim().is_empty() || head.trim().contains(' ') && head.split_whitespace().count() > 3 {
            return None;
        }
        let head = util::slug(head);
        self.list.iter().find(|p| p.slug == head || p.aliases.contains(&head) || util::slug(&p.title) == head)
    }

    pub fn place(&self, cwd: &str) -> Place {
        let path = norm_path(cwd);
        let name = base_name(cwd);
        let inside = |f: &String| path.ends_with(f.as_str()) || path.contains(&format!("{f}/"));
        if let Some(p) = self.list.iter().find(|p| p.folders.iter().any(inside)) {
            return Place::Project(p.slug.clone());
        }
        if is_ephemeral(cwd) {
            return Place::Ephemeral(name);
        }
        match self.get(&name) {
            Some(p) => Place::Project(p.slug.clone()),
            None => Place::Unregistered(name),
        }
    }

    /// Projeto que vale para uma sessão nesta pasta. Pasta descartável não tem projeto.
    pub fn current(&self, cwd: &str) -> Option<String> {
        match self.place(cwd) {
            Place::Project(s) | Place::Unregistered(s) => Some(s),
            Place::Ephemeral(_) => None,
        }
    }

    /// Como chamar o projeto na tela e no resumo: o título da nota, ou o próprio nome.
    pub fn title(&self, name: &str) -> String {
        self.get(name).map_or_else(|| name.to_string(), |p| p.title.clone())
    }
}

/// Texto inicial da nota de um projeto novo, no formato que `declare` entende.
pub fn note_template(title: &str, group: &str, folder: Option<&str>, aliases: &[String]) -> String {
    let mut s = format!("# {title}\n\n- **Grupo:** {group}\n");
    if let Some(f) = folder.filter(|f| !f.trim().is_empty()) {
        s.push_str(&format!("- **Pasta:** `{}`\n", f.trim()));
    }
    if !aliases.is_empty() {
        s.push_str(&format!("- **Apelidos:** {}\n", aliases.join(", ")));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRILHO: &str = "# trilho\n\nApp desktop local.\n\n- **Grupo:** Projetos\n- **Stack:** Rust, Tauri\n- **Pasta:** `~/Developer/projects/trilho`\n";

    fn sample() -> Projects {
        let a = declare(TRILHO).unwrap();
        let b = declare("# Vitrola\n\n- **Grupo:** Projetos\n- **Pasta:** `~/Developer/projects/toca-discos`\n- **Apelidos:** toca, Radiola\n").unwrap();
        let c = declare("---\ngroup: Estudos\nfolder: mina\n---\n# Mina\n").unwrap();
        Projects::build([("notes/projetos/trilho.md", "trilho", &a), ("notes/projetos/vitrola.md", "Vitrola", &b), ("notes/estudos/mina.md", "Mina", &c)].into_iter())
    }

    #[test]
    fn a_note_declares_a_project_with_bullets_or_frontmatter() {
        let d = declare(TRILHO).unwrap();
        assert_eq!(d.group, "Projetos");
        assert_eq!(d.folders, vec!["~/Developer/projects/trilho"]);
        let d = declare("---\ngroup: Jogos\nfolders: [~/a/b, ~/c/d]\naliases: [x, y]\n---\ntexto").unwrap();
        assert_eq!((d.group.as_str(), d.folders.len(), d.aliases.len()), ("Jogos", 2, 2));
        assert!(declare("# Panorama\n\nAgregado de 44 projetos.\n\n## Stacks\n").is_none(), "nota comum não vira projeto");
        assert!(declare("# Nota\n\n```\n- **Grupo:** dentro de código\n```\n").is_none());
        let far = format!("# Longe\n{}- **Grupo:** tarde demais\n", "linha\n".repeat(60));
        assert!(declare(&far).is_none(), "só o cabeçalho declara");
    }

    #[test]
    fn folder_and_aliases_place_a_session_in_its_project() {
        let p = sample();
        assert_eq!(p.place("/Users/x/Developer/projects/trilho"), Place::Project("trilho".into()));
        assert_eq!(p.place("/Users/x/Developer/projects/trilho/crates/core"), Place::Project("trilho".into()), "subpasta é o mesmo projeto");
        // O produto se chama Vitrola, a pasta se chama toca-discos.
        assert_eq!(p.place("/Users/x/Developer/projects/toca-discos"), Place::Project("vitrola".into()));
        assert_eq!(p.place("/outra/raiz/toca-discos"), Place::Project("vitrola".into()), "o nome da pasta vale como apelido");
        assert_eq!(p.place("/qualquer/Radiola/"), Place::Project("vitrola".into()));
        assert_eq!(p.place("/home/x/mina"), Place::Project("mina".into()), "pasta só com o nome é apelido");
        assert_eq!(p.place("/Users/x/Developer/projects/peneira"), Place::Unregistered("peneira".into()));
        assert_eq!(p.place("C:\\Users\\x\\code\\trilho"), Place::Project("trilho".into()));
    }

    #[test]
    fn throwaway_folders_are_never_a_project() {
        let p = sample();
        for cwd in ["/Users/x/Developer/projects/scratch-2026-01-01-abc123", "/private/tmp/claude-501/abc", "/tmp/x", "/var/folders/zz/T/job", "C:\\Users\\x\\AppData\\Local\\Temp\\run"] {
            assert!(matches!(p.place(cwd), Place::Ephemeral(_)), "{cwd}");
            assert_eq!(p.current(cwd), None, "{cwd}");
        }
        // Projeto registrado dentro de uma pasta temporária continua sendo o projeto.
        let d = declare("- **Grupo:** Testes\n- **Pasta:** `/tmp/fixo/app`\n").unwrap();
        let q = Projects::build([("notes/p/app.md", "App", &d)].into_iter());
        assert_eq!(q.place("/tmp/fixo/app"), Place::Project("app".into()));
        assert!(!is_ephemeral("/Users/x/Developer/projects/template"));
    }

    #[test]
    fn aliases_fold_old_names_into_the_note() {
        let p = sample();
        assert_eq!(p.canon("toca-discos"), "vitrola");
        assert_eq!(p.canon("Toca"), "vitrola");
        assert_eq!(p.canon("peneira"), "peneira", "sem nota, o nome fica como está");
        assert_eq!(p.title("toca-discos"), "Vitrola");
        assert_eq!(p.of_note("notes/projetos/trilho.md").unwrap().slug, "trilho");
    }

    #[test]
    fn a_title_prefix_names_its_project() {
        let p = sample();
        assert_eq!(p.named_in_title("trilho: índice das decisões 1 a 14").unwrap().slug, "trilho");
        assert_eq!(p.named_in_title("Vitrola: paleta final").unwrap().slug, "vitrola");
        assert!(p.named_in_title("Decidi usar o mina no fluxo").is_none(), "citar no meio não basta");
        assert!(p.named_in_title("A: fila em disco").is_none());
        assert!(p.named_in_title("trilho:").is_none());
    }

    #[test]
    fn the_template_is_read_back_as_the_same_project() {
        let t = note_template("Peneira", "Projetos", Some("~/Developer/projects/peneira"), &["tela".into()]);
        let d = declare(&t).unwrap();
        assert_eq!(d, Decl { group: "Projetos".into(), folders: vec!["~/Developer/projects/peneira".into()], aliases: vec!["tela".into()] });
    }
}
