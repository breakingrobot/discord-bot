//! Matchs, pronostics, paris en argent fictif, homme du match et rôle clown.

use crate::util::{ids::new_id, time::DbTime};
use anyhow::Result;
use sqlx::{FromRow, SqlitePool};

/// Buts au plus dans un pronostic ou un résultat.
pub const MAX_GOALS: i64 = 20;
/// Longueur maximale d'un nom de buteur.
pub const MAX_SCORER: usize = 40;
/// Cagnotte de départ de chaque membre, en $ fictifs.
pub const STARTING_BALANCE: i64 = 10_000;

#[derive(Clone, Debug, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Match {
    pub id: String,
    pub guild_id: String,
    pub adversaire: String,
    pub kickoff: DbTime,
    pub channel_id: String,
    pub message_id: Option<String>,
    pub live_madrid: i64,
    pub live_adverse: i64,
    pub live_status: Option<String>,
    pub score_madrid: Option<i64>,
    pub score_adverse: Option<i64>,
    pub buteurs: Option<String>,
    /// Événement de l'API ESPN suivi (NULL : pas encore relié).
    pub external_id: Option<String>,
    /// Minute en direct, ex : « 67' ».
    pub live_clock: Option<String>,
    /// Compétition ESPN (ex : `esp.1`).
    pub league: Option<String>,
    pub rival_logo: Option<String>,
    /// Fil Discord des annonces en direct.
    pub thread_id: Option<String>,
    /// Annonces déjà publiées, séparées par des virgules.
    pub announced: String,
}

impl Match {
    pub fn settled(&self) -> Option<(i64, i64)> {
        self.score_madrid.zip(self.score_adverse)
    }

    /// Pronostics et paris encore ouverts (avant le coup d'envoi, match pas réglé).
    pub fn open(&self, now: DbTime) -> bool {
        self.settled().is_none() && now.0 < self.kickoff.0
    }

    /// Cette annonce (kickoff, ht, ft, lineups, rouge:…) a-t-elle déjà été publiée ?
    pub fn has_announced(&self, key: &str) -> bool {
        self.announced.split(',').any(|known| known == key)
    }

    /// Salon des annonces en direct : le fil du match s'il existe.
    pub fn live_channel(&self) -> &str {
        self.thread_id.as_deref().unwrap_or(&self.channel_id)
    }
}

#[derive(Clone, Debug, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Prono {
    pub user_id: String,
    pub madrid: i64,
    pub adverse: i64,
    pub buteur: Option<String>,
    pub points: Option<i64>,
}

/// Issue d'un match, vue du Real.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Victoire,
    Nul,
    Defaite,
}

impl Outcome {
    pub const ALL: [Self; 3] = [Self::Victoire, Self::Nul, Self::Defaite];

    pub fn of(madrid: i64, adverse: i64) -> Self {
        match madrid.cmp(&adverse) {
            std::cmp::Ordering::Greater => Self::Victoire,
            std::cmp::Ordering::Equal => Self::Nul,
            std::cmp::Ordering::Less => Self::Defaite,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Victoire => "VICTOIRE",
            Self::Nul => "NUL",
            Self::Defaite => "DEFAITE",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|o| o.as_str() == raw)
    }
}

