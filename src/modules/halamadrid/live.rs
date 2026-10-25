//! Tâche de fond de la soirée Clásico, toutes les minutes, sur l'API publique d'ESPN :
//! - ouverture automatique des matchs du Real (serveurs en `/halamadrid auto`) ;
//! - suivi en direct de tout match en cours (ouvert à la main ou non) : compos, coup
//!   d'envoi (fil du match), buts, rouges, mi-temps et fin avec statistiques ;
//! - résultat automatique au coup de sifflet final ;
//! - retrait des rôles clown arrivés à échéance.

use super::{GoalInfo, announce_goal, cards, espn, match_message, refresh, service, settle_match};
use crate::{
    context::{Context, Ctx},
    discord::messages::{self, Mentions, Outgoing, parse_id},
    util::time::DbTime,
};
use anyhow::Result;
use chrono::{Duration, NaiveDate, Utc};
use std::{collections::HashMap, time::Duration as StdDuration};
use twilight_model::channel::thread::AutoArchiveDuration;

const INTERVAL: StdDuration = StdDuration::from_secs(60);
/// Suivi jusqu'à 3 h après le coup d'envoi.
const LIVE_HOURS: i64 = 3;
/// Ouverture automatique au plus tôt 12 h avant le coup d'envoi.
const OPEN_AHEAD_HOURS: i64 = 12;
/// Compos publiées dès qu'elles sont connues, à partir d'1 h avant.
const LINEUPS_AHEAD_MINUTES: i64 = 75;
/// Un événement de l'API correspond à un match ouvert à la main s'ils commencent à moins de 2 h d'écart.
const MATCH_WINDOW_HOURS: i64 = 2;

pub fn spawn(ctx: Context) {
    tokio::spawn(async move {
        // Laisse au bot le temps de se connecter et de remplir son cache.
        tokio::time::sleep(StdDuration::from_secs(30)).await;
        let mut ticker = tokio::time::interval(INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut tick: u64 = 0;
        loop {
            ticker.tick().await;
            expire_clowns(&ctx).await;
            let mut events = Events::default();
            // Toutes les 10 minutes suffisent pour repérer les prochains matchs.
            if tick.is_multiple_of(10) {
                open_upcoming(&ctx, &mut events).await;
            }
            follow(&ctx, &mut events).await;
            tick += 1;
        }
    });
    tracing::info!("clásico : suivi automatique des matchs du Real démarré (ESPN)");
}

/// Matchs du Real par jour, lus une fois par passage.
#[derive(Default)]
struct Events(HashMap<NaiveDate, Option<Vec<espn::Event>>>);

impl Events {
    async fn day(&mut self, day: NaiveDate) -> Option<&[espn::Event]> {
        if let std::collections::hash_map::Entry::Vacant(slot) = self.0.entry(day) {
            let events = match espn::real_madrid_events(day).await {
                Ok(events) => Some(events),
                Err(error) => {
                    tracing::warn!(?error, "clásico : ESPN indisponible");
                    None
                }
            };
            slot.insert(events);
        }
        self.0.get(&day).and_then(Option::as_deref)
    }
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

/// Nom affiché de l'adversaire (quelques clubs en français).
pub fn rival_name(espn_name: &str) -> String {
    match espn_name {
        "Barcelona" | "FC Barcelona" => "FC Barcelone",
        "Atletico Madrid" | "Atlético Madrid" => "Atlético de Madrid",
        "Bayern Munich" => "Bayern Munich",
        "Seville" | "Sevilla" => "Séville",
        other => other,
    }
    .to_owned()
}

/// Prochain match du Real à ouvrir : pas commencé, dans les 12 h.
pub fn next_to_open(events: &[espn::Event], now: chrono::DateTime<Utc>) -> Option<&espn::Event> {
    events
        .iter()
        .filter(|e| {
            e.state == espn::State::Pre && e.kickoff > now && e.kickoff <= now + Duration::hours(OPEN_AHEAD_HOURS)
        })
        .min_by_key(|e| e.kickoff)
}

async fn open_upcoming(ctx: &Ctx, events: &mut Events) {
    let guilds = match service::auto_guilds(&ctx.db).await {
        Ok(guilds) if !guilds.is_empty() => guilds,
        Ok(_) => return,
        Err(error) => return tracing::warn!(?error, "clásico : lecture des serveurs automatiques"),
    };
    let now = Utc::now();
    let mut upcoming: Vec<espn::Event> = Vec::new();
    for day in [now.date_naive(), (now + Duration::days(1)).date_naive()] {
        if let Some(found) = events.day(day).await {
            upcoming.extend(found.iter().cloned());
        }
    }
    let Some(event) = next_to_open(&upcoming, now).cloned() else { return };
    for (guild, channel) in guilds {
        if let Err(error) = open_for_guild(ctx, &guild, &channel, &event).await {
            tracing::warn!(guild, ?error, "clásico : ouverture automatique d'un match");
        }
    }
}

async fn open_for_guild(ctx: &Ctx, guild: &str, channel: &str, event: &espn::Event) -> Result<()> {
    // Un seul match non réglé à la fois, et jamais deux fois le même.
    if service::current(&ctx.db, guild).await?.is_some() || service::has_external(&ctx.db, guild, &event.id).await? {
        return Ok(());
    }
    let Some(channel_id) = parse_id(channel) else { return Ok(()) };
    let id = service::create_external(&ctx.db, guild, event, &rival_name(&event.opponent), channel).await?;
    let Some(game) = service::get(&ctx.db, guild, &id).await? else { return Ok(()) };
    let sent = messages::send(ctx, channel_id, &match_message(&game, &[], &[], DbTime::now())).await?;
    service::set_message(&ctx.db, &id, &sent.id.to_string()).await?;
    tracing::info!(guild, rival = %game.adversaire, "clásico : match ouvert automatiquement");
    Ok(())
}

/// Événement de l'API correspondant à un match : par identifiant, sinon par heure de coup d'envoi.
pub fn find_event<'a>(game: &service::Match, events: &'a [espn::Event]) -> Option<&'a espn::Event> {
    match game.external_id.as_deref() {
        Some(id) => events.iter().find(|e| e.id == id),
        None => events
            .iter()
            .filter(|e| (e.kickoff - game.kickoff.0).num_minutes().abs() <= MATCH_WINDOW_HOURS * 60)
            .min_by_key(|e| (e.kickoff - game.kickoff.0).num_minutes().abs()),
    }
}

