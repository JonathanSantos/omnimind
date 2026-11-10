use ropey::Rope;
use std::time::{Duration, Instant};
use unicode_segmentation::UnicodeSegmentation;

const COALESCE: Duration = Duration::from_millis(900);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Left,
    Right,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    Up,
    Down,
    DocStart,
    DocEnd,
}

#[derive(Clone, Debug)]
struct Edit {
    at: usize,
    removed: String,
    inserted: String,
    before: (usize, usize),
    after: (usize, usize),
}

/// Texto editável com seleção em índices de caractere.
pub struct Buffer {
    rope: Rope,
    anchor: usize,
    head: usize,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    last_edit: Option<Instant>,
    /// Composição em curso (tecla morta, IME). Nunca entra no texto até o commit.
    pub preedit: String,
    /// Sobe a cada mudança de texto. Quem guarda cache compara por este número.
    pub version: u64,
    /// Primeira linha tocada desde o último `take_dirty`.
    dirty: Option<usize>,
    goal_col: Option<usize>,
}

impl Buffer {
    pub fn new(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            anchor: 0,
            head: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: None,
            preedit: String::new(),
            version: 0,
            dirty: Some(0),
            goal_col: None,
        }
    }

    pub fn take_dirty(&mut self) -> Option<usize> {
        self.dirty.take()
    }

    fn touch(&mut self, ch: usize) {
        let line = self.rope.char_to_line(ch.min(self.rope.len_chars()));
        self.dirty = Some(self.dirty.map_or(line, |d| d.min(line)));
        self.version += 1;
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }
    /// Linha sem o '\n' final.
    pub fn line(&self, i: usize) -> String {
        let mut s = self.rope.line(i).to_string();
        if s.ends_with('\n') {
            s.pop();
        }
        s
    }
    pub fn head(&self) -> usize {
        self.head
    }
    pub fn anchor(&self) -> usize {
        self.anchor
    }
    pub fn selection(&self) -> (usize, usize) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }
    pub fn has_selection(&self) -> bool {
        self.anchor != self.head
    }
    pub fn selected_text(&self) -> String {
        let (a, b) = self.selection();
        self.rope.slice(a..b).to_string()
    }
    pub fn line_of(&self, ch: usize) -> usize {
        self.rope.char_to_line(ch.min(self.rope.len_chars()))
    }
    pub fn line_start(&self, line: usize) -> usize {
        self.rope.line_to_char(line.min(self.rope.len_lines()))
    }
    /// (linha, byte dentro da linha) de um índice de caractere.
    pub fn line_byte(&self, ch: usize) -> (usize, usize) {
        let ch = ch.min(self.rope.len_chars());
        let line = self.rope.char_to_line(ch);
        let start = self.rope.line_to_byte(line);
        (line, self.rope.char_to_byte(ch) - start)
    }
    pub fn char_at(&self, line: usize, byte: usize) -> usize {
        let start = self.rope.line_to_byte(line.min(self.rope.len_lines().saturating_sub(1)));
        self.rope.byte_to_char((start + byte).min(self.rope.len_bytes()))
    }

    pub fn set_caret(&mut self, ch: usize, extend: bool) {
        self.head = ch.min(self.rope.len_chars());
        if !extend {
            self.anchor = self.head;
        }
        self.goal_col = None;
        self.last_edit = None;
    }

    pub fn select(&mut self, a: usize, b: usize) {
        self.anchor = a.min(self.rope.len_chars());
        self.head = b.min(self.rope.len_chars());
        self.goal_col = None;
    }

    pub fn select_all(&mut self) {
        self.select(0, self.rope.len_chars());
    }

    pub fn select_word_at(&mut self, ch: usize) {
        let (line, byte) = self.line_byte(ch);
        let text = self.line(line);
        for (i, w) in text.split_word_bound_indices() {
            if byte >= i && byte < i + w.len() {
                let a = self.char_at(line, i);
                let b = self.char_at(line, i + w.len());
                self.select(a, b);
                return;
            }
        }
    }

    fn replace(&mut self, a: usize, b: usize, text: &str, record: bool) {
        let before = (self.anchor, self.head);
        let removed = self.rope.slice(a..b).to_string();
        self.rope.remove(a..b);
        self.rope.insert(a, text);
        let end = a + text.chars().count();
        self.anchor = end;
        self.head = end;
        self.touch(a);
        self.goal_col = None;
        if !record {
            return;
        }
        self.redo.clear();
        let now = Instant::now();
        let recent = self.last_edit.is_some_and(|t| now - t < COALESCE);
        self.last_edit = Some(now);
        if recent {
            if let Some(last) = self.undo.last_mut() {
                let last_end = last.at + last.inserted.chars().count();
                let plain_typing = removed.is_empty() && a == last_end && !text.contains('\n');
                let breaks_word = text.ends_with(' ') && !last.inserted.ends_with(' ');
                if plain_typing && !breaks_word {
                    last.inserted.push_str(text);
                    last.after = (end, end);
                    return;
                }
                let backspacing = text.is_empty() && last.inserted.is_empty() && b == last.at;
                if backspacing {
                    last.removed.insert_str(0, &removed);
                    last.at = a;
                    last.after = (end, end);
                    return;
                }
            }
        }
        self.undo.push(Edit { at: a, removed, inserted: text.into(), before, after: (end, end) });
    }

    pub fn insert(&mut self, text: &str) {
        let (a, b) = self.selection();
        self.replace(a, b, text, true);
    }

    pub fn backspace(&mut self) {
        if self.has_selection() {
            self.insert("");
            return;
        }
        if self.head == 0 {
            return;
        }
        let a = self.prev_boundary(self.head);
        self.replace(a, self.head, "", true);
    }

    pub fn delete_word_back(&mut self) {
        if self.has_selection() {
            self.insert("");
            return;
        }
        let a = self.word_left(self.head);
        self.replace(a, self.head, "", true);
    }

    pub fn delete_forward(&mut self) {
        if self.has_selection() {
            self.insert("");
            return;
        }
        if self.head >= self.rope.len_chars() {
            return;
        }
        let b = self.next_boundary(self.head);
        self.replace(self.head, b, "", true);
    }

    /// Enter com continuação de lista e citação.
    pub fn newline(&mut self) {
        let (line, _) = self.line_byte(self.head);
        let text = self.line(line);
        let trimmed = text.trim_start();
        let indent = &text[..text.len() - trimmed.len()];
        match list_marker(trimmed) {
            Some((_, rest)) if rest.trim().is_empty() => {
                // Marcador vazio: Enter encerra a lista.
                let a = self.line_start(line);
                self.select(a, a + text.chars().count());
                self.insert("");
            }
            Some((marker, _)) => {
                let next = next_marker(&marker);
                let indent = indent.to_string();
                self.insert(&format!("\n{indent}{next}"));
            }
            None => self.insert("\n"),
        }
    }

    fn prev_boundary(&self, ch: usize) -> usize {
        let (line, byte) = self.line_byte(ch);
        if byte == 0 {
            return ch - 1;
        }
        let text = self.line(line);
        let mut prev = 0;
        for (i, _) in text.grapheme_indices(true) {
            if i >= byte {
                break;
            }
            prev = i;
        }
        self.char_at(line, prev)
    }

    fn next_boundary(&self, ch: usize) -> usize {
        let (line, byte) = self.line_byte(ch);
        let text = self.line(line);
        if byte >= text.len() {
            return (ch + 1).min(self.rope.len_chars());
        }
        for (i, g) in text.grapheme_indices(true) {
            if i + g.len() > byte {
                return self.char_at(line, i + g.len());
            }
        }
        ch
    }

    fn word_left(&self, ch: usize) -> usize {
        let (line, byte) = self.line_byte(ch);
        if byte == 0 {
            return ch.saturating_sub(1);
        }
        let text = self.line(line);
        let mut at = 0;
        for (i, w) in text.split_word_bound_indices() {
            if i >= byte {
                break;
            }
            if !w.trim().is_empty() {
                at = i;
            }
        }
        self.char_at(line, at)
    }

    fn word_right(&self, ch: usize) -> usize {
        let (line, byte) = self.line_byte(ch);
        let text = self.line(line);
        if byte >= text.len() {
            return (ch + 1).min(self.rope.len_chars());
        }
        for (i, w) in text.split_word_bound_indices() {
            let end = i + w.len();
            if end > byte && !w.trim().is_empty() {
                return self.char_at(line, end);
            }
        }
        self.char_at(line, text.len())
    }

    pub fn motion(&mut self, m: Move, extend: bool) {
        // Sem Shift, seta sobre seleção colapsa para a ponta, como todo editor do macOS.
        if !extend && self.has_selection() {
            let (a, b) = self.selection();
            match m {
                Move::Left | Move::Up => return self.set_caret(a, false),
                Move::Right | Move::Down => return self.set_caret(b, false),
                _ => {}
            }
        }
        let h = self.head;
        let (line, _) = self.line_byte(h);
        let target = match m {
            Move::Left if h > 0 => self.prev_boundary(h),
            Move::Left => 0,
            Move::Right => self.next_boundary(h),
            Move::WordLeft => self.word_left(h),
            Move::WordRight => self.word_right(h),
            Move::LineStart => self.line_start(line),
            Move::LineEnd => self.line_start(line) + self.line(line).chars().count(),
            Move::DocStart => 0,
            Move::DocEnd => self.rope.len_chars(),
            Move::Up | Move::Down => {
                let col = self.goal_col.unwrap_or(h - self.line_start(line));
                let up = m == Move::Up;
                if (up && line == 0) || (!up && line + 1 >= self.rope.len_lines()) {
                    if up { 0 } else { self.rope.len_chars() }
                } else {
                    let next = if up { line - 1 } else { line + 1 };
                    let t = self.line_start(next) + col.min(self.line(next).chars().count());
                    self.head = t;
                    if !extend {
                        self.anchor = t;
                    }
                    self.goal_col = Some(col);
                    self.last_edit = None;
                    return;
                }
            }
        };
        self.set_caret(target, extend);
    }

    pub fn undo(&mut self) {
        let Some(e) = self.undo.pop() else { return };
        let end = e.at + e.inserted.chars().count();
        self.rope.remove(e.at..end);
        self.rope.insert(e.at, &e.removed);
        (self.anchor, self.head) = e.before;
        self.touch(e.at);
        self.last_edit = None;
        self.redo.push(e);
    }

    pub fn redo(&mut self) {
        let Some(e) = self.redo.pop() else { return };
        let end = e.at + e.removed.chars().count();
        self.rope.remove(e.at..end);
        self.rope.insert(e.at, &e.inserted);
        (self.anchor, self.head) = e.after;
        self.touch(e.at);
        self.last_edit = None;
        self.undo.push(e);
    }

    /// Troca o texto vindo de fora, mantendo o cursor por linha e coluna.
    pub fn reload(&mut self, text: &str) {
        let (line, byte) = self.line_byte(self.head);
        self.rope = Rope::from_str(text);
        self.undo.clear();
        self.redo.clear();
        self.touch(0);
        let line = line.min(self.rope.len_lines().saturating_sub(1));
        let max = self.line(line).len();
        let mut byte = byte.min(max);
        let l = self.line(line);
        while !l.is_char_boundary(byte) {
            byte -= 1;
        }
        let ch = self.char_at(line, byte);
        self.set_caret(ch, false);
    }
}

