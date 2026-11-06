//! Dates en base et fuseau horaire.
//!
//! Le conteneur de prod tourne en UTC : sans conversion explicite, une saisie
//! « 21:00 » serait enregistrée à 21:00 UTC (23:00 heure française). Toutes les
//! saisies sont donc interprétées en Europe/Paris, heure d'été comprise.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::{Europe::Paris, Tz};
use sqlx::{
    Decode, Encode, Sqlite, Type, TypeInfo, ValueRef,
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
};
use std::sync::LazyLock;

pub const TIMEZONE: Tz = Paris;

/// Instant UTC stocké en texte ISO 8601 (`2026-08-10T21:00:00.000Z`), le format
/// de l'ancienne version Prisma. Ce format se trie comme une chaîne : les
/// comparaisons SQL restent de simples comparaisons de texte, indexables.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DbTime(pub DateTime<Utc>);

impl DbTime {
    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn unix(self) -> i64 {
        self.0.timestamp()
    }

    pub fn to_db_string(self) -> String {
        self.0.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    }

    fn parse(raw: &str) -> Option<Self> {
        if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
            return Some(Self(parsed.with_timezone(&Utc)));
        }
        // Valeurs par défaut SQLite (CURRENT_TIMESTAMP) : « 2026-08-10 21:00:00 ».
        ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S%.f"]
            .iter()
            .find_map(|format| NaiveDateTime::parse_from_str(raw, format).ok())
            .map(|naive| Self(naive.and_utc()))
    }
}

impl From<DateTime<Utc>> for DbTime {
    fn from(value: DateTime<Utc>) -> Self {
        Self(value)
    }
}

impl Type<Sqlite> for DbTime {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        matches!(ty.name(), "TEXT" | "DATETIME" | "INTEGER" | "NULL")
    }
}

impl Encode<'_, Sqlite> for DbTime {
    fn encode_by_ref(&self, buf: &mut Vec<SqliteArgumentValue<'_>>) -> Result<IsNull, BoxDynError> {
        <String as Encode<Sqlite>>::encode(self.to_db_string(), buf)
    }
}

impl<'r> Decode<'r, Sqlite> for DbTime {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        if value.type_info().name() == "INTEGER" {
            let millis = <i64 as Decode<Sqlite>>::decode(value)?;
            return DateTime::from_timestamp_millis(millis)
                .map(Self)
                .ok_or_else(|| format!("timestamp invalide : {millis}").into());
        }
        let raw = <&str as Decode<Sqlite>>::decode(value)?;
        Self::parse(raw).ok_or_else(|| format!("date invalide en base : {raw}").into())
    }
}

static DATE_PATTERN: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(\d{1,2})/(\d{1,2})/(\d{4})$").expect("regex valide"));
static TIME_PATTERN: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(\d{1,2})[:hH](\d{2})$").expect("regex valide"));

/// Date du jour en heure de Paris, au format JJ/MM/AAAA.
pub fn today_label() -> String {
    Utc::now().with_timezone(&TIMEZONE).format("%d/%m/%Y").to_string()
}

/// Date lisible en heure de Paris, là où les timestamps Discord ne sont pas
/// rendus (libellés d'autocomplétion, titres).
pub fn format_date_label(date: DbTime) -> String {
    date.0.with_timezone(&TIMEZONE).format("%d/%m/%Y %H:%M").to_string()
}

/// Jour seul (JJ/MM/AAAA) en heure de Paris.
pub fn format_day_label(date: DbTime) -> String {
    date.0.with_timezone(&TIMEZONE).format("%d/%m/%Y").to_string()
}

/// Convertit une heure locale de Paris en instant UTC. Lors du passage à
/// l'heure d'été, une heure inexistante (02:30) est décalée d'une heure ; lors
/// du retour à l'heure d'hiver, la première occurrence est retenue.
fn paris_to_utc(naive: NaiveDateTime) -> Option<DateTime<Utc>> {
    TIMEZONE
        .from_local_datetime(&naive)
        .earliest()
        .or_else(|| TIMEZONE.from_local_datetime(&(naive + Duration::hours(1))).earliest())
        .map(|date| date.with_timezone(&Utc))
}