async fn follow(ctx: &Ctx, events: &mut Events) {
    let games = match service::live_matches(&ctx.db, LIVE_HOURS).await {
        Ok(games) => games,
        Err(error) => return tracing::warn!(?error, "clásico : lecture des matchs en cours"),
    };
    // Compos : les matchs qui commencent bientôt sont lus aussi.
    let soon = match upcoming_for_lineups(ctx).await {
        Ok(soon) => soon,
        Err(error) => {
            tracing::warn!(?error, "clásico : lecture des prochains matchs");
            Vec::new()
        }
    };
    // Un match qui commence dans les 10 min figure dans les deux listes : une seule fois.
    let mut seen = std::collections::HashSet::new();
    for game in soon.into_iter().chain(games).filter(|g| seen.insert(g.id.clone())) {
        let day = game.kickoff.0.date_naive();
        let Some(day_events) = events.day(day).await else { continue };
        let Some(event) = find_event(&game, day_events).cloned() else { continue };
        if let Err(error) = follow_game(ctx, game, &event).await {
            tracing::warn!(?error, "clásico : suivi d'un match");
        }
    }
}

/// Matchs non réglés qui commencent dans l'heure et quart, compos pas encore publiées.
async fn upcoming_for_lineups(ctx: &Ctx) -> Result<Vec<service::Match>> {
    let now = Utc::now();
    let mut games = Vec::new();
    for (guild, _) in service::auto_guilds(&ctx.db).await? {
        if let Some(game) = service::current(&ctx.db, &guild).await?
            && game.kickoff.0 > now
            && game.kickoff.0 <= now + Duration::minutes(LINEUPS_AHEAD_MINUTES)
            && !game.has_announced("lineups")
        {
            games.push(game);
        }
    }
    Ok(games)
}

async fn post(ctx: &Ctx, channel: &str, message: &Outgoing) {
    let Some(channel_id) = parse_id(channel) else { return };
    if let Err(error) = messages::send(ctx, channel_id, message).await {
        tracing::warn!(?error, "clásico : annonce en direct");
    }
}

