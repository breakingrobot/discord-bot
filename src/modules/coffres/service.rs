//! Mouvements des coffres, stock calculé, noms des propriétés et liens joueur → membre.

use super::parser::{Kind, Movement};
use crate::util::time::DbTime;
use anyhow::Result;
use regex::Regex;
use sqlx::{FromRow, SqlitePool};
use std::sync::LazyLock;

/// Mouvement à enregistrer : identifiant « message:rang », contenu, date.
pub struct NewMovement {
    pub id: String,
    pub movement: Movement,
    pub at: DbTime,
}

/// Mouvement réellement ajouté (pas déjà connu), avec le stock de son objet
/// dans sa propriété juste avant et juste après : de quoi détecter un passage
/// sous un seuil.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
    pub movement: Movement,
    pub at: DbTime,
    pub stock_before: i64,
    pub stock_after: i64,
}

/// Enregistre des mouvements d'un salon en une transaction, met le stock à jour
/// et avance le dernier message lu. Renvoie les nouveaux mouvements, dans
/// l'ordre (un message relu n'est jamais compté deux fois).
pub async fn insert_all(
    db: &SqlitePool,
    guild_id: &str,
    channel_id: &str,
    movements: &[NewMovement],
    last_message_id: Option<u64>,
) -> Result<Vec<Recorded>> {
    let mut tx = db.begin().await?;
    let mut added = Vec::new();
    for new in movements {
        let movement = &new.movement;
        let result = sqlx::query(
            r#"INSERT OR IGNORE INTO "CoffreMovement"
               ("id", "guildId", "source", "propertyId", "chest", "item", "amount", "kind", "playerName", "playerGameId", "at", "channelId")
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&new.id)
        .bind(guild_id)
        .bind(&movement.source)
        .bind(&movement.property_id)
        .bind(movement.chest)
        .bind(&movement.item)
        .bind(movement.amount)
        .bind(movement.kind.as_str())
        .bind(&movement.player_name)
        .bind(&movement.player_game_id)
        .bind(new.at)
        .bind(channel_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            continue;
        }
        let before: Option<(i64,)> = sqlx::query_as(
            r#"SELECT "deposited" - "withdrawn" + "adjusted" FROM "CoffreStock" WHERE "guildId" = ? AND "propertyId" = ? AND "item" = ?"#,
        )
        .bind(guild_id)
        .bind(&movement.property_id)
        .bind(&movement.item)
        .fetch_optional(&mut *tx)
        .await?;
        let stock_before = before.map_or(0, |(quantity,)| quantity);
        let stock_after = match movement.kind {
            Kind::Deposit | Kind::Adjust => stock_before + movement.amount,
            Kind::Withdraw => stock_before - movement.amount,
        };
        added.push(Recorded { movement: movement.clone(), at: new.at, stock_before, stock_after });
        let (deposited, withdrawn) = match movement.kind {
            Kind::Deposit => (movement.amount, 0),
            Kind::Withdraw => (0, movement.amount),
            // Le webhook ne produit jamais de correction (voir `parser::parse`).
            Kind::Adjust => (0, 0),
        };
        sqlx::query(
            r#"INSERT INTO "CoffreStock" ("guildId", "propertyId", "item", "deposited", "withdrawn") VALUES (?, ?, ?, ?, ?)
               ON CONFLICT ("guildId", "propertyId", "item") DO UPDATE SET
                 "deposited" = "deposited" + excluded."deposited", "withdrawn" = "withdrawn" + excluded."withdrawn""#,
        )
        .bind(guild_id)
        .bind(&movement.property_id)
        .bind(&movement.item)
        .bind(deposited)
        .bind(withdrawn)
        .execute(&mut *tx)
        .await?;
    }
    if let Some(message_id) = last_message_id {
        sqlx::query(
            r#"UPDATE "CoffreChannel" SET "lastMessageId" = ?2
               WHERE "channelId" = ?1 AND ("lastMessageId" IS NULL OR CAST("lastMessageId" AS INTEGER) < ?3)"#,
        )
        .bind(channel_id)
        .bind(message_id.to_string())
        .bind(message_id as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(added)
}

#[derive(Clone, Debug, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct MovementRow {
    pub property_id: String,
    pub chest: Option<i64>,
    pub item: String,
    pub amount: i64,
    pub kind: String,
    pub player_name: String,
    pub player_game_id: Option<String>,
    pub at: DbTime,
}

impl MovementRow {
    pub fn kind(&self) -> Kind {
        Kind::parse(&self.kind).unwrap_or(Kind::Deposit)
    }
}

/// Filtre commun aux commandes et au site.
#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub property_id: Option<String>,
    pub item: Option<String>,
    pub player_game_ids: Option<Vec<String>>,
    pub kind: Option<Kind>,
    pub since: Option<DbTime>,
}