/// Convertit une saisie JJ/MM/AAAA + HH:MM (heure de Paris) en instant UTC.
/// Renvoie `None` si le format ou la date sont invalides (ex : 31/02).
pub fn parse_event_date(date_label: &str, time_label: &str) -> Option<DbTime> {
    let date = DATE_PATTERN.captures(date_label.trim())?;
    let time = TIME_PATTERN.captures(time_label.trim())?;
    let day: u32 = date[1].parse().ok()?;
    let month: u32 = date[2].parse().ok()?;
    let year: i32 = date[3].parse().ok()?;
    let hours: u32 = time[1].parse().ok()?;
    let minutes: u32 = time[2].parse().ok()?;

    let date = NaiveDate::from_ymd_opt(year, month, day)?;
    let time = NaiveTime::from_hms_opt(hours, minutes, 0)?;
    paris_to_utc(date.and_time(time)).map(DbTime)
}

pub struct ResolvedDate {
    pub date: DbTime,
    pub rolled_to_tomorrow: bool,
}

/// Résout la date d'un événement. Sans date explicite on prend aujourd'hui, et
/// si l'heure est déjà passée on bascule sur demain.
pub fn resolve_event_date(date_label: Option<&str>, time_label: &str) -> Option<ResolvedDate> {
    if let Some(label) = date_label {
        return parse_event_date(label, time_label).map(|date| ResolvedDate { date, rolled_to_tomorrow: false });
    }

    let today = parse_event_date(&today_label(), time_label)?;
    if today.0 > Utc::now() {
        return Some(ResolvedDate { date: today, rolled_to_tomorrow: false });
    }

    let tomorrow = (Utc::now().with_timezone(&TIMEZONE) + Duration::days(1)).format("%d/%m/%Y").to_string();
    parse_event_date(&tomorrow, time_label).map(|date| ResolvedDate { date, rolled_to_tomorrow: true })
}

/// Minuit heure de Paris du jour `day`, exprimé en UTC.
pub fn paris_day_start(day: NaiveDate) -> DbTime {
    DbTime(paris_to_utc(day.and_time(NaiveTime::MIN)).expect("minuit existe toujours à Paris"))
}

/// Jour saisi dans un `<input type="date">` (`AAAA-MM-JJ`).
pub fn parse_iso_day(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").ok()
}

/// Lundi 00:00 heure de Paris de la semaine de `reference`, exprimé en UTC.
/// Sans conversion, un dimanche 23:30 à Paris serait rattaché à la semaine suivante.
pub fn week_start(reference: DateTime<Utc>) -> DbTime {
    let local = reference.with_timezone(&TIMEZONE).date_naive();
    paris_day_start(local - Duration::days(i64::from(local.weekday().num_days_from_monday())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_paris_time_with_dst() {
        // Été : UTC+2.
        let summer = parse_event_date("15/07/2026", "21:00").unwrap();
        assert_eq!(summer.to_db_string(), "2026-07-15T19:00:00.000Z");
        // Hiver : UTC+1.
        let winter = parse_event_date("15/01/2026", "21h00").unwrap();
        assert_eq!(winter.to_db_string(), "2026-01-15T20:00:00.000Z");
    }

    #[test]
    fn rejects_overflowing_dates() {
        assert!(parse_event_date("31/02/2026", "21:00").is_none());
        assert!(parse_event_date("10/13/2026", "21:00").is_none());
        assert!(parse_event_date("10/10/2026", "24:00").is_none());
        assert!(parse_event_date("10-10-2026", "21:00").is_none());
    }

    #[test]
    fn week_starts_on_paris_monday() {
        // Dimanche 23:30 à Paris (21:30 UTC en été) : encore la semaine en cours.
        let sunday_night = Utc.with_ymd_and_hms(2026, 8, 16, 21, 30, 0).unwrap();
        assert_eq!(week_start(sunday_night).to_db_string(), "2026-08-09T22:00:00.000Z");
        // Lundi 00:30 à Paris : nouvelle semaine.
        let monday = Utc.with_ymd_and_hms(2026, 8, 16, 22, 30, 0).unwrap();
        assert_eq!(week_start(monday).to_db_string(), "2026-08-16T22:00:00.000Z");
    }

    #[test]
    fn parses_iso_days_as_paris_day_start() {
        let day = parse_iso_day("2026-10-25").unwrap();
        // Jour du passage à l'heure d'hiver : minuit est encore en UTC+2.
        assert_eq!(paris_day_start(day).to_db_string(), "2026-10-24T22:00:00.000Z");
        assert_eq!(paris_day_start(day.succ_opt().unwrap()).to_db_string(), "2026-10-25T23:00:00.000Z");
        assert!(parse_iso_day("2026-02-30").is_none());
        assert!(parse_iso_day("25/10/2026").is_none());
    }

    #[test]
    fn parses_legacy_formats() {
        assert!(DbTime::parse("2026-08-10T02:04:20.123Z").is_some());
        assert!(DbTime::parse("2026-08-10 02:04:20").is_some());
    }
}
