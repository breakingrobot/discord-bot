//! Matchs du Real Madrid lus dans l'API publique d'ESPN (sans clé) :
//! `GET site.api.espn.com/apis/site/v2/sports/soccer/<compétition>/scoreboard?dates=AAAAMMJJ`.
//! Score, minute, statut et buts (buteur, minute, penalty, contre son camp).
//! Lecture tolérante : un champ absent ou renommé ne fait jamais échouer le bot.

use anyhow::{Context as _, Result};
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;
use std::time::Duration;

/// Hôtes de l'API : le second sert de secours si le premier refuse (403, panne).
const HOSTS: [&str; 2] = ["https://site.api.espn.com", "https://site.web.api.espn.com"];
const API_PATH: &str = "/apis/site/v2/sports/soccer";
/// ESPN refuse (403) les clients qui ne ressemblent pas à un navigateur.
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";
/// Compétitions où le Real peut jouer : Liga, Ligue des champions, Coupe et Supercoupe d'Espagne, amicaux.
const LEAGUES: [&str; 6] =
    ["esp.1", "uefa.champions", "esp.copa_del_rey", "esp.super_cup", "fifa.cwc", "club.friendly"];
/// Identifiant du Real Madrid chez ESPN.
const REAL_MADRID: &str = "86";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Pas commencé.
    Pre,
    /// En cours (mi-temps comprise).
    Live,
    /// Terminé.
    Post,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Goal {
    /// Minute affichée, ex : « 67' » ou « 90'+2' ».
    pub minute: String,
    pub scorer: Option<String>,
    /// But compté pour le Real (contre son camp adverse compris).
    pub madrid: bool,
    pub penalty: bool,
    pub own_goal: bool,
    /// Photo du buteur.
    pub headshot: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedCard {
    pub minute: String,
    pub player: String,
    pub madrid: bool,
}

impl RedCard {
    /// Clé stable, pour ne l'annoncer qu'une fois.
    pub fn key(&self) -> String {
        format!("rouge:{}:{}", self.minute, crate::modules::halamadrid::service::fold(&self.player))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    /// Compétition ESPN (ex : `esp.1`), pour lire compos et statistiques.
    pub league: String,
    pub kickoff: DateTime<Utc>,
    pub opponent: String,
    pub madrid_logo: Option<String>,
    pub opponent_logo: Option<String>,
    pub madrid: i64,
    pub adverse: i64,
    pub state: State,
    pub halftime: bool,
    /// Minute ou état court, ex : « 67' », « HT », « FT ».
    pub clock: String,
    pub goals: Vec<Goal>,
    pub red_cards: Vec<RedCard>,
}

impl Event {
    /// Buteurs madrilènes (hors contre son camp), dans l'ordre des buts.
    pub fn madrid_scorers(&self) -> Vec<String> {
        self.goals.iter().filter(|g| g.madrid && !g.own_goal).filter_map(|g| g.scorer.clone()).collect()
    }
}

fn text<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn is_madrid(team: &Value) -> bool {
    team.get("id").and_then(Value::as_str) == Some(REAL_MADRID)
        || team.get("displayName").and_then(Value::as_str).is_some_and(|n| n.eq_ignore_ascii_case("Real Madrid"))
}

fn score(competitor: &Value) -> i64 {
    match competitor.get("score") {
        Some(Value::String(s)) => s.trim().parse().unwrap_or(0),
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        _ => 0,
    }
}

/// Date ESPN : `2026-10-25T20:00Z` (sans secondes) ou RFC 3339 complet.
fn parse_date(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|d| d.with_timezone(&Utc))
        .ok()
        .or_else(|| chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%MZ").ok().map(|d| d.and_utc()))
}

/// Image : chaîne directe ou objet `{ "href": … }`.
fn image(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(url) => Some(url.clone()),
        other => other.get("href").and_then(Value::as_str).map(str::to_owned),
    }
    .filter(|url| url.starts_with("https://"))
}

fn is_red_card(detail: &Value) -> bool {
    detail.get("redCard").and_then(Value::as_bool).unwrap_or(false)
        || text(detail, "/type/text").is_some_and(|t| t.to_lowercase().contains("red card"))
}

fn parse_event(event: &Value, league: &str) -> Option<Event> {
    let competition = event.pointer("/competitions/0")?;
    let competitors = competition.get("competitors")?.as_array()?;
    let madrid = competitors.iter().find(|c| c.get("team").is_some_and(is_madrid))?;
    let rival = competitors.iter().find(|c| !c.get("team").is_some_and(is_madrid))?;
    let madrid_team_id = text(madrid, "/team/id").unwrap_or(REAL_MADRID).to_owned();
    let status = event.get("status").or_else(|| competition.get("status"))?;
    let state = match text(status, "/type/state") {
        Some("in") => State::Live,
        Some("post") => State::Post,
        _ => State::Pre,
    };
    let name = text(status, "/type/name").unwrap_or_default();
    let clock = text(status, "/type/shortDetail").or_else(|| text(status, "/displayClock")).unwrap_or_default();
    let kickoff = text(event, "/date").and_then(parse_date)?;
    let details = competition.get("details").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default();
    let red_cards = details
        .iter()
        .filter(|d| is_red_card(d))
        .map(|d| RedCard {
            minute: text(d, "/clock/displayValue").unwrap_or_default().to_owned(),
            player: text(d, "/athletesInvolved/0/displayName").unwrap_or("?").to_owned(),
            madrid: text(d, "/team/id") == Some(madrid_team_id.as_str()),
        })
        .collect();
    let goals = Some(details)
        .map(|details| {
            details
                .iter()
                .filter(|d| d.get("scoringPlay").and_then(Value::as_bool).unwrap_or(false))
                .map(|d| {
                    let own_goal = d.get("ownGoal").and_then(Value::as_bool).unwrap_or(false);
                    let scored_by_madrid_side = text(d, "/team/id") == Some(madrid_team_id.as_str());
                    Goal {
                        minute: text(d, "/clock/displayValue").unwrap_or_default().to_owned(),
                        scorer: text(d, "/athletesInvolved/0/displayName").map(str::to_owned),
                        // ESPN range un contre son camp du côté de l'équipe qui en profite.
                        madrid: scored_by_madrid_side,
                        penalty: d.get("penaltyKick").and_then(Value::as_bool).unwrap_or(false),
                        own_goal,
                        headshot: image(d.pointer("/athletesInvolved/0/headshot")),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Event {
        id: event.get("id").and_then(Value::as_str)?.to_owned(),
        league: league.to_owned(),
        kickoff,
        opponent: text(rival, "/team/displayName").unwrap_or("Adversaire").to_owned(),
        madrid_logo: image(madrid.pointer("/team/logo")),
        opponent_logo: image(rival.pointer("/team/logo")),
        madrid: score(madrid),
        adverse: score(rival),
        state,
        halftime: name == "STATUS_HALFTIME" || clock.eq_ignore_ascii_case("HT"),
        clock: clock.to_owned(),
        goals,
        red_cards,
    })
}

/// Matchs du Real dans une réponse `scoreboard`.
pub fn parse_scoreboard(body: &Value, league: &str) -> Vec<Event> {
    body.get("events")
        .and_then(Value::as_array)
        .map(|events| events.iter().filter_map(|e| parse_event(e, league)).collect())
        .unwrap_or_default()
}

async fn get_from(host: &str, path: &str, query: &[(&str, String)]) -> Result<Value> {
    let response = crate::util::http::client()
        .get(format!("{host}{API_PATH}/{path}"))
        .query(query)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/json, text/plain, */*")
        .header(reqwest::header::ACCEPT_LANGUAGE, "fr-FR,fr;q=0.9,en;q=0.8")
        .header(reqwest::header::REFERER, "https://www.espn.com/")
        .header(reqwest::header::ORIGIN, "https://www.espn.com")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .context("ESPN injoignable")?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("ESPN a répondu {} ({host})", status.as_u16());
    }
    response.json().await.context("réponse ESPN illisible")
}

/// Requête sur le premier hôte qui répond.
async fn get(path: &str, query: &[(&str, String)]) -> Result<Value> {
    let mut last_error = None;
    for host in HOSTS {
        match get_from(host, path, query).await {
            Ok(body) => return Ok(body),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("aucun hôte ESPN")))
}

async fn scoreboard(league: &str, day: NaiveDate) -> Result<Value> {
    get(&format!("{league}/scoreboard"), &[("dates", day.format("%Y%m%d").to_string())]).await
}

/// Onze de départ d'une équipe.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lineup {
    pub team: String,
    pub madrid: bool,
    /// « 1 Courtois », dans l'ordre de l'API.
    pub starters: Vec<String>,
}

/// Statistiques d'une équipe (valeurs affichées par l'API).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub madrid: bool,
    pub possession: Option<String>,
    pub shots: Option<String>,
    pub on_target: Option<String>,
    pub corners: Option<String>,
    pub fouls: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub lineups: Vec<Lineup>,
    pub stats: Vec<Stats>,
}

/// Compos et statistiques d'une réponse `summary?event=…` (vides si l'API ne les donne pas encore).
pub fn parse_summary(body: &Value) -> Summary {
    let lineups = body
        .get("rosters")
        .and_then(Value::as_array)
        .map(|rosters| {
            rosters
                .iter()
                .filter_map(|roster| {
                    let team = roster.get("team")?;
                    let starters: Vec<String> = roster
                        .get("roster")?
                        .as_array()?
                        .iter()
                        .filter(|p| p.get("starter").and_then(Value::as_bool).unwrap_or(false))
                        .filter_map(|p| {
                            let name = text(p, "/athlete/displayName")?;
                            Some(match p.get("jersey").and_then(Value::as_str) {
                                Some(jersey) => format!("{jersey} {name}"),
                                None => name.to_owned(),
                            })
                        })
                        .collect();
                    (!starters.is_empty()).then(|| Lineup {
                        team: text(team, "/displayName").unwrap_or_default().to_owned(),
                        madrid: is_madrid(team),
                        starters,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let stats = body
        .pointer("/boxscore/teams")
        .and_then(Value::as_array)
        .map(|teams| {
            teams
                .iter()
                .filter_map(|entry| {
                    let values = entry.get("statistics")?.as_array()?;
                    let stat = |name: &str| {
                        values
                            .iter()
                            .find(|s| s.get("name").and_then(Value::as_str) == Some(name))
                            .and_then(|s| s.get("displayValue").and_then(Value::as_str))
                            .map(str::to_owned)
                    };
                    Some(Stats {
                        madrid: entry.get("team").is_some_and(is_madrid),
                        possession: stat("possessionPct"),
                        shots: stat("totalShots"),
                        on_target: stat("shotsOnTarget"),
                        corners: stat("wonCorners"),
                        fouls: stat("foulsCommitted"),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Summary { lineups, stats }
}

pub async fn summary(league: &str, event_id: &str) -> Result<Summary> {
    Ok(parse_summary(&get(&format!("{league}/summary"), &[("event", event_id.to_owned())]).await?))
}

/// Matchs du Real ce jour-là (heure UTC), toutes compétitions. Une compétition
/// injoignable est ignorée ; `Err` seulement si aucune n'a répondu.
pub async fn real_madrid_events(day: NaiveDate) -> Result<Vec<Event>> {
    let mut events: Vec<Event> = Vec::new();
    let mut last_error = None;
    let mut answered = false;
    for league in LEAGUES {
        match scoreboard(league, day).await {
            Ok(body) => {
                answered = true;
                for event in parse_scoreboard(&body, league) {
                    if !events.iter().any(|known| known.id == event.id) {
                        events.push(event);
                    }
                }
            }
            Err(error) => last_error = Some(error),
        }
    }
    match (answered, last_error) {
        (false, Some(error)) => Err(error),
        _ => Ok(events),
    }
}

/// Matchs du Real sur les `days` prochains jours (aujourd'hui compris), dans l'ordre.
pub async fn upcoming_events(days: i64) -> Result<Vec<Event>> {
    let today = Utc::now().date_naive();
    let requests = (0..days.max(1)).map(|offset| real_madrid_events(today + chrono::Duration::days(offset)));
    let mut events: Vec<Event> = Vec::new();
    let mut last_error = None;
    for result in futures_util::future::join_all(requests).await {
        match result {
            Ok(found) => {
                events.extend(found.into_iter().filter(|e| !events.iter().any(|k| k.id == e.id)).collect::<Vec<_>>())
            }
            Err(error) => last_error = Some(error),
        }
    }
    if events.is_empty()
        && let Some(error) = last_error
    {
        return Err(error);
    }
    events.sort_by_key(|e| e.kickoff);
    Ok(events)
}

/// Prochain match à créer : en cours ou à venir (pas terminé).
pub fn next_playable(events: &[Event]) -> Option<&Event> {
    events.iter().filter(|e| e.state != State::Post).min_by_key(|e| e.kickoff)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Réponse `scoreboard` réduite aux champs lus.
    pub fn sample(state: &str, name: &str, home: i64, away: i64) -> Value {
        serde_json::json!({ "events": [
            { "id": "999", "date": "2026-10-25T20:00Z", "name": "Barcelona at Getafe",
              "status": { "type": { "state": "in", "name": "STATUS_IN_PROGRESS", "shortDetail": "1'" } },
              "competitions": [{ "competitors": [
                  { "homeAway": "home", "score": "0", "team": { "id": "83", "displayName": "Barcelona" } },
                  { "homeAway": "away", "score": "0", "team": { "id": "3", "displayName": "Getafe" } }
              ]}]
            },
            { "id": "401", "date": "2026-10-25T20:00Z",
              "status": { "type": { "state": state, "name": name, "shortDetail": "67'" } },
              "competitions": [{
                  "competitors": [
                      { "homeAway": "home", "score": home.to_string(), "team": { "id": "86", "displayName": "Real Madrid" } },
                      { "homeAway": "away", "score": away.to_string(), "team": { "id": "83", "displayName": "Barcelona",
                        "logo": "https://a.espncdn.com/i/teamlogos/soccer/500/83.png" } }
                  ],
                  "details": [
                      { "scoringPlay": true, "clock": { "displayValue": "23'" }, "team": { "id": "86" },
                        "athletesInvolved": [{ "displayName": "Kylian Mbappé",
                            "headshot": { "href": "https://a.espncdn.com/i/headshots/soccer/players/full/231388.png" } }],
                        "penaltyKick": true },
                      { "scoringPlay": false, "type": { "text": "Yellow Card" }, "team": { "id": "83" } },
                      { "scoringPlay": true, "clock": { "displayValue": "51'" }, "team": { "id": "83" },
                        "athletesInvolved": [{ "displayName": "Lamine Yamal" }] },
                      { "scoringPlay": false, "redCard": true, "type": { "text": "Red Card" }, "clock": { "displayValue": "70'" },
                        "team": { "id": "83" }, "athletesInvolved": [{ "displayName": "Gavi" }] },
                      { "scoringPlay": true, "clock": { "displayValue": "88'" }, "team": { "id": "86" }, "ownGoal": true,
                        "athletesInvolved": [{ "displayName": "Ronald Araújo" }] }
                  ]
              }]
            }
        ]})
    }

    #[test]
    fn keeps_only_real_madrid_and_reads_the_score() {
        let events = parse_scoreboard(&sample("in", "STATUS_SECOND_HALF", 2, 1), "esp.1");
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!((event.id.as_str(), event.league.as_str(), event.opponent.as_str()), ("401", "esp.1", "Barcelona"));
        assert_eq!((event.madrid, event.adverse, event.state), (2, 1, State::Live));
        assert_eq!(event.clock, "67'");
        assert_eq!(event.kickoff.to_rfc3339(), "2026-10-25T20:00:00+00:00");
        assert!(!event.halftime);
        assert_eq!(event.goals.len(), 3);
        assert!(event.goals[0].penalty && event.goals[0].madrid);
        assert!(!event.goals[1].madrid);
        // Contre son camp : compté pour le Real, mais pas comme buteur madrilène.
        assert!(event.goals[2].madrid && event.goals[2].own_goal);
        assert_eq!(event.madrid_scorers(), vec!["Kylian Mbappé"]);
        assert_eq!(event.opponent_logo.as_deref(), Some("https://a.espncdn.com/i/teamlogos/soccer/500/83.png"));
        assert!(event.madrid_logo.is_none());
        assert!(event.goals[0].headshot.as_deref().is_some_and(|h| h.ends_with("231388.png")));
        assert!(event.goals[1].headshot.is_none());
        assert_eq!(event.red_cards, vec![RedCard { minute: "70'".into(), player: "Gavi".into(), madrid: false }]);
        assert_eq!(event.red_cards[0].key(), "rouge:70':gavi");
    }

    #[test]
    fn parses_both_date_formats() {
        assert_eq!(
            parse_date("2026-10-25T20:00Z").map(|d| d.to_rfc3339()).as_deref(),
            Some("2026-10-25T20:00:00+00:00")
        );
        assert_eq!(
            parse_date("2026-10-25T20:00:00Z").map(|d| d.to_rfc3339()).as_deref(),
            Some("2026-10-25T20:00:00+00:00")
        );
        assert!(parse_date("demain").is_none());
    }

    #[test]
    fn reads_halftime_and_full_time() {
        let halftime = parse_scoreboard(&sample("in", "STATUS_HALFTIME", 1, 0), "esp.1");
        assert!(halftime[0].halftime);
        let finished = parse_scoreboard(&sample("post", "STATUS_FULL_TIME", 3, 1), "esp.1");
        assert_eq!(finished[0].state, State::Post);
        assert!(parse_scoreboard(&serde_json::json!({}), "esp.1").is_empty());
    }

    #[test]
    fn real_madrid_away_is_found_by_id_or_name() {
        let body = serde_json::json!({ "events": [{ "id": "7", "date": "2026-11-01T15:15Z",
            "status": { "type": { "state": "pre" } },
            "competitions": [{ "competitors": [
                { "score": 0, "team": { "id": "999", "displayName": "Atlético Madrid" } },
                { "score": 0, "team": { "displayName": "Real Madrid" } }
            ]}]
        }]});
        let events = parse_scoreboard(&body, "uefa.champions");
        assert_eq!(events[0].opponent, "Atlético Madrid");
        assert_eq!(events[0].state, State::Pre);
    }

    #[test]
    fn reads_lineups_and_stats() {
        let body = serde_json::json!({
            "rosters": [
                { "team": { "id": "86", "displayName": "Real Madrid" }, "roster": [
                    { "starter": true, "jersey": "1", "athlete": { "displayName": "Thibaut Courtois" } },
                    { "starter": false, "jersey": "13", "athlete": { "displayName": "Andriy Lunin" } }
                ]},
                { "team": { "id": "83", "displayName": "Barcelona" }, "roster": [] }
            ],
            "boxscore": { "teams": [
                { "team": { "id": "86" }, "statistics": [
                    { "name": "possessionPct", "displayValue": "58.4" },
                    { "name": "totalShots", "displayValue": "14" },
                    { "name": "shotsOnTarget", "displayValue": "6" }
                ]},
                { "team": { "id": "83" }, "statistics": [{ "name": "possessionPct", "displayValue": "41.6" }] }
            ]}
        });
        let summary = parse_summary(&body);
        assert_eq!(summary.lineups.len(), 1, "une compo vide n'est pas gardée");
        assert_eq!(summary.lineups[0].starters, vec!["1 Thibaut Courtois"]);
        assert!(summary.lineups[0].madrid);
        assert_eq!(summary.stats[0].possession.as_deref(), Some("58.4"));
        assert_eq!(summary.stats[0].on_target.as_deref(), Some("6"));
        assert!(!summary.stats[1].madrid);
        assert_eq!(parse_summary(&serde_json::json!({})), Summary::default());
    }
}