async fn follow_game(ctx: &Ctx, mut game: service::Match, event: &espn::Event) -> Result<()> {
    let guild = game.guild_id.clone();
    service::set_feed(&ctx.db, &game.id, &event.id, &event.league, &event.clock, event.opponent_logo.as_deref())
        .await?;
    let auto = service::auto(&ctx.db, &guild).await?;
    let mut changed = game.live_clock.as_deref() != Some(event.clock.as_str());

    // Compos (dès que l'API les donne).
    if !game.has_announced("lineups")
        && let Ok(summary) = espn::summary(&event.league, &event.id).await
        && let Some(card) = cards::lineups_card(&summary.lineups, event.opponent_logo.as_deref())
    {
        post(ctx, &game.channel_id, &Outgoing::embeds(vec![card])).await;
        service::mark_announced(&ctx.db, &game.id, "lineups").await?;
    }
    if event.state == espn::State::Pre {
        return Ok(());
    }

    // Coup d'envoi : fil du match, ping Madridista.
    if !game.has_announced("kickoff") {
        if game.thread_id.is_none()
            && let (Some(channel), Some(message)) =
                (parse_id(&game.channel_id), game.message_id.as_deref().and_then(parse_id))
        {
            let name = crate::util::format::truncate(&format!("⚽ Real Madrid – {} · en direct", game.adversaire), 100);
            match ctx
                .http
                .create_thread_from_message(channel, message, &name)
                .auto_archive_duration(AutoArchiveDuration::Day)
                .await
            {
                Ok(response) => {
                    let thread = response.model().await?;
                    service::set_thread(&ctx.db, &game.id, &thread.id.to_string()).await?;
                }
                Err(error) => {
                    tracing::warn!(?error, "clásico : fil du match impossible (permission « Créer des fils » ?)")
                }
            }
        }
        game = service::get(&ctx.db, &guild, &game.id).await?.unwrap_or(game);
        let (content, embed) =
            cards::kickoff_card(&game.adversaire, event.opponent_logo.as_deref(), auto.madridista_role_id.as_deref());
        let mut message = Outgoing::embeds(vec![embed]);
        if let (Some(content), Some(role)) = (content, auto.madridista_role_id.as_deref().and_then(parse_id)) {
            message.content = Some(content);
            message.mentions = Mentions::Explicit { users: Vec::new(), roles: vec![role], everyone: false };
        }
        post(ctx, game.live_channel(), &message).await;
        service::mark_announced(&ctx.db, &game.id, "kickoff").await?;
        changed = true;
    }

    // Buts (et buts annulés), dans l'ordre de l'API.
    let (mut madrid, mut adverse) = (game.live_madrid, game.live_adverse);
    if event.madrid < madrid || event.adverse < adverse {
        let embed = cards::base("VAR", cards::ALERT, None)
            .title("🖥️ But annulé après vérification")
            .description(cards::scoreline(event.madrid, event.adverse, &game.adversaire))
            .build();
        post(ctx, game.live_channel(), &Outgoing::embeds(vec![embed])).await;
        (madrid, adverse) = (event.madrid.min(madrid), event.adverse.min(adverse));
        changed = true;
    }
    let madrid_goals: Vec<&espn::Goal> = event.goals.iter().filter(|g| g.madrid).collect();
    let rival_goals: Vec<&espn::Goal> = event.goals.iter().filter(|g| !g.madrid).collect();
    while madrid < event.madrid || adverse < event.adverse {
        let real = madrid < event.madrid;
        let goal = if real { madrid_goals.get(madrid as usize) } else { rival_goals.get(adverse as usize) };
        if real {
            madrid += 1
        } else {
            adverse += 1
        }
        service::set_live(&ctx.db, &game.id, madrid, adverse, None).await?;
        let info = goal.map_or_else(GoalInfo::default, |g| GoalInfo {
            scorer: g.scorer.as_deref(),
            minute: Some(g.minute.as_str()),
            penalty: g.penalty,
            own_goal: g.own_goal,
            headshot: g.headshot.as_deref(),
        });
        announce_goal(ctx, &game, madrid, adverse, real, info).await;
        changed = true;
    }
    let status = if event.halftime {
        "HT"
    } else if event.state == espn::State::Post {
        "FT"
    } else {
        "LIVE"
    };
    if game.live_status.as_deref() != Some(status) {
        changed = true;
    }
    service::set_live(&ctx.db, &game.id, madrid, adverse, Some(status)).await?;

    // Cartons rouges.
    for card in &event.red_cards {
        let key = card.key();
        if !game.has_announced(&key) {
            post(ctx, game.live_channel(), &Outgoing::embeds(vec![cards::red_card_card(card, &game.adversaire)])).await;
            service::mark_announced(&ctx.db, &game.id, &key).await?;
        }
    }

    // Mi-temps et fin, avec les statistiques.
    let stats = || async { espn::summary(&event.league, &event.id).await.map(|s| s.stats).unwrap_or_default() };
    if event.halftime && !game.has_announced("ht") {
        let card =
            cards::halftime_card(madrid, adverse, &game.adversaire, event.opponent_logo.as_deref(), &stats().await);
        post(ctx, game.live_channel(), &Outgoing::embeds(vec![card])).await;
        service::mark_announced(&ctx.db, &game.id, "ht").await?;
    }
    if event.state == espn::State::Post && !game.has_announced("ft") {
        let card = cards::full_time_card(
            event.madrid,
            event.adverse,
            &game.adversaire,
            event.opponent_logo.as_deref(),
            &stats().await,
        );
        post(ctx, game.live_channel(), &Outgoing::embeds(vec![card])).await;
        service::mark_announced(&ctx.db, &game.id, "ft").await?;
        // Résultat automatique : score et buteurs de l'API.
        let game = service::get(&ctx.db, &guild, &game.id).await?.unwrap_or(game);
        let scorers = event.madrid_scorers().join(", ");
        let clown = auto.clown_role_id.as_deref().and_then(parse_id);
        let scorers = (!scorers.is_empty()).then_some(scorers.as_str());
        let result = settle_match(ctx, &guild, &game, event.madrid, event.adverse, scorers, clown).await?;
        post(ctx, &game.channel_id, &Outgoing::embeds(vec![result])).await;
        return Ok(());
    }
    if changed {
        refresh(ctx, &guild, &game.id).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, minutes_from_now: i64, state: espn::State) -> espn::Event {
        espn::Event {
            id: id.into(),
            league: "esp.1".into(),
            kickoff: Utc::now() + Duration::minutes(minutes_from_now),
            opponent: "Barcelona".into(),
            madrid_logo: None,
            opponent_logo: None,
            madrid: 0,
            adverse: 0,
            state,
            halftime: false,
            clock: String::new(),
            goals: Vec::new(),
            red_cards: Vec::new(),
        }
    }

    #[test]
    fn opens_the_next_match_within_twelve_hours() {
        let now = Utc::now();
        let events = [
            event("late", 60 * 20, espn::State::Pre),
            event("soon", 60 * 3, espn::State::Pre),
            event("live", -30, espn::State::Live),
        ];
        assert_eq!(next_to_open(&events, now).map(|e| e.id.as_str()), Some("soon"));
        assert!(next_to_open(&events[..1], now).is_none());
        assert_eq!(rival_name("Barcelona"), "FC Barcelone");
        assert_eq!(rival_name("Getafe"), "Getafe");
    }

    #[test]
    fn manual_matches_are_linked_by_kickoff() {
        let events = [event("a", 60 * 5, espn::State::Pre), event("b", 10, espn::State::Pre)];
        let game = service::Match {
            id: "m".into(),
            guild_id: "1".into(),
            adversaire: "FC Barcelone".into(),
            kickoff: DbTime(Utc::now()),
            channel_id: "1".into(),
            message_id: None,
            live_madrid: 0,
            live_adverse: 0,
            live_status: None,
            score_madrid: None,
            score_adverse: None,
            buteurs: None,
            external_id: None,
            live_clock: None,
            league: None,
            rival_logo: None,
            thread_id: None,
            announced: String::new(),
        };
        assert_eq!(find_event(&game, &events).map(|e| e.id.as_str()), Some("b"));
        let linked = service::Match { external_id: Some("a".into()), ..game.clone() };
        assert_eq!(find_event(&linked, &events).map(|e| e.id.as_str()), Some("a"));
        let far = service::Match { kickoff: DbTime(Utc::now() - Duration::hours(10)), ..game };
        assert!(find_event(&far, &events).is_none());
    }
}