/// Nom de buteur comparable : sans accents ni majuscules, espaces réduits.
pub fn fold(name: &str) -> String {
    deunicode::deunicode(name).to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Le buteur annoncé figure-t-il parmi les buteurs (nom complet ou nom de famille) ?
pub fn scorer_matches(announced: &str, scorers: &str) -> bool {
    let announced = fold(announced);
    if announced.is_empty() {
        return false;
    }
    scorers.split([',', ';', '\n']).map(fold).filter(|s| !s.is_empty()).any(|scorer| {
        scorer == announced
            || scorer.split(' ').next_back() == Some(announced.as_str())
            || announced.split(' ').next_back() == Some(scorer.as_str())
    })
}

/// Barème : score exact 3 points, bonne issue 1 point, buteur annoncé qui marque +1.
pub fn points(prono: &Prono, madrid: i64, adverse: i64, scorers: Option<&str>) -> i64 {
    let base = if (prono.madrid, prono.adverse) == (madrid, adverse) {
        3
    } else if Outcome::of(prono.madrid, prono.adverse) == Outcome::of(madrid, adverse) {
        1
    } else {
        0
    };
    let bonus = match (prono.buteur.as_deref(), scorers) {
        (Some(announced), Some(scorers)) if scorer_matches(announced, scorers) => 1,
        _ => 0,
    };
    base + bonus
}

/// Pronostic le plus éloigné du score final (écart de buts cumulé) ; `None` sans pronostic.
pub fn worst(pronos: &[Prono], madrid: i64, adverse: i64) -> Option<&Prono> {
    pronos.iter().max_by_key(|p| ((p.madrid - madrid).abs() + (p.adverse - adverse).abs(), p.adverse - p.madrid))
}

/// Répartition des pronostics : victoire, nul, défaite (en %, arrondis).
pub fn tendencies(pronos: &[Prono]) -> Option<[u32; 3]> {
    if pronos.is_empty() {
        return None;
    }
    let mut counts = [0u32; 3];
    for prono in pronos {
        let outcome = Outcome::of(prono.madrid, prono.adverse);
        counts[Outcome::ALL.iter().position(|o| *o == outcome).unwrap_or(0)] += 1;
    }
    let total = pronos.len() as f64;
    Some(counts.map(|count| (f64::from(count) * 100.0 / total).round() as u32))
}

/// Pari mutuel : les perdants financent les gagnants au prorata des mises. Personne
/// n'a trouvé l'issue : chacun récupère sa mise. Renvoie le gain (mise comprise) de chaque pari.
pub fn payouts(bets: &[(Outcome, i64)], result: Outcome) -> Vec<i64> {
    let pot: i64 = bets.iter().map(|(_, mise)| mise).sum();
    let winning: i64 = bets.iter().filter(|(o, _)| *o == result).map(|(_, mise)| mise).sum();
    bets.iter()
        .map(|(outcome, mise)| match (winning, *outcome == result) {
            (0, _) => *mise,
            (_, true) => (i128::from(*mise) * i128::from(pot) / i128::from(winning)) as i64,
            (_, false) => 0,
        })
        .collect()
}

/// Cote d'une issue : pot total ÷ mises sur cette issue (`None` : personne n'a misé dessus).
pub fn odds(bets: &[(Outcome, i64)], outcome: Outcome) -> Option<f64> {
    let pot: i64 = bets.iter().map(|(_, mise)| mise).sum();
    let on: i64 = bets.iter().filter(|(o, _)| *o == outcome).map(|(_, mise)| mise).sum();
    (on > 0).then(|| pot as f64 / on as f64)
}

const MATCH_COLUMNS: &str = r#""id", "guildId", "adversaire", "kickoff", "channelId", "messageId", "liveMadrid", "liveAdverse", "liveStatus", "scoreMadrid", "scoreAdverse", "buteurs", "externalId", "liveClock", "league", "rivalLogo", "threadId", "announced""#;

/// Dernier match non réglé du serveur.
pub async fn current(db: &SqlitePool, guild_id: &str) -> Result<Option<Match>> {
    Ok(sqlx::query_as(&format!(
        r#"SELECT {MATCH_COLUMNS} FROM "MadridMatch" WHERE "guildId" = ? AND "settledAt" IS NULL ORDER BY "kickoff" DESC LIMIT 1"#
    ))
    .bind(guild_id)
    .fetch_optional(db)
    .await?)
}

/// Dernier match réglé du serveur.
pub async fn last_settled(db: &SqlitePool, guild_id: &str) -> Result<Option<Match>> {
    Ok(sqlx::query_as(&format!(
        r#"SELECT {MATCH_COLUMNS} FROM "MadridMatch" WHERE "guildId" = ? AND "settledAt" IS NOT NULL ORDER BY "settledAt" DESC LIMIT 1"#
    ))
    .bind(guild_id)
    .fetch_optional(db)
    .await?)
}

/// Matchs non réglés dont le coup d'envoi est passé depuis moins de `hours` heures (suivi en direct).
pub async fn live_matches(db: &SqlitePool, hours: i64) -> Result<Vec<Match>> {
    let now = DbTime::now();
    Ok(sqlx::query_as(&format!(
        r#"SELECT {MATCH_COLUMNS} FROM "MadridMatch" WHERE "settledAt" IS NULL AND "kickoff" <= ? AND "kickoff" >= ?"#
    ))
    .bind(DbTime(now.0 + chrono::Duration::minutes(10)))
    .bind(DbTime(now.0 - chrono::Duration::hours(hours)))
    .fetch_all(db)
    .await?)
}

pub async fn get(db: &SqlitePool, guild_id: &str, id: &str) -> Result<Option<Match>> {
    Ok(sqlx::query_as(&format!(r#"SELECT {MATCH_COLUMNS} FROM "MadridMatch" WHERE "guildId" = ? AND "id" = ?"#))
        .bind(guild_id)
        .bind(id)
        .fetch_optional(db)
        .await?)
}

pub async fn create(
    db: &SqlitePool,
    guild_id: &str,
    adversaire: &str,
    kickoff: DbTime,
    channel_id: &str,
    created_by: &str,
) -> Result<String> {
    let id = new_id();
    sqlx::query(
        r#"INSERT INTO "MadridMatch" ("id", "guildId", "adversaire", "kickoff", "channelId", "createdById", "createdAt")
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(guild_id)
    .bind(adversaire)
    .bind(kickoff)
    .bind(channel_id)
    .bind(created_by)
    .bind(DbTime::now())
    .execute(db)
    .await?;
    Ok(id)
}

pub async fn set_message(db: &SqlitePool, id: &str, message_id: &str) -> Result<()> {
    sqlx::query(r#"UPDATE "MadridMatch" SET "messageId" = ? WHERE "id" = ?"#)
        .bind(message_id)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Score en direct (et statut de l'API s'il est connu).
pub async fn set_live(db: &SqlitePool, id: &str, madrid: i64, adverse: i64, status: Option<&str>) -> Result<()> {
    sqlx::query(
        r#"UPDATE "MadridMatch" SET "liveMadrid" = ?, "liveAdverse" = ?, "liveStatus" = COALESCE(?, "liveStatus") WHERE "id" = ?"#,
    )
    .bind(madrid)
    .bind(adverse)
    .bind(status)
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}

/// Événement de l'API relié au match : identifiant, compétition, minute, logo adverse.
pub async fn set_feed(
    db: &SqlitePool,
    id: &str,
    external_id: &str,
    league: &str,
    clock: &str,
    rival_logo: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"UPDATE "MadridMatch" SET "externalId" = ?, "league" = ?, "liveClock" = ?,
           "rivalLogo" = COALESCE(?, "rivalLogo") WHERE "id" = ?"#,
    )
    .bind(external_id)
    .bind(league)
    .bind(clock)
    .bind(rival_logo)
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}

/// Note une annonce comme publiée (une seule fois par match).
pub async fn mark_announced(db: &SqlitePool, id: &str, key: &str) -> Result<()> {
    sqlx::query(
        r#"UPDATE "MadridMatch" SET "announced" = CASE WHEN "announced" = '' THEN ? ELSE "announced" || ',' || ? END
           WHERE "id" = ?"#,
    )
    .bind(key)
    .bind(key)
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_thread(db: &SqlitePool, id: &str, thread_id: &str) -> Result<()> {
    sqlx::query(r#"UPDATE "MadridMatch" SET "threadId" = ? WHERE "id" = ?"#)
        .bind(thread_id)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

/// Ce serveur a-t-il déjà un match pour cet événement de l'API ?
pub async fn has_external(db: &SqlitePool, guild_id: &str, external_id: &str) -> Result<bool> {
    let row: Option<(i64,)> = sqlx::query_as(r#"SELECT 1 FROM "MadridMatch" WHERE "guildId" = ? AND "externalId" = ?"#)
        .bind(guild_id)
        .bind(external_id)
        .fetch_optional(db)
        .await?;
    Ok(row.is_some())
}

pub async fn pronos(db: &SqlitePool, match_id: &str) -> Result<Vec<Prono>> {
    Ok(sqlx::query_as(
        r#"SELECT "userId", "madrid", "adverse", "buteur", "points" FROM "MadridProno" WHERE "matchId" = ?
           ORDER BY "points" DESC, "updatedAt" ASC"#,
    )
    .bind(match_id)
    .fetch_all(db)
    .await?)
}

/// Enregistre (ou remplace) le pronostic d'un membre.
pub async fn set_prono(
    db: &SqlitePool,
    match_id: &str,
    user_id: &str,
    madrid: i64,
    adverse: i64,
    buteur: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO "MadridProno" ("matchId", "userId", "madrid", "adverse", "buteur", "updatedAt") VALUES (?, ?, ?, ?, ?, ?)
           ON CONFLICT ("matchId", "userId") DO UPDATE SET "madrid" = excluded."madrid", "adverse" = excluded."adverse",
           "buteur" = excluded."buteur", "updatedAt" = excluded."updatedAt""#,
    )
    .bind(match_id)
    .bind(user_id)
    .bind(madrid)
    .bind(adverse)
    .bind(buteur)
    .bind(DbTime::now())
    .execute(db)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Paris

pub async fn balance(db: &SqlitePool, guild_id: &str, user_id: &str) -> Result<i64> {
    let row: Option<(i64,)> =
        sqlx::query_as(r#"SELECT "balance" FROM "MadridWallet" WHERE "guildId" = ? AND "userId" = ?"#)
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(db)
            .await?;
    Ok(row.map_or(STARTING_BALANCE, |(balance,)| balance))
}

#[derive(Clone, Debug, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Bet {
    pub user_id: String,
    pub issue: String,
    pub mise: i64,
    pub gain: Option<i64>,
}

impl Bet {
    pub fn outcome(&self) -> Outcome {
        Outcome::parse(&self.issue).unwrap_or(Outcome::Victoire)
    }
}

pub async fn bets(db: &SqlitePool, match_id: &str) -> Result<Vec<Bet>> {
    Ok(sqlx::query_as(
        r#"SELECT "userId", "issue", "mise", "gain" FROM "MadridBet" WHERE "matchId" = ? ORDER BY "mise" DESC"#,
    )
    .bind(match_id)
    .fetch_all(db)
    .await?)
}

/// Place (ou remplace) un pari : l'ancienne mise est rendue, la nouvelle débitée.
/// `Err` lisible si la cagnotte ne suffit pas.
pub async fn place_bet(
    db: &SqlitePool,
    guild_id: &str,
    match_id: &str,
    user_id: &str,
    outcome: Outcome,
    mise: i64,
) -> Result<Result<i64, String>> {
    let mut tx = db.begin().await?;
    let previous: Option<(i64,)> =
        sqlx::query_as(r#"SELECT "mise" FROM "MadridBet" WHERE "matchId" = ? AND "userId" = ?"#)
            .bind(match_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
    let wallet: Option<(i64,)> =
        sqlx::query_as(r#"SELECT "balance" FROM "MadridWallet" WHERE "guildId" = ? AND "userId" = ?"#)
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
    let available = wallet.map_or(STARTING_BALANCE, |(b,)| b) + previous.map_or(0, |(m,)| m);
    if mise > available {
        return Ok(Err(format!(
            "Cagnotte insuffisante : **{} $** disponibles.",
            crate::util::format::format_price(available as f64)
        )));
    }
    let remaining = available - mise;
    sqlx::query(
        r#"INSERT INTO "MadridWallet" ("guildId", "userId", "balance") VALUES (?, ?, ?)
           ON CONFLICT ("guildId", "userId") DO UPDATE SET "balance" = excluded."balance""#,
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(remaining)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"INSERT INTO "MadridBet" ("matchId", "userId", "issue", "mise") VALUES (?, ?, ?, ?)
           ON CONFLICT ("matchId", "userId") DO UPDATE SET "issue" = excluded."issue", "mise" = excluded."mise""#,
    )
    .bind(match_id)
    .bind(user_id)
    .bind(outcome.as_str())
    .bind(mise)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok(remaining))
}

// ---------------------------------------------------------------------------
// Résultat

/// Règle le match : score, buteurs, points des pronostics et gains des paris, en une transaction.
pub async fn settle(db: &SqlitePool, game: &Match, madrid: i64, adverse: i64, buteurs: Option<&str>) -> Result<()> {
    let pronos = pronos(db, &game.id).await?;
    let bets = bets(db, &game.id).await?;
    let gains = payouts(&bets.iter().map(|b| (b.outcome(), b.mise)).collect::<Vec<_>>(), Outcome::of(madrid, adverse));
    let mut tx = db.begin().await?;
    sqlx::query(
        r#"UPDATE "MadridMatch" SET "scoreMadrid" = ?, "scoreAdverse" = ?, "liveMadrid" = ?, "liveAdverse" = ?,
           "buteurs" = ?, "settledAt" = ? WHERE "id" = ?"#,
    )
    .bind(madrid)
    .bind(adverse)
    .bind(madrid)
    .bind(adverse)
    .bind(buteurs)
    .bind(DbTime::now())
    .bind(&game.id)
    .execute(&mut *tx)
    .await?;
    for prono in &pronos {
        sqlx::query(r#"UPDATE "MadridProno" SET "points" = ? WHERE "matchId" = ? AND "userId" = ?"#)
            .bind(points(prono, madrid, adverse, buteurs))
            .bind(&game.id)
            .bind(&prono.user_id)
            .execute(&mut *tx)
            .await?;
    }
    for (bet, gain) in bets.iter().zip(gains) {
        sqlx::query(r#"UPDATE "MadridBet" SET "gain" = ? WHERE "matchId" = ? AND "userId" = ?"#)
            .bind(gain)
            .bind(&game.id)
            .bind(&bet.user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(r#"UPDATE "MadridWallet" SET "balance" = "balance" + ? WHERE "guildId" = ? AND "userId" = ?"#)
            .bind(gain)
            .bind(&game.guild_id)
            .bind(&bet.user_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Ligne du classement de la saison.
#[derive(Clone, Debug, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Standing {
    pub user_id: String,
    pub points: i64,
    pub exact: i64,
    pub scorers: i64,
    /// Pronostics d'une victoire de l'adversaire qui ne s'est pas produite.
    pub optimist: i64,
    pub played: i64,
}

/// Classement de la saison, avec de quoi décerner les badges.
pub async fn standings(db: &SqlitePool, guild_id: &str) -> Result<Vec<Standing>> {
    Ok(sqlx::query_as(
        r#"SELECT p."userId" AS "userId", SUM(p."points") AS "points",
                  SUM(p."madrid" = m."scoreMadrid" AND p."adverse" = m."scoreAdverse") AS "exact",
                  SUM(p."points" IN (2, 4)) AS "scorers",
                  SUM(p."adverse" > p."madrid" AND m."scoreAdverse" <= m."scoreMadrid") AS "optimist",
                  COUNT(*) AS "played"
           FROM "MadridProno" p JOIN "MadridMatch" m ON m."id" = p."matchId"
           WHERE m."guildId" = ? AND p."points" IS NOT NULL
           GROUP BY p."userId" ORDER BY 2 DESC, 3 DESC, 6 ASC"#,
    )
    .bind(guild_id)
    .fetch_all(db)
    .await?)
}

// ---------------------------------------------------------------------------
// Homme du match

#[derive(Clone, Debug, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Motm {
    pub joueurs: String,
    pub channel_id: String,
    pub message_id: Option<String>,
    pub closes_at: DbTime,
}

impl Motm {
    pub fn players(&self) -> Vec<String> {
        split_players(&self.joueurs)
    }
}

/// Joueurs saisis, séparés par des virgules : 2 à 10, sans doublon.
pub fn split_players(raw: &str) -> Vec<String> {
    let mut players: Vec<String> = Vec::new();
    for name in raw.split([',', ';', '\n']).map(str::trim).filter(|n| !n.is_empty()) {
        let name: String = name.chars().take(MAX_SCORER).collect();
        if !players.iter().any(|p| fold(p) == fold(&name)) {
            players.push(name);
        }
    }
    players.truncate(10);
    players
}

pub async fn motm(db: &SqlitePool, match_id: &str) -> Result<Option<Motm>> {
    Ok(sqlx::query_as(
        r#"SELECT "joueurs", "channelId", "messageId", "closesAt" FROM "MadridMotm" WHERE "matchId" = ?"#,
    )
    .bind(match_id)
    .fetch_optional(db)
    .await?)
}

pub async fn open_motm(
    db: &SqlitePool,
    match_id: &str,
    joueurs: &str,
    channel_id: &str,
    closes_at: DbTime,
) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO "MadridMotm" ("matchId", "joueurs", "channelId", "closesAt") VALUES (?, ?, ?, ?)
           ON CONFLICT ("matchId") DO UPDATE SET "joueurs" = excluded."joueurs", "channelId" = excluded."channelId",
           "messageId" = NULL, "closesAt" = excluded."closesAt""#,
    )
    .bind(match_id)
    .bind(joueurs)
    .bind(channel_id)
    .bind(closes_at)
    .execute(db)
    .await?;
    sqlx::query(r#"DELETE FROM "MadridMotmVote" WHERE "matchId" = ?"#).bind(match_id).execute(db).await?;
    Ok(())
}

pub async fn set_motm_message(db: &SqlitePool, match_id: &str, message_id: &str) -> Result<()> {
    sqlx::query(r#"UPDATE "MadridMotm" SET "messageId" = ? WHERE "matchId" = ?"#)
        .bind(message_id)
        .bind(match_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn vote_motm(db: &SqlitePool, match_id: &str, user_id: &str, joueur: i64) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO "MadridMotmVote" ("matchId", "userId", "joueur") VALUES (?, ?, ?)
           ON CONFLICT ("matchId", "userId") DO UPDATE SET "joueur" = excluded."joueur""#,
    )
    .bind(match_id)
    .bind(user_id)
    .bind(joueur)
    .execute(db)
    .await?;
    Ok(())
}

/// Voix par joueur (index dans la liste).
pub async fn motm_tally(db: &SqlitePool, match_id: &str, players: usize) -> Result<Vec<i64>> {
    let rows: Vec<(i64, i64)> =
        sqlx::query_as(r#"SELECT "joueur", COUNT(*) FROM "MadridMotmVote" WHERE "matchId" = ? GROUP BY "joueur""#)
            .bind(match_id)
            .fetch_all(db)
            .await?;
    let mut tally = vec![0; players];
    for (index, count) in rows {
        if let Some(slot) = usize::try_from(index).ok().and_then(|i| tally.get_mut(i)) {
            *slot = count;
        }
    }
    Ok(tally)
}

// ---------------------------------------------------------------------------
// Rôle clown et réglages

pub async fn add_clown(db: &SqlitePool, guild_id: &str, user_id: &str, role_id: &str, until: DbTime) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO "MadridClown" ("guildId", "userId", "roleId", "until") VALUES (?, ?, ?, ?)
           ON CONFLICT ("guildId", "userId", "roleId") DO UPDATE SET "until" = excluded."until""#,
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(role_id)
    .bind(until)
    .execute(db)
    .await?;
    Ok(())
}

/// Rôles clown arrivés à échéance : (serveur, membre, rôle).
pub async fn expired_clowns(db: &SqlitePool) -> Result<Vec<(String, String, String)>> {
    Ok(sqlx::query_as(r#"SELECT "guildId", "userId", "roleId" FROM "MadridClown" WHERE "until" <= ?"#)
        .bind(DbTime::now())
        .fetch_all(db)
        .await?)
}

pub async fn remove_clown(db: &SqlitePool, guild_id: &str, user_id: &str, role_id: &str) -> Result<()> {
    sqlx::query(r#"DELETE FROM "MadridClown" WHERE "guildId" = ? AND "userId" = ? AND "roleId" = ?"#)
        .bind(guild_id)
        .bind(user_id)
        .bind(role_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn reactions_enabled(db: &SqlitePool, guild_id: &str) -> Result<bool> {
    let row: Option<(bool,)> = sqlx::query_as(r#"SELECT "reactions" FROM "MadridConfig" WHERE "guildId" = ?"#)
        .bind(guild_id)
        .fetch_optional(db)
        .await?;
    Ok(row.is_some_and(|(on,)| on))
}

pub async fn set_reactions(db: &SqlitePool, guild_id: &str, on: bool) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO "MadridConfig" ("guildId", "reactions") VALUES (?, ?)
           ON CONFLICT ("guildId") DO UPDATE SET "reactions" = excluded."reactions""#,
    )
    .bind(guild_id)
    .bind(on)
    .execute(db)
    .await?;
    Ok(())
}

/// Mode automatique d'un serveur : salon où publier les matchs, rôle clown du résultat.
#[derive(Clone, Debug, Default, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Auto {
    pub auto_channel_id: Option<String>,
    pub clown_role_id: Option<String>,
    pub madridista_role_id: Option<String>,
}

pub async fn auto(db: &SqlitePool, guild_id: &str) -> Result<Auto> {
    let row: Option<Auto> = sqlx::query_as(
        r#"SELECT "autoChannelId", "clownRoleId", "madridistaRoleId" FROM "MadridConfig" WHERE "guildId" = ?"#,
    )
    .bind(guild_id)
    .fetch_optional(db)
    .await?;
    Ok(row.unwrap_or_default())
}

pub async fn set_auto(db: &SqlitePool, guild_id: &str, auto: &Auto) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO "MadridConfig" ("guildId", "autoChannelId", "clownRoleId", "madridistaRoleId") VALUES (?, ?, ?, ?)
           ON CONFLICT ("guildId") DO UPDATE SET "autoChannelId" = excluded."autoChannelId",
           "clownRoleId" = excluded."clownRoleId", "madridistaRoleId" = excluded."madridistaRoleId""#,
    )
    .bind(guild_id)
    .bind(&auto.auto_channel_id)
    .bind(&auto.clown_role_id)
    .bind(&auto.madridista_role_id)
    .execute(db)
    .await?;
    Ok(())
}

/// Serveurs en mode automatique : (serveur, salon).
pub async fn auto_guilds(db: &SqlitePool) -> Result<Vec<(String, String)>> {
    Ok(sqlx::query_as(r#"SELECT "guildId", "autoChannelId" FROM "MadridConfig" WHERE "autoChannelId" IS NOT NULL"#)
        .fetch_all(db)
        .await?)
}

/// Match ouvert automatiquement, déjà relié à l'événement de l'API.
pub async fn create_external(
    db: &SqlitePool,
    guild_id: &str,
    event: &super::espn::Event,
    adversaire: &str,
    channel_id: &str,
) -> Result<String> {
    let id = create(db, guild_id, adversaire, DbTime(event.kickoff), channel_id, "auto").await?;
    set_feed(db, &id, &event.id, &event.league, &event.clock, event.opponent_logo.as_deref()).await?;
    Ok(id)
}

// ---------------------------------------------------------------------------
// Paris en direct « prochain but »

/// Camp du prochain but.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NextGoal {
    Real,
    Adverse,
    Aucun,
}

impl NextGoal {
    pub const ALL: [Self; 3] = [Self::Real, Self::Adverse, Self::Aucun];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Real => "REAL",
            Self::Adverse => "ADVERSE",
            Self::Aucun => "AUCUN",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|n| n.as_str() == raw)
    }
}

/// Pari mutuel sur une manche « prochain but » (même règle que les paris d'avant-match).
pub fn next_goal_payouts(bets: &[(NextGoal, i64)], result: NextGoal) -> Vec<i64> {
    let as_outcome = |n: NextGoal| match n {
        NextGoal::Real => Outcome::Victoire,
        NextGoal::Aucun => Outcome::Nul,
        NextGoal::Adverse => Outcome::Defaite,
    };
    payouts(&bets.iter().map(|(n, mise)| (as_outcome(*n), *mise)).collect::<Vec<_>>(), as_outcome(result))
}

#[derive(Clone, Debug, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct LiveBet {
    pub user_id: String,
    pub choice: String,
    pub mise: i64,
}

impl LiveBet {
    pub fn next_goal(&self) -> NextGoal {
        NextGoal::parse(&self.choice).unwrap_or(NextGoal::Aucun)
    }
}

/// Mise sur le prochain but de la manche `round` (buts déjà marqués) : débite la cagnotte.
pub async fn place_live_bet(
    db: &SqlitePool,
    guild_id: &str,
    match_id: &str,
    round: i64,
    user_id: &str,
    choice: NextGoal,
    mise: i64,
) -> Result<Result<i64, String>> {
    let mut tx = db.begin().await?;
    let previous: Option<(i64,)> = sqlx::query_as(
        r#"SELECT "mise" FROM "MadridLiveBet" WHERE "matchId" = ? AND "round" = ? AND "userId" = ? AND "gain" IS NULL"#,
    )
    .bind(match_id)
    .bind(round)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let wallet: Option<(i64,)> =
        sqlx::query_as(r#"SELECT "balance" FROM "MadridWallet" WHERE "guildId" = ? AND "userId" = ?"#)
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
    let available = wallet.map_or(STARTING_BALANCE, |(b,)| b) + previous.map_or(0, |(m,)| m);
    if mise > available {
        return Ok(Err(format!(
            "Cagnotte insuffisante : **{} $** disponibles.",
            crate::util::format::format_price(available as f64)
        )));
    }
    sqlx::query(
        r#"INSERT INTO "MadridWallet" ("guildId", "userId", "balance") VALUES (?, ?, ?)
           ON CONFLICT ("guildId", "userId") DO UPDATE SET "balance" = excluded."balance""#,
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(available - mise)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"INSERT INTO "MadridLiveBet" ("matchId", "round", "userId", "choice", "mise") VALUES (?, ?, ?, ?, ?)
           ON CONFLICT ("matchId", "round", "userId") DO UPDATE SET "choice" = excluded."choice", "mise" = excluded."mise""#,
    )
    .bind(match_id)
    .bind(round)
    .bind(user_id)
    .bind(choice.as_str())
    .bind(mise)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok(available - mise))
}

/// Paris non réglés d'une manche.
pub async fn open_live_bets(db: &SqlitePool, match_id: &str, round: i64) -> Result<Vec<LiveBet>> {
    Ok(sqlx::query_as(
        r#"SELECT "userId", "choice", "mise" FROM "MadridLiveBet" WHERE "matchId" = ? AND "round" = ? AND "gain" IS NULL"#,
    )
    .bind(match_id)
    .bind(round)
    .fetch_all(db)
    .await?)
}

/// Manches encore ouvertes (normalement une seule : celle du score actuel).
pub async fn open_live_rounds(db: &SqlitePool, match_id: &str) -> Result<Vec<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        r#"SELECT DISTINCT "round" FROM "MadridLiveBet" WHERE "matchId" = ? AND "gain" IS NULL ORDER BY "round""#,
    )
    .bind(match_id)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|(round,)| round).collect())
}

/// Règle une manche : gains versés, renvoie (membre, mise, gain) pour l'annonce.
pub async fn settle_live_round(
    db: &SqlitePool,
    guild_id: &str,
    match_id: &str,
    round: i64,
    result: NextGoal,
) -> Result<Vec<(String, i64, i64)>> {
    let bets = open_live_bets(db, match_id, round).await?;
    let gains = next_goal_payouts(&bets.iter().map(|b| (b.next_goal(), b.mise)).collect::<Vec<_>>(), result);
    let mut tx = db.begin().await?;
    let mut settled = Vec::new();
    for (bet, gain) in bets.iter().zip(gains) {
        sqlx::query(r#"UPDATE "MadridLiveBet" SET "gain" = ? WHERE "matchId" = ? AND "round" = ? AND "userId" = ?"#)
            .bind(gain)
            .bind(match_id)
            .bind(round)
            .bind(&bet.user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(r#"UPDATE "MadridWallet" SET "balance" = "balance" + ? WHERE "guildId" = ? AND "userId" = ?"#)
            .bind(gain)
            .bind(guild_id)
            .bind(&bet.user_id)
            .execute(&mut *tx)
            .await?;
        settled.push((bet.user_id.clone(), bet.mise, gain));
    }
    tx.commit().await?;
    Ok(settled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prono(madrid: i64, adverse: i64, buteur: Option<&str>) -> Prono {
        Prono { user_id: format!("{madrid}-{adverse}"), madrid, adverse, buteur: buteur.map(Into::into), points: None }
    }

    #[test]
    fn scores_exact_outcome_and_scorer() {
        assert_eq!(points(&prono(3, 1, None), 3, 1, None), 3);
        assert_eq!(points(&prono(2, 0, None), 3, 1, None), 1);
        assert_eq!(points(&prono(1, 1, None), 2, 2, None), 1);
        assert_eq!(points(&prono(0, 2, None), 3, 1, None), 0);
        assert_eq!(points(&prono(3, 1, Some("Mbappé")), 3, 1, Some("Kylian Mbappe, Vinicius Jr")), 4);
        assert_eq!(points(&prono(0, 1, Some("vinicius jr")), 3, 1, Some("Vinícius Jr")), 1);
        assert_eq!(points(&prono(3, 1, Some("Bellingham")), 3, 1, Some("")), 3);
    }

    #[test]
    fn scorer_matching_is_forgiving() {
        assert!(scorer_matches("Bellingham", "Jude Bellingham"));
        assert!(scorer_matches("jude bellingham", "Bellingham"));
        assert!(scorer_matches(" Mbappé ", "Mbappe;Rodrygo"));
        assert!(!scorer_matches("", "Mbappe"));
        assert!(!scorer_matches("Rodrygo", "Mbappe, Vinicius"));
    }

    #[test]
    fn tendencies_and_worst_prono() {
        assert_eq!(tendencies(&[]), None);
        let pronos = [prono(2, 1, None), prono(3, 0, None), prono(1, 1, None), prono(0, 4, None)];
        assert_eq!(tendencies(&pronos), Some([50, 25, 25]));
        assert_eq!(worst(&pronos, 3, 1).map(|p| p.user_id.as_str()), Some("0-4"));
        assert!(worst(&[], 1, 0).is_none());
    }

    #[test]
    fn mutual_bets_pay_winners_from_losers() {
        use Outcome::*;
        let bets = [(Victoire, 1000), (Victoire, 3000), (Defaite, 4000), (Nul, 2000)];
        assert_eq!(payouts(&bets, Victoire), vec![2500, 7500, 0, 0]);
        assert_eq!(payouts(&bets, Defaite), vec![0, 0, 10_000, 0]);
        // Personne n'a trouvé : mises rendues.
        assert_eq!(payouts(&[(Victoire, 500), (Nul, 200)], Defaite), vec![500, 200]);
        assert_eq!(odds(&bets, Victoire), Some(2.5));
        assert_eq!(odds(&[(Victoire, 100)], Nul), None);
    }

    #[test]
    fn next_goal_bets_follow_the_same_rule() {
        let bets = [(NextGoal::Real, 1000), (NextGoal::Adverse, 1000), (NextGoal::Aucun, 2000)];
        assert_eq!(next_goal_payouts(&bets, NextGoal::Real), vec![4000, 0, 0]);
        assert_eq!(next_goal_payouts(&[(NextGoal::Real, 500)], NextGoal::Adverse), vec![500]);
        assert_eq!(NextGoal::parse("AUCUN"), Some(NextGoal::Aucun));
    }

    #[test]
    fn players_are_trimmed_and_deduplicated() {
        assert_eq!(split_players(" Vinícius, Mbappé ,vinicius,, Bellingham"), vec!["Vinícius", "Mbappé", "Bellingham"]);
        assert_eq!(split_players(&(0..15).map(|i| format!("J{i}")).collect::<Vec<_>>().join(",")).len(), 10);
    }
}
