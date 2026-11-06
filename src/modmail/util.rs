//! Fonctions utilitaires reprises de modmailbot (`src/utils.js`).

use chrono::{NaiveDateTime, Utc};
use regex::Regex;
use std::sync::LazyLock;

/// Format des dates en base modmailbot (UTC).
pub const DB_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn now_db() -> String {
    Utc::now().format(DB_TIME_FORMAT).to_string()
}

pub fn db_time_in(ms: i64) -> String {
    (Utc::now() + chrono::Duration::milliseconds(ms)).format(DB_TIME_FORMAT).to_string()
}

pub fn parse_db_time(raw: &str) -> Option<chrono::DateTime<Utc>> {
    NaiveDateTime::parse_from_str(raw, DB_TIME_FORMAT)
        .ok()
        .map(|n| n.and_utc())
        .or_else(|| chrono::DateTime::parse_from_rfc3339(raw).ok().map(|d| d.with_timezone(&Utc)))
        .or_else(|| raw.parse::<i64>().ok().and_then(chrono::DateTime::from_timestamp_millis))
}

static SNOWFLAKE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]{17,}$").expect("regex valide"));
static USER_MENTION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^<@!?([0-9]+?)>$").expect("regex valide"));
pub static DELAY_STRING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^([0-9]+)(?:([dhms])[a-z]*)?").expect("regex valide"));

pub fn is_snowflake(value: &str) -> bool {
    SNOWFLAKE.is_match(value)
}

/// Identifiant d'utilisateur à partir d'un identifiant brut ou d'une mention.
pub fn get_user_mention(value: &str) -> Option<String> {
    let value = value.trim();
    if is_snowflake(value) {
        return Some(value.to_owned());
    }
    USER_MENTION.captures(value).map(|caps| caps[1].to_owned())
}

/// Durée « 1h30m » en millisecondes ; `None` si le format est invalide.
pub fn convert_delay_string_to_ms(value: &str) -> Option<i64> {
    let mut rest = value.trim();
    let mut ms: i64 = 0;
    while !rest.is_empty() {
        let caps = DELAY_STRING.captures(rest)?;
        let amount: i64 = caps[1].parse().ok()?;
        let unit = caps.get(2).map(|m| m.as_str().to_lowercase());
        ms += match unit.as_deref() {
            Some("d") => amount * 86_400_000,
            Some("h") => amount * 3_600_000,
            Some("s") => amount * 1000,
            _ => amount * 60_000,
        };
        rest = &rest[caps[0].len()..];
    }
    Some(ms)
}

/// Équivalent de `humanize-duration` (en français, arrondi, `largest` unités).
pub fn humanize_duration(ms: i64, largest: usize, conjunction: Option<&str>) -> String {
    // (singulier, pluriel, durée en ms)
    const UNITS: [(&str, &str, i64); 7] = [
        ("an", "ans", 31_557_600_000),
        ("mois", "mois", 2_629_800_000),
        ("semaine", "semaines", 604_800_000),
        ("jour", "jours", 86_400_000),
        ("heure", "heures", 3_600_000),
        ("minute", "minutes", 60_000),
        ("seconde", "secondes", 1000),
    ];
    let mut remaining = ms.max(0) as f64;
    let mut parts: Vec<(i64, &str, &str)> = Vec::new();
    for (index, (singular, plural, unit)) in UNITS.iter().enumerate() {
        if parts.len() == largest {
            break;
        }
        let is_last_slot = parts.len() + 1 == largest || index == UNITS.len() - 1;
        let value = if is_last_slot { (remaining / *unit as f64).round() } else { (remaining / *unit as f64).floor() };
        if value >= 1.0 {
            parts.push((value as i64, singular, plural));
            remaining -= value * *unit as f64;
        } else if !parts.is_empty() {
            // humanize-duration garde des unités consécutives : une unité vide
            // compte comme une place prise.
            parts.push((0, singular, plural));
        }
    }
    let parts: Vec<String> = parts
        .into_iter()
        .filter(|(v, _, _)| *v > 0)
        .map(|(v, singular, plural)| format!("{v} {}", if v == 1 { singular } else { plural }))
        .collect();
    if parts.is_empty() {
        return "0 seconde".to_owned();
    }
    match (conjunction, parts.len()) {
        (Some(conj), n) if n > 1 => format!("{}{conj}{}", parts[..n - 1].join(", "), parts[n - 1]),
        _ => parts.join(", "),
    }
}

/// `humanizeDelay` de modmailbot : durée complète avec « et ».
pub fn humanize_delay(ms: i64) -> String {
    humanize_duration(ms, 7, Some(" et "))
}

