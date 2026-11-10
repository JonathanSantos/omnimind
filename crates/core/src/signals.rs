//! Classificador de regras PT/EN sobre a mensagem da pessoa. Sem LLM; sinal é literal e barato de errar.

use crate::analyzer::fold;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    Correction,
    Preference,
    Request,
}

impl SignalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SignalKind::Correction => "correction",
            SignalKind::Preference => "preference",
            SignalKind::Request => "request",
        }
    }
}

const START_CORRECTION: &[&str] = &[
    "nao,",
    "nao.",
    "nao ",
    "errado",
    "ta errado",
    "esta errado",
    "na verdade",
    "eu disse",
    "eu falei",
    "pare de",
    "para de",
    "de novo",
    "outra vez",
    "corrigindo",
    "correcao:",
    "nao era",
    "nao e isso",
    "no,",
    "no.",
    "wrong",
    "actually",
    "stop ",
    "i said",
    "i told you",
    "that's not",
    "thats not",
    "not that",
    "again,",
    "don't",
    "dont ",
    "do not",
];

/// Mais fortes: valem no começo de qualquer oração, não só da mensagem.
const CLAUSE_CORRECTION: &[&str] = &["para de", "pare de"];

const ANY_CORRECTION: &[&str] = &[
    "ta errado",
    "esta errado",
    "isso esta errado",
    "voce errou",
    "nao foi isso",
    "nao era isso",
    "ja te disse",
    "ja disse",
    "ja falei",
    "de novo nao",
    "em vez de",
    "ao inves de",
    "instead of",
    "you got it wrong",
    "that's wrong",
    "thats wrong",
    "i already told",
];

const START_PREFERENCE: &[&str] = &["prefiro", "gosto de", "nada de", "evite", "i prefer", "i like", "i'd rather"];
const ANY_PREFERENCE: &[&str] = &[
    "eu prefiro",
    "prefiro ",
    "gosto de ",
    "gosto que ",
    "sempre use",
    "sempre faca",
    "nunca use",
    "nunca faca",
    "sempre ",
    "nunca ",
    "always ",
    "never ",
    "i prefer",
    "por padrao",
    "by default",
];

const START_REQUEST: &[&str] = &[
    "lembre",
    "lembra",
    "anote",
    "anota",
    "guarde",
    "guarda",
    "grave",
    "grava",
    "registre",
    "registra",
    "memorize",
    "remember",
    "note that",
    "keep in mind",
    "nao esqueca",
    "don't forget",
];

/// A pessoa dizendo quem é. Vale no começo de qualquer oração.
const IDENTITY: &[&str] = &[
    "me chamo",
    "meu nome e",
    "pode me chamar",
    "sou a ",
    "sou o ",
    "sou responsavel",
    "my name is",
    "call me",
    "i am the",
    "i'm the",
];

/// Palavras de ligação que não mudam o que a oração pede.
const FILLERS: &[&str] = &[
    "e ", "mas ", "entao ", "ah ", "por favor ", "pode ", "and ", "but ", "please ",
];

fn clause_starts(folded: &str, list: &[&str]) -> bool {
    folded
        .split(['.', ',', ';', ':', '!', '?', '\n'])
        .any(|clause| {
            let mut c = clause.trim();
            while let Some(rest) = FILLERS.iter().find_map(|f| c.strip_prefix(f)) {
                c = rest.trim_start();
            }
            list.iter().any(|p| c.starts_with(p))
        })
}

/// A frase da pessoa diz quem ela é?
pub fn names_the_person(text: &str) -> bool {
    clause_starts(&fold(text), IDENTITY)
}

/// A frase em que a pessoa diz quem é, sem o resto da mensagem.
pub fn person_sentence(text: &str) -> Option<&str> {
    text.split(['.', '!', '?', '\n'])
        .map(str::trim)
        .find(|s| names_the_person(s))
}

/// Tira colagens e blocos de sistema antes de classificar.
pub fn strip_pasted(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(a) = rest.find("<pasted_content>") {
        out.push_str(&rest[..a]);
        match rest[a..].find("</pasted_content>") {
            Some(b) => rest = &rest[a + b + "</pasted_content>".len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Classifica a mensagem. Texto longo, ou de sistema, não é correção direta.
pub fn classify(text: &str) -> Option<SignalKind> {
    let clean = strip_pasted(text);
    let t = clean.trim();
    if t.is_empty() || t.chars().count() > 600 {
        return None;
    }
    if t.starts_with('<') || t.starts_with('/') {
        return None;
    }
    let f = fold(t);
    let padded = format!(" {f}");
    if START_CORRECTION.iter().any(|p| f.starts_with(p))
        || ANY_CORRECTION.iter().any(|p| f.contains(p))
        || clause_starts(&f, CLAUSE_CORRECTION)
    {
        return Some(SignalKind::Correction);
    }
    if START_PREFERENCE.iter().any(|p| f.starts_with(p))
        || ANY_PREFERENCE
            .iter()
            .any(|p| padded.contains(&format!(" {p}")))
    {
        return Some(SignalKind::Preference);
    }
    if clause_starts(&f, START_REQUEST) || clause_starts(&f, IDENTITY) {
        return Some(SignalKind::Request);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_pt_and_en() {
        for (t, k) in [
            ("não, faz commits menores", SignalKind::Correction),
            ("Na verdade o deploy é na terça", SignalKind::Correction),
            ("pare de usar tabs", SignalKind::Correction),
            ("no, use spaces", SignalKind::Correction),
            ("actually it runs at 3am", SignalKind::Correction),
            ("use pnpm em vez de npm", SignalKind::Correction),
            ("prefiro respostas curtas", SignalKind::Preference),
            ("sempre rode os testes", SignalKind::Preference),
            ("always run the linter", SignalKind::Preference),
            ("lembre que o cliente é a ACME", SignalKind::Request),
            ("remember the staging URL", SignalKind::Request),
            ("boa. por favor, para de me mandar lista gigante", SignalKind::Correction),
            ("tudo certo, gosto de resposta curta", SignalKind::Preference),
            ("boa. anota aí que a Nina cobre o painel", SignalKind::Request),
            ("decidi ontem, guarda isso", SignalKind::Request),
            ("nada de gradiente nem neon", SignalKind::Preference),
            ("evite sombra larga", SignalKind::Preference),
            ("oi, me chamo Bia", SignalKind::Request),
            ("my name is Ana", SignalKind::Request),
        ] {
            assert_eq!(classify(t), Some(k), "{t}");
        }
    }

    #[test]
    fn finds_the_sentence_that_says_who_the_person_is() {
        assert_eq!(
            person_sentence("oi, me chamo Bia e cuido da API. prefiro respostas curtas."),
            Some("oi, me chamo Bia e cuido da API")
        );
        assert_eq!(person_sentence("prefiro respostas curtas."), None);
    }

    #[test]
    fn ignores_plain_requests_and_pastes() {
        for t in [
            "adiciona um botão de salvar",
            "qual é o status do deploy?",
            "quero ir para de manhã",
            "o bug é grave, tem como ver?",
            "<pasted_content>não, isso está errado</pasted_content>",
            "<system-reminder>nunca faça isso</system-reminder>",
            "/compact",
            "",
        ] {
            assert_eq!(classify(t), None, "{t}");
        }
    }
}
