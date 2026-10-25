//! Tâche de fond de la soirée Clásico, toutes les minutes :
//! - retire les rôles clown arrivés à échéance ;
//! - si `FOOTBALL_DATA_TOKEN` est défini (clé gratuite de football-data.org),
//!   suit le score des matchs en cours : message du match mis à jour, but annoncé,
//!   coup de sifflet final signalé (le résultat reste validé par un gradé).

use super::{announce_goal, refresh, service};
use crate::{
    context::{Context, Ctx},
    discord::messages::{self, Outgoing, parse_id},
};
use anyhow::{Context as _, Result};
use serde_json::Value;
use std::time::Duration;

const INTERVAL: Duration = Duration::from_secs(60);
const API: &str = "https://api.football-data.org/v4/teams";
/// Identifiant du Real Madrid chez football-data.org.
const REAL_MADRID: i64 = 86;
/// Suivi jusqu'à 3 h après le coup d'envoi.
const LIVE_HOURS: i64 = 3;

pub fn spawn(ctx: Context) {
    let token = std::env::var("FOOTBALL_DATA_TOKEN").ok().filter(|t| !t.trim().is_empty());
    if token.is_some() {
        tracing::info!("clásico : suivi du score en direct activé (football-data.org)");
    }
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            expire_clowns(&ctx).await;
            if let Some(token) = &token {
                follow(&ctx, token).await;
            }
        }
    });
}

async fn expire_clowns(ctx: &Ctx) {
    let clowns = match service::expired_clowns(&ctx.db).await {
        Ok(clowns) => clowns,
        Err(error) => return tracing::warn!(?error, "clásico : lecture des rôles clown"),
    };
    for (guild, user, role) in clowns {
        if let (Some(guild_id), Some(user_id), Some(role_id)) = (parse_id(&guild), parse_id(&user), parse_id(&role)) {
            // Membre parti ou rôle supprimé : rien à retirer, on oublie l'entrée.
            let _ = ctx.http.remove_guild_member_role(guild_id, user_id, role_id).await;
        }
        if let Err(error) = service::remove_clown(&ctx.db, &guild, &user, &role).await {
            tracing::warn!(?error, "clásico : oubli d'un rôle clown");
        }
    }
}

/// Score lu dans l'API : (buts du Real, buts de l'adversaire, statut).
#[derive(Debug, PartialEq, Eq)]
pub struct LiveScore {
    pub madrid: i64,
    pub adverse: i64,
    pub status: String,
}

/// Match du Real dans la réponse `GET /v4/teams/86/matches` : score courant
/// (`score.fullTime`, mis à jour pendant le match) vu du Real.
pub fn parse_live(body: &Value) -> Option<LiveScore> {
    let matches = body.get("matches")?.as_array()?;
    let game = matches.iter().find(|m| {
        let status = m.get("status").and_then(Value::as_str).unwrap_or_default();
        matches!(status, "IN_PLAY" | "PAUSED" | "FINISHED" | "LIVE")
    })?;
    let home_is_madrid = game.pointer("/homeTeam/id").and_then(Value::as_i64) == Some(REAL_MADRID);
    let goals = |side: &str| game.pointer(&format!("/score/fullTime/{side}")).and_then(Value::as_i64).unwrap_or(0);
    let (home, away) = (goals("home"), goals("away"));
    let (madrid, adverse) = if home_is_madrid { (home, away) } else { (away, home) };
    let status = game.get("status").and_then(Value::as_str).unwrap_or_default().to_owned();
    Some(LiveScore { madrid, adverse, status })
}

async fn fetch(token: &str, day: &str) -> Result<Value> {
    let response = crate::util::http::client()
        .get(format!("{API}/{REAL_MADRID}/matches"))
        .query(&[("dateFrom", day), ("dateTo", day)])
        .header("X-Auth-Token", token)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .context("football-data.org injoignable")?
        .error_for_status()
        .context("football-data.org a refusé la requête")?;
    Ok(response.json().await?)
}

async fn follow(ctx: &Ctx, token: &str) {
    let games = match service::live_matches(&ctx.db, LIVE_HOURS).await {
        Ok(games) => games,
        Err(error) => return tracing::warn!(?error, "clásico : lecture des matchs en cours"),
    };
    for game in games {
        let day = game.kickoff.0.format("%Y-%m-%d").to_string();
        let live = match fetch(token, &day).await.map(|body| parse_live(&body)) {
            Ok(Some(live)) => live,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(guild = %game.guild_id, ?error, "clásico : score en direct indisponible");
                continue;
            }
        };
        let changed = (live.madrid, live.adverse) != (game.live_madrid, game.live_adverse);
        let finished = live.status == "FINISHED" && game.live_status.as_deref() != Some("FINISHED");
        if !changed && game.live_status.as_deref() == Some(live.status.as_str()) {
            continue;
        }
        if let Err(error) = service::set_live(&ctx.db, &game.id, live.madrid, live.adverse, Some(&live.status)).await {
            tracing::warn!(?error, "clásico : enregistrement du score en direct");
            continue;
        }
        if changed {
            let madrid_scored = live.madrid > game.live_madrid;
            announce_goal(ctx, &game, live.madrid, live.adverse, madrid_scored, None).await;
        }
        refresh(ctx, &game.guild_id, &game.id).await;
        if finished && let Some(channel) = parse_id(&game.channel_id) {
            let text = format!(
                "🏁 Coup de sifflet final : **Real Madrid {} – {} {}**. Un gradé valide avec `/halamadrid resultat` (score déjà prérempli) pour distribuer points et gains.",
                live.madrid, live.adverse, game.adversaire
            );
            if let Err(error) = messages::send(ctx, channel, &Outgoing::text(text)).await {
                tracing::warn!(?error, "clásico : annonce de fin de match");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_score_from_the_real_madrid_side() {
        let away = serde_json::json!({ "matches": [{
            "status": "IN_PLAY",
            "homeTeam": { "id": 81, "name": "FC Barcelona" },
            "awayTeam": { "id": 86, "name": "Real Madrid CF" },
            "score": { "fullTime": { "home": 1, "away": 2 } }
        }]});
        assert_eq!(parse_live(&away), Some(LiveScore { madrid: 2, adverse: 1, status: "IN_PLAY".into() }));
        let home_not_started = serde_json::json!({ "matches": [{
            "status": "TIMED",
            "homeTeam": { "id": 86 }, "awayTeam": { "id": 81 },
            "score": { "fullTime": { "home": null, "away": null } }
        }]});
        assert_eq!(parse_live(&home_not_started), None);
        let kicked_off = serde_json::json!({ "matches": [{
            "status": "IN_PLAY",
            "homeTeam": { "id": 86 }, "awayTeam": { "id": 81 },
            "score": { "fullTime": { "home": null, "away": null } }
        }]});
        assert_eq!(parse_live(&kicked_off), Some(LiveScore { madrid: 0, adverse: 0, status: "IN_PLAY".into() }));
        assert_eq!(parse_live(&serde_json::json!({})), None);
    }
}
