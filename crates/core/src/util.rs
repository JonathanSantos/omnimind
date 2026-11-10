//! Tempo, ids, hashes e escrita atômica.

use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use std::collections::hash_map::RandomState;
use std::fs::{self, File, OpenOptions};
use std::hash::{BuildHasher, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_micros() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2026-10-31T19:20:00.123456Z`
pub fn format_ts(micros: i64) -> String {
    let secs = micros.div_euclid(1_000_000);
    let frac = micros.rem_euclid(1_000_000);
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{frac:06}Z",
        tod / 3600,
        (tod / 60) % 60,
        tod % 60
    )
}

/// `2026-10-31T19:20:00Z`, sem fração, para o frontmatter.
pub fn format_ts_seconds(micros: i64) -> String {
    let mut s = format_ts(micros);
    s.truncate(19);
    s.push('Z');
    s
}

pub fn now_ts() -> String {
    format_ts(now_micros())
}

pub fn now_ts_seconds() -> String {
    format_ts_seconds(now_micros())
}

pub fn today() -> String {
    let mut s = now_ts();
    s.truncate(10);
    s
}

/// Aceita `YYYY-MM-DD` e `YYYY-MM-DDTHH:MM:SS[.frac]Z`. Devolve micros desde a época.
pub fn parse_ts(s: &str) -> Option<i64> {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y: i64 = s.get(0..4)?.parse().ok()?;
    let m: u32 = s.get(5..7)?.parse().ok()?;
    let d: u32 = s.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let mut micros = days_from_civil(y, m, d) * 86_400_000_000;
    if b.len() >= 19 && (b[10] == b'T' || b[10] == b' ') {
        let hh: i64 = s.get(11..13)?.parse().ok()?;
        let mm: i64 = s.get(14..16)?.parse().ok()?;
        let ss: i64 = s.get(17..19)?.parse().ok()?;
        micros += (hh * 3600 + mm * 60 + ss) * 1_000_000;
        if let Some(rest) = s.get(19..) {
            if let Some(fr) = rest.strip_prefix('.') {
                let digits: String = fr.chars().take_while(char::is_ascii_digit).collect();
                let mut padded = digits.clone();
                padded.truncate(6);
                while padded.len() < 6 {
                    padded.push('0');
                }
                micros += padded.parse::<i64>().unwrap_or(0);
            }
        }
    }
    Some(micros)
}

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
static ULID_LAST: Mutex<(u64, u128)> = Mutex::new((0, 0));
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn random_u64() -> u64 {
    let mut h = RandomState::new().build_hasher();
    h.write_u64(now_micros() as u64);
    h.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    h.write_u32(std::process::id());
    h.finish()
}

/// ULID monotônico dentro do processo.
pub fn new_ulid() -> String {
    let ms = (now_micros() / 1000) as u64;
    let mut guard = ULID_LAST.lock().unwrap_or_else(|p| p.into_inner());
    let rand = if guard.0 == ms {
        guard.1 + 1
    } else {
        (u128::from(random_u64()) << 16 | u128::from(random_u64() & 0xffff)) & ((1u128 << 80) - 1)
    };
    *guard = (ms, rand);
    drop(guard);
    let v = (u128::from(ms) << 80) | rand;
    let mut out = [0u8; 26];
    for (i, slot) in out.iter_mut().enumerate() {
        let shift = 125 - i * 5;
        *slot = CROCKFORD[((v >> shift) & 31) as usize];
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn is_ulid(s: &str) -> bool {
    s.len() == 26 && s.bytes().all(|b| CROCKFORD.contains(&b))
}

/// 16 hex do SHA-256.
pub fn hash_bytes(data: &[u8]) -> String {
    let d = Sha256::digest(data);
    let mut s = String::with_capacity(16);
    for b in &d[..8] {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn hash_str(s: &str) -> String {
    hash_bytes(s.as_bytes())
}

/// Hasher rápido para mapas internos. Sem resistência a colisão provocada: dados são locais.
#[derive(Default, Clone, Copy)]
pub struct FastHasher(u64);

impl Hasher for FastHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut h = self.0;
        let (chunks, rest) = bytes.as_chunks::<8>();
        for c in chunks {
            let v = u64::from_le_bytes(*c);
            h = (h.rotate_left(5) ^ v).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
        for b in rest {
            h = (h.rotate_left(5) ^ u64::from(*b)).wrapping_mul(0x517c_c1b7_2722_0a95);
        }
        self.0 = h;
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

pub type FastBuild = std::hash::BuildHasherDefault<FastHasher>;
pub type FastMap<K, V> = std::collections::HashMap<K, V, FastBuild>;

pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in data {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Checksum rápido do instantâneo (não criptográfico).
pub fn fast_checksum(data: &[u8]) -> u64 {
    let mut h: u64 = 0x9e37_79b9_7f4a_7c15 ^ (data.len() as u64);
    let (chunks, rest) = data.as_chunks::<8>();
    for c in chunks {
        let v = u64::from_le_bytes(*c);
        h = (h.rotate_left(5) ^ v).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    for b in rest {
        h = (h.rotate_left(5) ^ u64::from(*b)).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    h
}

/// Costura de teste: pausa entre gravar o temporário e o `rename`, para o teste de `kill -9`.
fn test_pause() {
    if let Ok(v) = std::env::var("OMNIMIND_TEST_PAUSE_MS") {
        if let Ok(ms) = v.parse::<u64>() {
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
    }
}

/// Grava `data` em temporário no diretório `tmp_dir` (mesmo volume), `sync_all`, `rename`, `fsync` do pai.
pub fn atomic_write(tmp_dir: &Path, dest: &Path, data: &[u8], mode: Option<u32>) -> Result<()> {
    let tmp = tmp_path(tmp_dir);
    let res = (|| -> Result<()> {
        let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        f.write_all(data)?;
        #[cfg(unix)]
        if let Some(m) = mode {
            use std::os::unix::fs::PermissionsExt;
            f.set_permissions(fs::Permissions::from_mode(m))?;
        }
        #[cfg(not(unix))]
        let _ = mode;
        f.sync_all()?;
        drop(f);
        test_pause();
        fs::rename(&tmp, dest)?;
        if let Some(parent) = dest.parent() {
            if let Ok(d) = File::open(parent) {
                let _ = d.sync_all();
            }
        }
        Ok(())
    })();
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res
}

pub fn tmp_path(tmp_dir: &Path) -> PathBuf {
    tmp_dir.join(format!("{}.{}.tmp", std::process::id(), new_ulid()))
}

/// Corta em fronteira de caractere, no máximo `max` bytes.
pub fn truncate_chars(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Slug ASCII para nome de arquivo.
/// Id como o modelo o cita: `[mem:ID]` e `mem:ID` valem o mesmo que `ID`.
pub fn plain_id(s: &str) -> &str {
    let t = s.trim().trim_matches(['[', ']']);
    t.strip_prefix("mem:").unwrap_or(t)
}

pub fn slug(s: &str) -> String {
    let folded = crate::analyzer::fold(s);
    let mut out = String::new();
    let mut dash = false;
    for c in folded.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.len() > 80 {
        out.truncate(80);
        while out.ends_with('-') {
            out.pop();
        }
    }
    if out.is_empty() {
        out.push_str("memory");
    }
    out
}

pub fn invalid<T>(msg: impl Into<String>) -> Result<T> {
    Err(Error::Invalid(msg.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ts_round_trip() {
        let t = 1_793_000_000_123_456;
        assert_eq!(parse_ts(&format_ts(t)), Some(t));
        assert_eq!(parse_ts("2026-10-31"), parse_ts("2026-10-31T00:00:00Z"));
        assert_eq!(parse_ts("bogus"), None);
        assert_eq!(parse_ts("2026-13-01"), None);
    }

    #[test]
    fn ulids_are_valid_and_ordered() {
        let a = new_ulid();
        let b = new_ulid();
        assert!(is_ulid(&a));
        assert!(a < b);
    }

    #[test]
    fn citation_forms_are_the_same_id() {
        assert_eq!(plain_id("[mem:01ABC]"), "01ABC");
        assert_eq!(plain_id(" mem:01ABC "), "01ABC");
        assert_eq!(plain_id("note:notes/a.md"), "note:notes/a.md");
    }

    #[test]
    fn slug_is_ascii() {
        assert_eq!(slug("Revisão semanal (30 min)"), "revisao-semanal-30-min");
        assert_eq!(slug("../../etc"), "etc");
        assert_eq!(slug("###"), "memory");
    }

    #[test]
    fn truncate_respects_boundaries() {
        assert_eq!(truncate_chars("ação", 3), "a\u{e7}");
    }
}

/// Pasta pessoal. `HOME` vale em todo lugar, o que mantém os testes isolados;
/// no Windows ela quase nunca existe, e a resposta é `USERPROFILE`.
pub fn home_dir() -> Option<std::path::PathBuf> {
    let set = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty());
    set("HOME").or_else(|| if cfg!(windows) { set("USERPROFILE") } else { None }).map(std::path::PathBuf::from)
}