impl Filter {
    fn sql(&self) -> (String, Vec<String>) {
        let mut clauses = vec![r#""guildId" = ?"#.to_owned()];
        let mut binds = Vec::new();
        if let Some(property) = &self.property_id {
            clauses.push(r#""propertyId" = ?"#.to_owned());
            binds.push(property.clone());
        }
        if let Some(item) = &self.item {
            clauses.push(r#""item" = ?"#.to_owned());
            binds.push(item.clone());
        }
        if let Some(players) = &self.player_game_ids {
            if players.is_empty() {
                clauses.push("0".to_owned());
            } else {
                clauses.push(format!(r#""playerGameId" IN ({})"#, vec!["?"; players.len()].join(", ")));
                binds.extend(players.iter().cloned());
            }
        }
        if let Some(kind) = self.kind {
            clauses.push(r#""kind" = ?"#.to_owned());
            binds.push(kind.as_str().to_owned());
        }
        if let Some(since) = self.since {
            clauses.push(r#""at" >= ?"#.to_owned());
            binds.push(since.to_db_string());
        }
        (clauses.join(" AND "), binds)
    }
}

pub async fn movements(
    db: &SqlitePool,
    guild_id: &str,
    filter: &Filter,
    limit: i64,
    offset: i64,
) -> Result<Vec<MovementRow>> {
    let (clause, binds) = filter.sql();
    let sql = format!(
        r#"SELECT "propertyId", "chest", "item", "amount", "kind", "playerName", "playerGameId", "at"
           FROM "CoffreMovement" WHERE {clause} ORDER BY "at" DESC, "id" DESC LIMIT ? OFFSET ?"#
    );
    let mut query = sqlx::query_as::<_, MovementRow>(&sql).bind(guild_id);
    for bind in &binds {
        query = query.bind(bind);
    }
    Ok(query.bind(limit).bind(offset).fetch_all(db).await?)
}

pub async fn count(db: &SqlitePool, guild_id: &str, filter: &Filter) -> Result<i64> {
    let (clause, binds) = filter.sql();
    let sql = format!(r#"SELECT COUNT(*) FROM "CoffreMovement" WHERE {clause}"#);
    let mut query = sqlx::query_as::<_, (i64,)>(&sql).bind(guild_id);
    for bind in &binds {
        query = query.bind(bind);
    }
    Ok(query.fetch_one(db).await?.0)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StockLine {
    pub property_id: String,
    pub item: String,
    pub deposited: i64,
    pub withdrawn: i64,
    /// Somme des corrections d'inventaire (signée).
    pub adjusted: i64,
}

impl StockLine {
    /// Solde connu : dépôts − retraits + corrections d'inventaire.
    pub fn quantity(&self) -> i64 {
        self.deposited - self.withdrawn + self.adjusted
    }
}

/// Stock par propriété et par objet, d'après les mouvements enregistrés.
pub async fn stock(db: &SqlitePool, guild_id: &str, property_id: Option<&str>) -> Result<Vec<StockLine>> {
    let sql = format!(
        r#"SELECT "propertyId", "item", "deposited", "withdrawn", "adjusted" FROM "CoffreStock" WHERE "guildId" = ?{}
           ORDER BY "propertyId", "item""#,
        if property_id.is_some() { r#" AND "propertyId" = ?"# } else { "" }
    );
    let mut query = sqlx::query_as::<_, (String, String, i64, i64, i64)>(&sql).bind(guild_id);
    if let Some(property_id) = property_id {
        query = query.bind(property_id);
    }
    Ok(query
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|(property_id, item, deposited, withdrawn, adjusted)| StockLine {
            property_id,
            item,
            deposited,
            withdrawn,
            adjusted,
        })
        .collect())
}

/// Activité par joueur : (identifiant, nom, dépôts, retraits, mouvements).
pub async fn by_player(
    db: &SqlitePool,
    guild_id: &str,
    filter: &Filter,
) -> Result<Vec<(String, String, i64, i64, i64)>> {
    let (clause, binds) = filter.sql();
    let sql = format!(
        r#"SELECT COALESCE("playerGameId", "playerName"), MAX("playerName"),
                  COALESCE(SUM(CASE WHEN "kind" = 'DEPOSIT' THEN "amount" END), 0),
                  COALESCE(SUM(CASE WHEN "kind" = 'WITHDRAW' THEN "amount" END), 0),
                  COUNT(*)
           FROM "CoffreMovement" WHERE {clause} AND "kind" IN ('DEPOSIT', 'WITHDRAW')
           GROUP BY COALESCE("playerGameId", "playerName") ORDER BY COUNT(*) DESC"#
    );
    let mut query = sqlx::query_as::<_, (String, String, i64, i64, i64)>(&sql).bind(guild_id);
    for bind in &binds {
        query = query.bind(bind);
    }
    Ok(query.fetch_all(db).await?)
}

/// Quantité maximale saisie lors d'un inventaire (comme pour un mouvement du jeu).
pub const MAX_INVENTORY: i64 = super::parser::MAX_AMOUNT;
/// Objets au plus par inventaire (comme sur le site).
pub const MAX_INVENTORY_ITEMS: usize = 300;

/// Écart enregistré par un inventaire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Correction {
    pub item: String,
    pub before: i64,
    pub after: i64,
}

impl Correction {
    pub fn delta(&self) -> i64 {
        self.after - self.before
    }
}

/// Inventaire d'une propriété : pour chaque objet, la quantité réellement
/// présente en jeu. L'écart avec le stock connu devient un mouvement de
/// correction (`ADJUST`, quantité signée, auteur) ; un objet déjà juste ne
/// produit rien. Une seule transaction : tout ou rien. Renvoie les écarts.
pub async fn set_inventory(
    db: &SqlitePool,
    guild_id: &str,
    property_id: &str,
    counted: &[(String, i64)],
    author_id: &str,
    author_name: &str,
) -> Result<Vec<Correction>> {
    let now = DbTime::now();
    let mut tx = db.begin().await?;
    let mut corrections = Vec::new();
    for (item, quantity) in counted {
        let before: Option<(i64,)> = sqlx::query_as(
            r#"SELECT "deposited" - "withdrawn" + "adjusted" FROM "CoffreStock"
               WHERE "guildId" = ? AND "propertyId" = ? AND "item" = ?"#,
        )
        .bind(guild_id)
        .bind(property_id)
        .bind(item)
        .fetch_optional(&mut *tx)
        .await?;
        let correction = Correction { item: item.clone(), before: before.map_or(0, |(q,)| q), after: *quantity };
        let delta = correction.delta();
        if delta == 0 {
            continue;
        }
        sqlx::query(
            r#"INSERT INTO "CoffreMovement"
               ("id", "guildId", "source", "propertyId", "chest", "item", "amount", "kind", "playerName", "playerGameId", "at", "channelId", "authorId")
               VALUES (?, ?, 'Inventaire', ?, NULL, ?, ?, 'ADJUST', ?, NULL, ?, '', ?)"#,
        )
        .bind(format!("inv:{}", uuid::Uuid::new_v4()))
        .bind(guild_id)
        .bind(property_id)
        .bind(item)
        .bind(delta)
        .bind(author_name)
        .bind(now)
        .bind(author_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            r#"INSERT INTO "CoffreStock" ("guildId", "propertyId", "item", "adjusted") VALUES (?, ?, ?, ?)
               ON CONFLICT ("guildId", "propertyId", "item") DO UPDATE SET "adjusted" = "adjusted" + excluded."adjusted""#,
        )
        .bind(guild_id)
        .bind(property_id)
        .bind(item)
        .bind(delta)
        .execute(&mut *tx)
        .await?;
        corrections.push(correction);
    }
    tx.commit().await?;
    Ok(corrections)
}

/// Puce ou tiret en début de ligne (`- `, `* `, `• `), ignoré.
static BULLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:[•·▪‣◦]\s*|[-*–—]\s+)").expect("regex valide"));
/// `42x repair_kit`, `42 x repair_kit`, `42×repair_kit`.
static QUANTITY_FIRST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9][0-9\s_]*?)\s*(?:[xX]\s+|×\s*)(\S.*)$").expect("regex valide"));
/// `repair_kit x42`, `repair_kit x 42`, `repair_kit×42`.
static QUANTITY_LAST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*?\S)(?:\s+[xX]|\s*×)\s*([0-9][0-9\s_]*)$").expect("regex valide"));

/// Quantité saisie : entier, espaces (de milliers) et `_` ignorés.
fn read_integer(raw: &str) -> Option<i64> {
    let digits: String = raw.chars().filter(|c| !c.is_whitespace() && *c != '_').collect();
    let unsigned = digits.strip_prefix(['+', '-']).unwrap_or(&digits);
    (!unsigned.is_empty() && unsigned.len() <= 16 && unsigned.bytes().all(|b| b.is_ascii_digit()))
        .then(|| digits.parse().ok())
        .flatten()
}

/// Sépare une entrée en (objet, quantité). Formats, dans l'ordre : `objet:quantité`
/// ou `objet=quantité` ; `42x objet` ; `objet x42` ; à défaut le dernier espace ou
/// la dernière tabulation (`acetone 2790`), ou le premier si la quantité est en tête
/// (`42 acetone`). Même règle que l'interface (`ui/src/lib/coffre-inventory.ts`).
fn split_entry(part: &str) -> Option<(&str, &str)> {
    if let Some(at) = part.find([':', '=']) {
        return Some((&part[..at], &part[at + 1..]));
    }
    if let Some(captures) = QUANTITY_FIRST.captures(part) {
        let (quantity, item) = (captures.get(1)?, captures.get(2)?);
        return Some((item.as_str(), quantity.as_str()));
    }
    if let Some(captures) = QUANTITY_LAST.captures(part) {
        let (item, quantity) = (captures.get(1)?, captures.get(2)?);
        return Some((item.as_str(), quantity.as_str()));
    }
    let (item, quantity) = part.rsplit_once(char::is_whitespace)?;
    if read_integer(quantity).is_none()
        && let Some((first, rest)) = part.split_once(char::is_whitespace)
        && read_integer(first).is_some()
    {
        return Some((rest, first));
    }
    Some((item, quantity))
}

/// Lit une liste d'inventaire, telle que tapée ou collée depuis le jeu : entrées
/// séparées par `,`, `;` ou des retours à la ligne, puces ignorées (formats :
/// [`split_entry`]). Un objet répété garde sa dernière quantité. Renvoie un
/// message clair à la première entrée illisible.
pub fn parse_inventory(raw: &str) -> Result<Vec<(String, i64)>, String> {
    let mut entries: Vec<(String, i64)> = Vec::new();
    for line in raw.split([',', ';', '\n']).map(str::trim).filter(|p| !p.is_empty()) {
        let part = BULLET.replace(line, "");
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (item, quantity) = split_entry(part)
            .map(|(i, q)| (i.trim(), q.trim()))
            .ok_or_else(|| format!("« {part} » : écris `objet:quantité`, par exemple `repair_kit:42`."))?;
        let quantity = read_integer(quantity).ok_or_else(|| format!("« {part} » : quantité illisible."))?;
        let item = normalize_item(item).ok_or_else(|| format!("« {part} » : nom d'objet invalide."))?;
        if !(0..=MAX_INVENTORY).contains(&quantity) {
            return Err(format!("« {part} » : quantité entre 0 et {MAX_INVENTORY}."));
        }
        match entries.iter_mut().find(|(known, _)| *known == item) {
            Some(entry) => entry.1 = quantity,
            None => entries.push((item, quantity)),
        }
    }
    if entries.is_empty() {
        return Err("Indique au moins un objet, par exemple `repair_kit:42`.".to_owned());
    }
    Ok(entries)
}

/// Identifiant d'objet du jeu (`repair_kit`) : minuscules, chiffres, `_`, `-`, `.`.
pub fn normalize_item(raw: &str) -> Option<String> {
    let item = raw.trim().to_lowercase().replace(' ', "_");
    (!item.is_empty() && item.len() <= 64 && item.bytes().all(|b| b.is_ascii_alphanumeric() || b"_-.".contains(&b)))
        .then_some(item)
}

/// Propriétés connues (vues dans les mouvements ou nommées) avec leur nom.
pub async fn properties(db: &SqlitePool, guild_id: &str) -> Result<Vec<(String, Option<String>)>> {
    Ok(sqlx::query_as(
        r#"SELECT p."propertyId", n."name" FROM
             (SELECT DISTINCT "propertyId" FROM "CoffreStock" WHERE "guildId" = ?1
              UNION SELECT "propertyId" FROM "CoffreProperty" WHERE "guildId" = ?1) p
           LEFT JOIN "CoffreProperty" n ON n."guildId" = ?1 AND n."propertyId" = p."propertyId"
           ORDER BY COALESCE(n."name", p."propertyId")"#,
    )
    .bind(guild_id)
    .fetch_all(db)
    .await?)
}

pub async fn name_property(db: &SqlitePool, guild_id: &str, property_id: &str, name: &str) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO "CoffreProperty" ("guildId", "propertyId", "name") VALUES (?, ?, ?)
           ON CONFLICT ("guildId", "propertyId") DO UPDATE SET "name" = excluded."name""#,
    )
    .bind(guild_id)
    .bind(property_id)
    .bind(name)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn link_player(db: &SqlitePool, guild_id: &str, game_id: &str, user_id: &str) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO "CoffrePlayer" ("guildId", "gameId", "userId") VALUES (?, ?, ?)
           ON CONFLICT ("guildId", "gameId") DO UPDATE SET "userId" = excluded."userId""#,
    )
    .bind(guild_id)
    .bind(game_id)
    .bind(user_id)
    .execute(db)
    .await?;
    Ok(())
}

/// Identifiants de jeu reliés à un membre.
pub async fn game_ids_of(db: &SqlitePool, guild_id: &str, user_id: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> =
        sqlx::query_as(r#"SELECT "gameId" FROM "CoffrePlayer" WHERE "guildId" = ? AND "userId" = ?"#)
            .bind(guild_id)
            .bind(user_id)
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Liens joueur → membre du serveur.
pub async fn player_links(db: &SqlitePool, guild_id: &str) -> Result<std::collections::HashMap<String, String>> {
    let rows: Vec<(String, String)> =
        sqlx::query_as(r#"SELECT "gameId", "userId" FROM "CoffrePlayer" WHERE "guildId" = ?"#)
            .bind(guild_id)
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().collect())
}

/// Personnage vu dans les mouvements, pour l'autocomplétion de `/coffre lier`.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct PlayerSeen {
    pub game_id: String,
    pub name: String,
    /// Membre relié, le cas échéant.
    pub user_id: Option<String>,
    pub last_seen: DbTime,
}

/// Personnages vus dans les mouvements (avec un identifiant de jeu).
pub async fn players_seen(db: &SqlitePool, guild_id: &str) -> Result<Vec<PlayerSeen>> {
    Ok(sqlx::query_as(
        r#"SELECT m."playerGameId" AS "gameId", MAX(m."playerName") AS "name", p."userId", MAX(m."at") AS "lastSeen"
           FROM "CoffreMovement" m
           LEFT JOIN "CoffrePlayer" p ON p."guildId" = m."guildId" AND p."gameId" = m."playerGameId"
           WHERE m."guildId" = ? AND m."playerGameId" IS NOT NULL
           GROUP BY m."playerGameId""#,
    )
    .bind(guild_id)
    .fetch_all(db)
    .await?)
}

/// Choix de `/coffre lier id_jeu` : « Nom (id) », les personnages encore
/// reliés à personne d'abord, puis les plus récemment vus.
pub fn player_choices(mut players: Vec<PlayerSeen>, typed: &str) -> Vec<(String, String)> {
    players.sort_by(|a, b| {
        a.user_id
            .is_some()
            .cmp(&b.user_id.is_some())
            .then(b.last_seen.cmp(&a.last_seen))
            .then(a.game_id.cmp(&b.game_id))
    });
    crate::discord::suggest::filter(
        players.into_iter().map(|p| {
            let linked = if p.user_id.is_some() { " · déjà relié" } else { "" };
            (format!("{} ({}){linked}", p.name, p.game_id), p.game_id)
        }),
        typed,
    )
}

/// Objets connus, pour l'autocomplétion.
pub async fn items(db: &SqlitePool, guild_id: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> =
        sqlx::query_as(r#"SELECT DISTINCT "item" FROM "CoffreStock" WHERE "guildId" = ? ORDER BY "item""#)
            .bind(guild_id)
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().map(|(item,)| item).collect())
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Channel {
    pub channel_id: String,
    pub guild_id: String,
    pub last_message_id: Option<String>,
    /// Webhook du jeu retenu pour ce salon (`None` : pas encore vu).
    pub webhook_id: Option<String>,
}

pub async fn channels(db: &SqlitePool, guild_id: &str) -> Result<Vec<Channel>> {
    Ok(sqlx::query_as(r#"SELECT * FROM "CoffreChannel" WHERE "guildId" = ?"#).bind(guild_id).fetch_all(db).await?)
}

pub async fn channel(db: &SqlitePool, channel_id: &str) -> Result<Option<Channel>> {
    Ok(sqlx::query_as(r#"SELECT * FROM "CoffreChannel" WHERE "channelId" = ?"#)
        .bind(channel_id)
        .fetch_optional(db)
        .await?)
}

/// Retient le webhook du jeu pour un salon, s'il n'est pas déjà connu.
pub async fn set_webhook(db: &SqlitePool, channel_id: &str, webhook_id: &str) -> Result<()> {
    sqlx::query(r#"UPDATE "CoffreChannel" SET "webhookId" = ? WHERE "channelId" = ? AND "webhookId" IS NULL"#)
        .bind(webhook_id)
        .bind(channel_id)
        .execute(db)
        .await?;
    Ok(())
}

/// Webhook du jeu reconnu dans un salon.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Webhook {
    pub channel_id: String,
    pub webhook_id: String,
    pub name: Option<String>,
}

/// Webhooks reconnus d'un serveur (tous salons), ou d'un seul salon.
pub async fn webhooks(db: &SqlitePool, guild_id: &str, channel_id: Option<&str>) -> Result<Vec<Webhook>> {
    Ok(sqlx::query_as(
        r#"SELECT "channelId", "webhookId", "name" FROM "CoffreWebhook"
           WHERE "guildId" = ?1 AND (?2 IS NULL OR "channelId" = ?2) ORDER BY "addedAt", "webhookId""#,
    )
    .bind(guild_id)
    .bind(channel_id)
    .fetch_all(db)
    .await?)
}

/// Reconnaît un webhook dans un salon ; `true` s'il était inconnu.
pub async fn add_webhook(
    db: &SqlitePool,
    guild_id: &str,
    channel_id: &str,
    webhook_id: &str,
    name: Option<&str>,
) -> Result<bool> {
    let added = sqlx::query(
        r#"INSERT OR IGNORE INTO "CoffreWebhook" ("channelId", "webhookId", "guildId", "name", "addedAt") VALUES (?, ?, ?, ?, ?)"#,
    )
    .bind(channel_id)
    .bind(webhook_id)
    .bind(guild_id)
    .bind(name)
    .bind(DbTime::now())
    .execute(db)
    .await?
    .rows_affected()
        > 0;
    // Compatibilité : « au moins un webhook reconnu » pour l'état et le site.
    set_webhook(db, channel_id, webhook_id).await?;
    Ok(added)
}

/// Oublie un webhook : ses prochains messages seront ignorés (les mouvements
/// déjà lus restent). Renvoie `false` s'il était inconnu sur ce serveur.
pub async fn remove_webhook(db: &SqlitePool, guild_id: &str, webhook_id: &str) -> Result<bool> {
    let mut tx = db.begin().await?;
    let removed = sqlx::query(r#"DELETE FROM "CoffreWebhook" WHERE "guildId" = ? AND "webhookId" = ?"#)
        .bind(guild_id)
        .bind(webhook_id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
        > 0;
    // L'ancienne colonne suit : premier webhook restant du salon, sinon aucun.
    sqlx::query(
        r#"UPDATE "CoffreChannel" SET "webhookId" = (
             SELECT w."webhookId" FROM "CoffreWebhook" w WHERE w."channelId" = "CoffreChannel"."channelId" ORDER BY w."addedAt" LIMIT 1)
           WHERE "guildId" = ?"#,
    )
    .bind(guild_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(removed)
}

pub async fn add_channel(db: &SqlitePool, guild_id: &str, channel_id: &str) -> Result<()> {
    sqlx::query(r#"INSERT OR IGNORE INTO "CoffreChannel" ("channelId", "guildId") VALUES (?, ?)"#)
        .bind(channel_id)
        .bind(guild_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn remove_channel(db: &SqlitePool, guild_id: &str, channel_id: &str) -> Result<bool> {
    sqlx::query(r#"DELETE FROM "CoffreWebhook" WHERE "guildId" = ? AND "channelId" = ?"#)
        .bind(guild_id)
        .bind(channel_id)
        .execute(db)
        .await?;
    let result = sqlx::query(r#"DELETE FROM "CoffreChannel" WHERE "guildId" = ? AND "channelId" = ?"#)
        .bind(guild_id)
        .bind(channel_id)
        .execute(db)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::coffres::parser::Movement;

    fn movement(item: &str, amount: i64, kind: Kind, player: &str) -> Movement {
        Movement {
            source: "Coffre propriété".into(),
            property_id: "8740".into(),
            chest: Some(1),
            item: item.into(),
            amount,
            kind,
            player_name: format!("Joueur {player}"),
            player_game_id: Some(player.into()),
            at: None,
        }
    }

    #[tokio::test]
    async fn stock_history_and_dedupe() {
        let dir = std::env::temp_dir().join(format!("coffre-test-{}", uuid::Uuid::new_v4()));
        let db = crate::db::open_main(&dir.join("bot.db")).await.unwrap();
        let now = DbTime::now();
        let rows = [
            ("m1:0", movement("repair_kit", 8, Kind::Deposit, "1")),
            ("m2:0", movement("repair_kit", 3, Kind::Withdraw, "2")),
            ("m3:0", movement("lockpick", 2, Kind::Deposit, "2")),
        ];
        add_channel(&db, "g", "c").await.unwrap();
        let batch: Vec<NewMovement> =
            rows.iter().map(|(id, m)| NewMovement { id: (*id).to_owned(), movement: m.clone(), at: now }).collect();
        assert_eq!(insert_all(&db, "g", "c", &batch, Some(200)).await.unwrap().len(), 3);
        // Message relu (rattrapage) : ignoré, le stock ne bouge pas, le curseur ne recule pas.
        assert_eq!(insert_all(&db, "g", "c", &batch[..1], Some(100)).await.unwrap().len(), 0);
        let read = channels(&db, "g").await.unwrap();
        assert_eq!(read[0].last_message_id.as_deref(), Some("200"));
        set_webhook(&db, "c", "w1").await.unwrap();
        set_webhook(&db, "c", "w2").await.unwrap();
        assert_eq!(channel(&db, "c").await.unwrap().unwrap().webhook_id.as_deref(), Some("w1"));

        let lines = stock(&db, "g", None).await.unwrap();
        let kit = lines.iter().find(|l| l.item == "repair_kit").unwrap();
        assert_eq!((kit.deposited, kit.withdrawn, kit.quantity()), (8, 3, 5));

        link_player(&db, "g", "2", "u2").await.unwrap();
        let ids = game_ids_of(&db, "g", "u2").await.unwrap();
        let filter = Filter { player_game_ids: Some(ids), ..Default::default() };
        assert_eq!(count(&db, "g", &filter).await.unwrap(), 2);
        let withdrawals = Filter { kind: Some(Kind::Withdraw), ..Default::default() };
        assert_eq!(movements(&db, "g", &withdrawals, 10, 0).await.unwrap()[0].amount, 3);
        let nobody = Filter { player_game_ids: Some(Vec::new()), ..Default::default() };
        assert_eq!(count(&db, "g", &nobody).await.unwrap(), 0);

        name_property(&db, "g", "8740", "Planque").await.unwrap();
        assert_eq!(properties(&db, "g").await.unwrap(), vec![("8740".to_owned(), Some("Planque".to_owned()))]);

        assert_eq!(items(&db, "g").await.unwrap(), vec!["lockpick".to_owned(), "repair_kit".to_owned()]);
        assert_eq!(stock(&db, "g", Some("autre")).await.unwrap(), Vec::new());

        db.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn inventory_records_only_the_differences() {
        let dir = std::env::temp_dir().join(format!("coffre-inv-{}", uuid::Uuid::new_v4()));
        let db = crate::db::open_main(&dir.join("bot.db")).await.unwrap();
        add_channel(&db, "g", "c").await.unwrap();
        let seen = [NewMovement {
            id: "m1:0".into(),
            movement: movement("repair_kit", 8, Kind::Deposit, "1"),
            at: DbTime::now(),
        }];
        insert_all(&db, "g", "c", &seen, None).await.unwrap();

        // Stock antérieur au suivi : 42 kits en réalité, et de l'acétone jamais vue.
        let counted = parse_inventory("repair_kit:42, acetone 2790; lockpick=0").unwrap();
        let corrections = set_inventory(&db, "g", "8740", &counted, "77", "Gradé").await.unwrap();
        let summary: Vec<(&str, i64, i64)> = corrections.iter().map(|c| (c.item.as_str(), c.before, c.after)).collect();
        assert_eq!(summary, [("repair_kit", 8, 42), ("acetone", 0, 2790)], "un objet déjà juste ne produit rien");

        let lines = stock(&db, "g", Some("8740")).await.unwrap();
        let kit = lines.iter().find(|l| l.item == "repair_kit").unwrap();
        assert_eq!((kit.deposited, kit.withdrawn, kit.adjusted, kit.quantity()), (8, 0, 34, 42));

        // Les mouvements suivants s'ajoutent à l'inventaire.
        let later = [NewMovement {
            id: "m2:0".into(),
            movement: movement("repair_kit", 2, Kind::Withdraw, "1"),
            at: DbTime::now(),
        }];
        let recorded = insert_all(&db, "g", "c", &later, None).await.unwrap();
        assert_eq!((recorded[0].stock_before, recorded[0].stock_after), (42, 40));

        // Correction visible dans l'historique, signée, avec son auteur ; hors activité des joueurs.
        let adjust = Filter { kind: Some(Kind::Adjust), ..Default::default() };
        let rows = movements(&db, "g", &adjust, 10, 0).await.unwrap();
        assert!(
            rows.iter().any(|r| r.item == "repair_kit"
                && r.amount == 34
                && r.kind() == Kind::Adjust
                && r.player_name == "Gradé")
        );
        assert!(by_player(&db, "g", &Filter::default()).await.unwrap().iter().all(|(_, name, ..)| name != "Gradé"));

        // Recompter la même chose ne crée rien.
        assert!(set_inventory(&db, "g", "8740", &[("repair_kit".into(), 40)], "77", "Gradé").await.unwrap().is_empty());
        db.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn parses_inventory_lists() {
        assert_eq!(
            parse_inventory("Repair Kit:42\nacetone = 2 790, repair_kit:41").unwrap(),
            [("repair_kit".to_owned(), 41), ("acetone".to_owned(), 2790)]
        );
        assert!(parse_inventory("").is_err());
        assert!(parse_inventory("repair_kit").is_err());
        assert!(parse_inventory("repair_kit:-1").is_err());
        assert!(parse_inventory("drop table:1").is_ok(), "espaces → _");
        assert!(parse_inventory("kit;--:1").is_err());
    }

    #[test]
    fn parses_pasted_inventories() {
        let pasted = "- repair_kit x42\n• 2790x acetone\n* 650 × red_powder\r\nlockpick\t12\nmedikit×3\n\n— 5 bandage\n· weapon_pistol x 1 200";
        assert_eq!(
            parse_inventory(pasted).unwrap(),
            [
                ("repair_kit".to_owned(), 42),
                ("acetone".to_owned(), 2790),
                ("red_powder".to_owned(), 650),
                ("lockpick".to_owned(), 12),
                ("medikit".to_owned(), 3),
                ("bandage".to_owned(), 5),
                ("weapon_pistol".to_owned(), 1200),
            ]
        );
        // Un nom qui finit par « x » n'est pas pris pour un multiplicateur.
        assert_eq!(parse_inventory("box 4").unwrap(), [("box".to_owned(), 4)]);
        assert_eq!(parse_inventory("42 X Repair Kit").unwrap(), [("repair_kit".to_owned(), 42)]);
        assert_eq!(parse_inventory("4x4_kit 3").unwrap(), [("4x4_kit".to_owned(), 3)]);
        assert_eq!(parse_inventory("x=1").unwrap(), [("x".to_owned(), 1)]);
        assert!(parse_inventory("-1 x kit").is_err(), "le tiret sans espace n'est pas une puce");
        assert_eq!(
            parse_inventory("repair_kit x beaucoup").unwrap_err(),
            "« repair_kit x beaucoup » : quantité illisible."
        );
    }

    #[test]
    fn suggests_unlinked_players_first() {
        let seen = |id: &str, name: &str, user: Option<&str>, hours: i64| PlayerSeen {
            game_id: id.into(),
            name: name.into(),
            user_id: user.map(Into::into),
            last_seen: DbTime(DbTime::now().0 - chrono::Duration::hours(hours)),
        };
        let players = vec![
            seen("1", "Jean Dupont", Some("u1"), 0),
            seen("2", "Marie Curie", None, 5),
            seen("3", "Paul Martin", None, 1),
        ];
        let values: Vec<String> = player_choices(players.clone(), "").into_iter().map(|(_, v)| v).collect();
        assert_eq!(values, ["3", "2", "1"]);
        assert_eq!(
            player_choices(players.clone(), "dupont"),
            [("Jean Dupont (1) · déjà relié".to_owned(), "1".to_owned())]
        );
        assert_eq!(player_choices(players, "2").len(), 1, "par identifiant");
    }

    #[tokio::test]
    async fn lists_players_seen() {
        let dir = std::env::temp_dir().join(format!("coffre-players-{}", uuid::Uuid::new_v4()));
        let db = crate::db::open_main(&dir.join("bot.db")).await.unwrap();
        add_channel(&db, "g", "c").await.unwrap();
        let batch: Vec<NewMovement> = [("m1:0", "1"), ("m2:0", "2"), ("m3:0", "1")]
            .iter()
            .map(|(id, player)| NewMovement {
                id: (*id).to_owned(),
                movement: movement("kit", 1, Kind::Deposit, player),
                at: DbTime::now(),
            })
            .collect();
        insert_all(&db, "g", "c", &batch, None).await.unwrap();
        link_player(&db, "g", "2", "u2").await.unwrap();
        let mut players = players_seen(&db, "g").await.unwrap();
        players.sort_by(|a, b| a.game_id.cmp(&b.game_id));
        let summary: Vec<_> = players.iter().map(|p| (p.game_id.as_str(), p.user_id.as_deref())).collect();
        assert_eq!(summary, [("1", None), ("2", Some("u2"))]);
        db.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn several_webhooks_per_channel() {
        let dir = std::env::temp_dir().join(format!("coffre-wh-{}", uuid::Uuid::new_v4()));
        let db = crate::db::open_main(&dir.join("bot.db")).await.unwrap();
        add_channel(&db, "g", "c").await.unwrap();
        // Deux propriétés, deux webhooks dans le même salon : les deux comptent.
        assert!(add_webhook(&db, "g", "c", "w1", Some("Stock gradé")).await.unwrap());
        assert!(add_webhook(&db, "g", "c", "w2", Some("Stock vide poche")).await.unwrap());
        assert!(!add_webhook(&db, "g", "c", "w2", Some("Stock vide poche")).await.unwrap(), "déjà connu");
        let known: Vec<String> =
            webhooks(&db, "g", Some("c")).await.unwrap().into_iter().map(|w| w.webhook_id).collect();
        assert_eq!(known, ["w1", "w2"]);
        assert_eq!(channel(&db, "c").await.unwrap().unwrap().webhook_id.as_deref(), Some("w1"));

        // Oublier le premier : le second reste, l'ancienne colonne suit.
        assert!(remove_webhook(&db, "g", "w1").await.unwrap());
        assert!(!remove_webhook(&db, "autre", "w2").await.unwrap(), "seulement sur son serveur");
        assert_eq!(channel(&db, "c").await.unwrap().unwrap().webhook_id.as_deref(), Some("w2"));
        assert!(remove_webhook(&db, "g", "w2").await.unwrap());
        assert_eq!(channel(&db, "c").await.unwrap().unwrap().webhook_id, None);

        // Retirer le salon oublie ses webhooks.
        add_webhook(&db, "g", "c", "w3", None).await.unwrap();
        remove_channel(&db, "g", "c").await.unwrap();
        assert!(webhooks(&db, "g", None).await.unwrap().is_empty());
        db.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }
}