static MARKDOWN_CHARS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([\\_*|`~])").expect("regex valide"));
static NAME_FORMATTING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([_`~*|])").expect("regex valide"));
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)(^|[^<])(https?://\S+)").expect("regex valide"));

pub fn escape_markdown(value: &str) -> String {
    MARKDOWN_CHARS.replace_all(value, "\\$1").into_owned()
}

/// Échappe le formatage dans les pseudos (`breakFormattingForNames`).
pub fn escape_name_formatting(value: &str) -> String {
    NAME_FORMATTING.replace_all(value, "\\$1").into_owned()
}

pub fn disable_inline_code(value: &str) -> String {
    value.replace('`', "'")
}

pub fn disable_code_blocks(value: &str) -> String {
    value.replace('`', "`\u{200b}")
}

pub fn disable_link_previews(value: &str) -> String {
    LINK.replace_all(value, "$1<$2>").into_owned()
}

/// Découpe une chaîne en morceaux, de préférence sur un retour à la ligne.
/// Les longueurs se comptent en unités UTF-16, comme côté Discord.
fn chunk_by_lines(text: &str, max: usize) -> Vec<String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.len() < max {
        return vec![text.to_owned()];
    }
    let mut chunks = Vec::new();
    let mut rest: &[u16] = &units;
    while !rest.is_empty() {
        if rest.len() <= max {
            chunks.push(String::from_utf16_lossy(rest));
            break;
        }
        let slice = &rest[..max];
        match slice.iter().rposition(|&c| c == u16::from(b'\n')) {
            Some(index) => {
                chunks.push(String::from_utf16_lossy(&rest[..index]));
                rest = &rest[index + 1..];
            }
            None => {
                chunks.push(String::from_utf16_lossy(slice));
                rest = &rest[max..];
            }
        }
    }
    chunks
}

/// Découpe un long message en plusieurs, en conservant les retours à la ligne
/// en bord de morceau et en refermant/rouvrant les blocs de code.
pub fn chunk_message_lines(text: &str) -> Vec<String> {
    let mut open_code_block = false;
    chunk_by_lines(text, 1990)
        .into_iter()
        .map(|mut chunk| {
            if chunk.starts_with('\n') {
                chunk.insert(0, '\u{200b}');
            }
            if chunk.ends_with('\n') {
                chunk.push('\u{200b}');
            }
            if open_code_block {
                open_code_block = false;
                if let Some(stripped) = chunk.strip_prefix("```") {
                    chunk = stripped.to_owned();
                } else {
                    chunk.insert_str(0, "```");
                }
            }
            if chunk.matches("```").count() % 2 != 0 {
                chunk.push_str("```");
                open_code_block = true;
            }
            chunk
        })
        .collect()
}

pub fn chunk_lines(lines: &[&str], size: usize) -> Vec<String> {
    lines.chunks(size).map(|chunk| chunk.join("\n")).collect()
}

/// Équivalent de `transliteration.slugify` pour les noms de salon.
pub fn slugify(value: &str) -> String {
    let ascii = deunicode::deunicode(value).to_lowercase();
    let mut slug = String::new();
    for ch in ascii.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_delays() {
        assert_eq!(convert_delay_string_to_ms("1h30m"), Some(5_400_000));
        assert_eq!(convert_delay_string_to_ms("15"), Some(900_000));
        assert_eq!(convert_delay_string_to_ms("2days"), Some(172_800_000));
        assert_eq!(convert_delay_string_to_ms("abc"), None);
    }

    #[test]
    fn humanizes_like_humanize_duration() {
        assert_eq!(humanize_duration(7 * 86_400_000, 2, None), "1 semaine");
        assert_eq!(humanize_delay(5_400_000), "1 heure et 30 minutes");
        assert_eq!(humanize_duration(90_061_000, 2, None), "1 jour, 1 heure");
        assert_eq!(humanize_delay(900_000), "15 minutes");
        assert_eq!(humanize_duration(2 * 31_557_600_000 + 2_629_800_000, 2, None), "2 ans, 1 mois");
        assert_eq!(humanize_delay(2000), "2 secondes");
    }

    #[test]
    fn resolves_user_mentions() {
        assert_eq!(get_user_mention("<@!106391128718245888>").as_deref(), Some("106391128718245888"));
        assert_eq!(get_user_mention("106391128718245888").as_deref(), Some("106391128718245888"));
        assert_eq!(get_user_mention("12345"), None);
    }

    #[test]
    fn chunks_long_messages_and_code_blocks() {
        let text = format!("```{}```", "a\n".repeat(1500));
        let chunks = chunk_message_lines(&text);
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|c| c.matches("```").count() % 2 == 0));
    }

    #[test]
    fn slugifies_usernames() {
        assert_eq!(slugify("Jöhn.Doe"), "john-doe");
        assert_eq!(slugify("★"), "");
    }
}