fn list_marker(t: &str) -> Option<(String, &str)> {
    for m in ["- [ ] ", "- [x] ", "- ", "* ", "+ ", "> "] {
        if let Some(r) = t.strip_prefix(m) {
            return Some((m.to_string(), r));
        }
    }
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 && t[digits..].starts_with(". ") {
        return Some((t[..digits + 2].to_string(), &t[digits + 2..]));
    }
    None
}

fn next_marker(m: &str) -> String {
    if m == "- [x] " {
        return "- [ ] ".into();
    }
    if let Some(n) = m.strip_suffix(". ").and_then(|d| d.parse::<u64>().ok()) {
        return format!("{}. ", n + 1);
    }
    m.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % n as u64) as usize
        }
    }

    /// Oráculo: mil sementes de edições aleatórias contra uma String.
    #[test]
    fn random_edits_match_string_oracle_and_undo_restores_everything() {
        let alphabet = ["a", "é", "ã", " ", "\n", "ç", "🙂", "- ", "e\u{301}"];
        for seed in 1..=1000u64 {
            let mut r = Rng(seed * 2654435761 + 1);
            let start = "olá mundo\nsegunda linha\n";
            let mut b = Buffer::new(start);
            let mut o: Vec<char> = start.chars().collect();
            let steps = 60;
            for _ in 0..steps {
                b.last_edit = None;
                match r.next(4) {
                    0 | 1 => {
                        let at = r.next(o.len() + 1);
                        b.set_caret(at, false);
                        let s = alphabet[r.next(alphabet.len())];
                        b.insert(s);
                        for (i, c) in s.chars().enumerate() {
                            o.insert(at + i, c);
                        }
                    }
                    2 => {
                        let (x, y) = (r.next(o.len() + 1), r.next(o.len() + 1));
                        let (x, y) = (x.min(y), x.max(y));
                        b.select(x, y);
                        b.insert("");
                        o.drain(x..y);
                    }
                    _ => {
                        b.motion(Move::WordRight, false);
                        b.motion(Move::Down, r.next(2) == 0);
                        b.set_caret(b.head(), false);
                    }
                }
                assert_eq!(b.text(), o.iter().collect::<String>(), "seed {seed}");
            }
            let end = b.text();
            while !b.undo.is_empty() {
                b.undo();
            }
            assert_eq!(b.text(), start, "undo all, seed {seed}");
            while !b.redo.is_empty() {
                b.redo();
            }
            assert_eq!(b.text(), end, "redo all, seed {seed}");
        }
    }

    #[test]
    fn backspace_removes_whole_grapheme() {
        let mut b = Buffer::new("e\u{301}🙂");
        b.set_caret(3, false);
        b.backspace();
        assert_eq!(b.text(), "e\u{301}");
        b.backspace();
        assert_eq!(b.text(), "");
    }

    #[test]
    fn enter_continues_and_ends_lists() {
        let mut b = Buffer::new("- item");
        b.set_caret(6, false);
        b.newline();
        assert_eq!(b.text(), "- item\n- ");
        b.newline();
        assert_eq!(b.text(), "- item\n");
        let mut n = Buffer::new("9. nove");
        n.set_caret(7, false);
        n.newline();
        assert_eq!(n.text(), "9. nove\n10. ");
    }

    #[test]
    fn typing_coalesces_into_one_undo_step_per_word() {
        let mut b = Buffer::new("");
        for c in ["o", "l", "á"] {
            b.insert(c);
        }
        b.insert(" ");
        b.undo();
        assert_eq!(b.text(), "olá");
        b.undo();
        assert_eq!(b.text(), "");
    }

    #[test]
    fn reload_keeps_caret_on_char_boundary() {
        let mut b = Buffer::new("ação\nabc");
        b.set_caret(4, false);
        b.reload("ação nova\nabc");
        assert_eq!(b.head(), 4);
        b.reload("á");
        assert_eq!(b.head(), 1);
    }
}
