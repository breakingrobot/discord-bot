//! Soirée Clásico : `/halamadrid` (cris, pronostics, paris en argent fictif,
//! résultat, homme du match, classement et badges), `/golazo` (but en direct) et
//! `/putabarca` (chambrage). Jouer : `clasico.jouer` (membres) ; ouvrir un match,
//! annoncer un but, saisir le résultat : `clasico.gerer` (gradés).

pub mod cards;
pub mod espn;
pub mod live;
pub mod service;

use crate::{
    context::Ctx,
    discord::{
        commands::modal_text,
        components::{ButtonExt as _, button, label, row, text_input},
        embed::{EmbedBuilder, footer},
        interaction::{GUILD_ONLY, Ix, Reply},
        messages::{self, Outgoing, parse_id},
        options::Options,
    },
    modules::permissions::{self, Perm},
    util::{
        format::{DESCRIPTION_LIMIT, field, format_price, truncate},
        ids::{CustomId, encode_custom_id},
        time::{self, DbTime},
    },
};
use anyhow::Result;
use rand::seq::IndexedRandom as _;
use service::{MAX_GOALS, MAX_SCORER, Match, Outcome, Prono};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};
use twilight_model::channel::ChannelType;
use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::modal::ModalInteractionData,
    },
    channel::{
        Message,
        message::{
            Component, Embed,
            component::{ButtonStyle, TextInputStyle},
        },
    },
};
use twilight_util::builder::command::{
    BooleanBuilder, ChannelBuilder, CommandBuilder, IntegerBuilder, RoleBuilder, StringBuilder, SubCommandBuilder,
    UserBuilder,
};

pub const NAMESPACE: &str = "clasico";
use cards::{BARCA, MADRID};
const DEFAULT_RIVAL: &str = "FC Barcelone";
const DEFAULT_KICKOFF: &str = "21:00";
/// Durée du rôle clown.
const CLOWN_HOURS: i64 = 24;
/// Délai entre deux réponses automatiques dans un même salon.
const REACTION_COOLDOWN: Duration = Duration::from_secs(60);

const CRIS: &[&str] = &[
    "¡Hala Madrid y nada más! ⚪👑",
    "Ce soir, le Bernabéu tremble. ¡Vamos!",
    "Noventa minuti en el Bernabéu son molto longo. ⏱️",
    "Une remontada ? Chez nous, c'est une tradition.",
    "Le Clásico ne se joue pas, il se gagne. ⚪",
    "Blanc comme le maillot, lourd comme le palmarès. 👑",
    "Que la Casa Blanca se lève ! ¡A por ellos!",
    "Minute 90 : le Real commence à peine à jouer.",
    "¡Cómo no te voy a querer! ⚪",
    "Le Roi d'Europe reçoit, que les visiteurs s'essuient les pieds. 👑",
    "Ce soir on chante, on gagne, et on recommence la semaine prochaine.",
];

const CHAMBRAGES: &[&str] = &[
    "Le Barça, c'est comme le wifi d'un aéroport : ça promet beaucoup, ça lâche au pire moment. 📶",
    "Més que un club ? Ce soir, surtout « més que un but » encaissé. 🥅",
    "Les Blaugrana ont demandé la VAR pour revoir le palmarès européen. Toujours pas trouvé. 🔍",
    "Le tiki-taka, c'est joli. Mais ça marque pas tout seul. 🥱",
    "Ce soir, la seule remontada au programme est madrilène. ⚪",
    "Ils viennent avec le ballon, ils repartiront avec la défaite. Échange équitable. 🤝",
    "Le Barça a réservé l'autocar du retour avant le coup d'envoi. Prudent. 🚌",
    "Rayures bleues et grenat : parfait pour se fondre dans le banc des remplaçants. 🪑",
    "Un Culé m'a dit « on va gagner ». Je rigole encore. 😂",
    "Ce soir, le Clásico se joue à 11 contre 11 et c'est toujours le Real qui gagne à la fin. 👑",
];

const BUTS: &[&str] = &[
    "GOOOOOOOOL ! ⚽🔥",
    "¡GOLAZO! Le Bernabéu explose ! 🏟️",
    "BUT ! Dans la lucarne, comme d'habitude. 🎯",
    "Et c'est au fond ! ¡HALA MADRID! ⚪",
];

const BUTS_ENCAISSES: &[&str] = &[
    "Bon… ils ont marqué. Pas de panique, on a l'habitude des remontadas. 😤",
    "But adverse. Le Real est piqué, et un Real piqué, c'est dangereux. 🐝",
    "Ils ont eu de la chance. On se réveille, ¡vamos! ⏰",
];

fn goals_option(name: &'static str, description: &'static str, required: bool) -> IntegerBuilder {
    IntegerBuilder::new(name, description).required(required).min_value(0).max_value(MAX_GOALS)
}

pub fn halamadrid_definition() -> Command {
    let issues = [("Victoire du Real", "VICTOIRE"), ("Match nul", "NUL"), ("Défaite du Real", "DEFAITE")];
    CommandBuilder::new("halamadrid", "Soirée Clásico : cris, pronostics, paris et classement", CommandType::ChatInput)
        .option(SubCommandBuilder::new("cri", "Lance un cri de supporter madrilène dans le salon"))
        .option(
            SubCommandBuilder::new("match", "Ouvre un match aux pronostics et aux paris (gradés)")
                .option(StringBuilder::new("heure", "Coup d'envoi (défaut : 21:00)"))
                .option(StringBuilder::new("adversaire", "Adversaire (défaut : FC Barcelone)").max_length(40))
                .option(StringBuilder::new("date", "JJ/MM/AAAA (défaut : aujourd'hui)")),
        )
        .option(
            SubCommandBuilder::new("prono", "Ton pronostic, modifiable jusqu'au coup d'envoi")
                .option(goals_option("madrid", "Buts du Real", true))
                .option(goals_option("adverse", "Buts de l'adversaire", true))
                .option(StringBuilder::new("buteur", "Premier buteur madrilène (bonus)").max_length(MAX_SCORER as u16)),
        )
        .option(SubCommandBuilder::new("pronos", "Pronostics et paris du match en cours"))
        .option(
            SubCommandBuilder::new("parier", "Mise des $ fictifs sur l'issue (jusqu'au coup d'envoi)")
                .option(StringBuilder::new("issue", "Sur quoi tu mises").required(true).choices(issues))
                .option(
                    IntegerBuilder::new("mise", "Montant en $ fictifs")
                        .required(true)
                        .min_value(100)
                        .max_value(10_000_000),
                ),
        )
        .option(SubCommandBuilder::new("cagnotte", "Ta cagnotte de $ fictifs et les plus riches"))
        .option(
            SubCommandBuilder::new("resultat", "Score final : points, gains et rôle clown (gradés)")
                .option(goals_option("madrid", "Buts du Real (défaut : score en direct)", false))
                .option(goals_option("adverse", "Buts de l'adversaire (défaut : score en direct)", false))
                .option(StringBuilder::new("buteurs", "Buteurs madrilènes, séparés par des virgules").max_length(300))
                .option(RoleBuilder::new("role_clown", "Rôle donné 24 h à ceux qui ont annoncé l'adversaire gagnant")),
        )
        .option(
            SubCommandBuilder::new("hommedumatch", "Vote de l'homme du match (gradés)")
                .option(
                    StringBuilder::new("joueurs", "2 à 10 joueurs, séparés par des virgules")
                        .required(true)
                        .max_length(400),
                )
                .option(IntegerBuilder::new("minutes", "Durée du vote (défaut : 15)").min_value(1).max_value(120)),
        )
        .option(SubCommandBuilder::new("classement", "Classement des pronostiqueurs et badges"))
        .option(SubCommandBuilder::new("mur", "Le mur de la honte des Culés du dernier match"))
        .option(
            SubCommandBuilder::new("reactions", "Répondre aux « Visca Barça » du serveur (gradés)")
                .option(BooleanBuilder::new("actif", "Activer les réponses automatiques").required(true)),
        )
        .option(
            SubCommandBuilder::new("prochainbut", "Pari en direct : qui marque le prochain but ?")
                .option(StringBuilder::new("camp", "Prochain but").required(true).choices([
                    ("Le Real", "REAL"),
                    ("L'adversaire", "ADVERSE"),
                    ("Plus aucun but", "AUCUN"),
                ]))
                .option(
                    IntegerBuilder::new("mise", "Montant en $ fictifs")
                        .required(true)
                        .min_value(100)
                        .max_value(10_000_000),
                ),
        )
        .option(
            SubCommandBuilder::new("auto", "Mode automatique : matchs du Real suivis en direct (gradés)")
                .option(BooleanBuilder::new("actif", "Ouvrir et suivre les matchs tout seul").required(true))
                .option(
                    ChannelBuilder::new("salon", "Salon des matchs (défaut : celui-ci)")
                        .channel_types([ChannelType::GuildText]),
                )
                .option(RoleBuilder::new("role_clown", "Rôle clown donné au résultat (24 h)"))
                .option(RoleBuilder::new(
                    "role_madridista",
                    "Rôle que chacun prend, pingé au coup d'envoi et aux buts",
                )),
        )
        .build()
}

pub fn golazo_definition() -> Command {
    CommandBuilder::new("golazo", "Annonce un but du match en cours (gradés)", CommandType::ChatInput)
        .option(StringBuilder::new("buteur", "Buteur").max_length(MAX_SCORER as u16))
        .option(BooleanBuilder::new("adverse", "But de l'adversaire (défaut : but du Real)"))
        .build()
}

pub fn putabarca_definition() -> Command {
    CommandBuilder::new("putabarca", "Chambrage de Clásico (bon enfant)", CommandType::ChatInput)
        .option(UserBuilder::new("cule", "Un supporter du Barça à chambrer"))
        .build()
}

fn pick(lines: &'static [&'static str]) -> &'static str {
    lines.choose(&mut rand::rng()).copied().unwrap_or_default()
}

fn money(amount: i64) -> String {
    format!("{} $", format_price(amount as f64))
}

async fn allowed(ix: &Ix, perm: Perm) -> Result<bool> {
    if ix.guild_id().is_none() {
        ix.reply(Reply::ephemeral(GUILD_ONLY)).await?;
        return Ok(false);
    }
    permissions::require(ix, perm).await
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------
// /putabarca et réponses automatiques

pub async fn handle_putabarca(ix: &Ix, options: &Options) -> Result<()> {
    if !allowed(ix, Perm::ClasicoJouer).await? {
        return Ok(());
    }
    // La cible est nommée sans être notifiée : on chambre, on ne harcèle pas.
    let text = match options.user("cule") {
        Some(cule) => format!("<@{cule}>, {}", lowercase_first(pick(CHAMBRAGES))),
        None => pick(CHAMBRAGES).to_owned(),
    };
    let embed = EmbedBuilder::new().color(BARCA).description(text).footer(footer("¡Hala Madrid! ⚪")).build();
    ix.reply(Reply::embed(embed)).await
}

/// Le message encourage-t-il le Barça (sans accents ni majuscules) ?
pub fn is_barca_cheer(content: &str) -> bool {
    let text = service::fold(content);
    ["visca barca", "visca el barca", "forca barca", "força barça", "vamos barca"]
        .iter()
        .any(|cheer| text.contains(&service::fold(cheer)))
}

static LAST_REPLY: LazyLock<Mutex<HashMap<u64, Instant>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Réponse à un « Visca Barça » (serveurs qui l'ont activée), au plus une fois par minute et par salon.
pub async fn on_message(ctx: &Ctx, message: &Message) {
    let Some(guild_id) = message.guild_id else { return };
    if message.author.bot || !is_barca_cheer(&message.content) {
        return;
    }
    if !service::reactions_enabled(&ctx.db, &guild_id.to_string()).await.unwrap_or(false) {
        return;
    }
    {
        let mut last = LAST_REPLY.lock().expect("verrou des réponses");
        let channel = message.channel_id.get();
        if last.get(&channel).is_some_and(|at| at.elapsed() < REACTION_COOLDOWN) {
            return;
        }
        last.insert(channel, Instant::now());
    }
    let reaction = twilight_http::request::channel::reaction::RequestReactionType::Unicode { name: "🤡" };
    let _ = ctx.http.create_reaction(message.channel_id, message.id, &reaction).await;
    let embeds =
        [EmbedBuilder::new().color(BARCA).description(pick(CHAMBRAGES)).footer(footer("¡Hala Madrid! ⚪")).build()];
    if let Err(error) = ctx.http.create_message(message.channel_id).reply(message.id).embeds(&embeds).await {
        tracing::warn!(?error, "clásico : réponse à un « Visca Barça »");
    }
}

// ---------------------------------------------------------------------------
// /halamadrid

pub async fn handle(ix: &Ix, options: &Options) -> Result<()> {
    let perm = if matches!(options.sub(), "match" | "resultat" | "hommedumatch" | "reactions" | "auto") {
        Perm::ClasicoGerer
    } else {
        Perm::ClasicoJouer
    };
    if !allowed(ix, perm).await? {
        return Ok(());
    }
    let guild = ix.guild_id().expect("vérifié").to_string();
    match options.sub() {
        "cri" => cri(ix).await,
        "match" => open_match(ix, options, &guild).await,
        "prono" => {
            let text = submit(
                ix,
                &guild,
                options.integer("madrid").unwrap_or(0),
                options.integer("adverse").unwrap_or(0),
                options.string("buteur").as_deref(),
            )
            .await?;
            ix.reply(Reply::ephemeral(text)).await
        }
        "pronos" => list(ix, &guild).await,
        "parier" => bet(ix, options, &guild).await,
        "cagnotte" => wallet(ix, &guild).await,
        "resultat" => settle(ix, options, &guild).await,
        "hommedumatch" => open_motm(ix, options, &guild).await,
        "classement" => standings(ix, &guild).await,
        "mur" => wall(ix, &guild).await,
        "prochainbut" => next_goal_bet(ix, options, &guild).await,
        "auto" => configure_auto(ix, options, &guild).await,
        "reactions" => {
            let on = options.boolean("actif").unwrap_or(false);
            service::set_reactions(&ix.ctx.db, &guild, on).await?;
            let text = if on {
                "Réponses automatiques activées : chaque « Visca Barça » aura droit à sa vanne (une par minute et par salon). 🤡"
            } else {
                "Réponses automatiques coupées."
            };
            ix.reply(Reply::ephemeral(text)).await
        }
        _ => Ok(()),
    }
}

async fn configure_auto(ix: &Ix, options: &Options, guild: &str) -> Result<()> {
    let on = options.boolean("actif").unwrap_or(false);
    let mut auto = service::auto(&ix.ctx.db, guild).await?;
    auto.auto_channel_id = if on { options.channel("salon").or(ix.channel_id()).map(|c| c.to_string()) } else { None };
    if let Some(role) = options.role("role_clown") {
        auto.clown_role_id = Some(role.to_string());
    }
    if let Some(role) = options.role("role_madridista") {
        auto.madridista_role_id = Some(role.to_string());
    }
    service::set_auto(&ix.ctx.db, guild, &auto).await?;
    let role = |r: &Option<String>| r.as_deref().map_or_else(|| "aucun".to_owned(), |r| format!("<@&{r}>"));
    let text = match &auto.auto_channel_id {
        Some(channel) => format!(
            "⚙️ **Mode automatique activé** dans <#{channel}>.\n\
             • Chaque match du Real est publié quelques heures avant (pronos et paris ouverts).\n\
             • En direct, dans un fil : coup d'envoi, compos, buts (buteur, minute, photo), rouges, mi-temps et fin, avec les stats.\n\
             • Au coup de sifflet final : points, gains et carte du résultat, sans gradé.\n\
             Rôle clown : {} · rôle Madridista : {}\n-# Données : API publique d'ESPN.",
            role(&auto.clown_role_id),
            role(&auto.madridista_role_id)
        ),
        None => "Mode automatique coupé : `/halamadrid match` et `/golazo` restent disponibles.".to_owned(),
    };
    ix.reply(Reply::ephemeral(text)).await
}

async fn next_goal_bet(ix: &Ix, options: &Options, guild: &str) -> Result<()> {
    let Some(game) = service::current(&ix.ctx.db, guild).await? else {
        return ix.reply(Reply::ephemeral("Aucun match en cours.")).await;
    };
    if game.open(DbTime::now()) {
        return ix
            .reply(Reply::ephemeral(
                "Les paris « prochain but » ouvrent au coup d'envoi. Avant : `/halamadrid parier`.",
            ))
            .await;
    }
    let Some(choice) = options.string("camp").as_deref().and_then(service::NextGoal::parse) else {
        return ix.reply(Reply::ephemeral("Choix inconnu.")).await;
    };
    let mise = options.integer("mise").unwrap_or(0);
    let round = game.live_madrid + game.live_adverse;
    let user = ix.user_id().to_string();
    match service::place_live_bet(&ix.ctx.db, guild, &game.id, round, &user, choice, mise).await? {
        Err(message) => ix.reply(Reply::ephemeral(message)).await,
        Ok(remaining) => {
            let label = match choice {
                service::NextGoal::Real => "le Real marque le prochain but".to_owned(),
                service::NextGoal::Adverse => format!("{} marque le prochain but", game.adversaire),
                service::NextGoal::Aucun => "plus aucun but".to_owned(),
            };
            ix.reply(Reply::ephemeral(format!(
                "🎲 Pari en direct : **{}** sur « {label} » (score actuel {} – {}). Il te reste **{}**.",
                money(mise),
                game.live_madrid,
                game.live_adverse,
                money(remaining)
            )))
            .await
        }
    }
}

/// Prend ou retire le rôle Madridista (bouton de l'affiche).
async fn toggle_madridista(ix: &Ix) -> Result<()> {
    let Some(guild_id) = ix.guild_id() else { return Ok(()) };
    let auto = service::auto(&ix.ctx.db, &guild_id.to_string()).await?;
    let Some(role) = auto.madridista_role_id.as_deref().and_then(parse_id) else {
        return ix
            .reply(Reply::ephemeral(
                "Pas de rôle Madridista sur ce serveur : un gradé le règle avec `/halamadrid auto`.",
            ))
            .await;
    };
    let has = ix.member_roles().contains(&role);
    let result = if has {
        ix.ctx.http.remove_guild_member_role(guild_id, ix.user_id(), role).await.map(drop)
    } else {
        ix.ctx.http.add_guild_member_role(guild_id, ix.user_id(), role).await.map(drop)
    };
    if let Err(error) = result {
        tracing::warn!(?error, "clásico : rôle Madridista refusé (permission « Gérer les rôles » ?)");
        return ix.reply(Reply::ephemeral("Impossible de changer ton rôle : préviens un admin.")).await;
    }
    let text = if has {
        "Rôle Madridista retiré : plus de ping pendant les matchs."
    } else {
        "⚪ Bienvenue chez les Madridistas ! Tu seras pingé au coup d'envoi et à chaque but du Real."
    };
    ix.reply(Reply::ephemeral(text)).await
}

async fn cri(ix: &Ix) -> Result<()> {
    let name = crate::discord::members::display_name(&ix.ctx, ix.guild_id().expect("vérifié"), ix.user_id())
        .unwrap_or_else(|| ix.username());
    let embed = EmbedBuilder::new().color(MADRID).description(format!("📣 **{name}** : {}", pick(CRIS))).build();
    ix.reply(Reply::embed(embed)).await
}

/// Buts saisis dans le formulaire ; `None` si illisible.
fn parse_goals(raw: &str) -> Option<i64> {
    raw.trim().parse::<i64>().ok().filter(|goals| (0..=MAX_GOALS).contains(goals))
}

fn clean_scorer(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim).filter(|s| !s.is_empty()).map(|s| s.chars().take(MAX_SCORER).collect())
}

/// Enregistre le pronostic et rafraîchit le message du match ; renvoie la réponse à afficher.
async fn submit(ix: &Ix, guild: &str, madrid: i64, adverse: i64, buteur: Option<&str>) -> Result<String> {
    let Some(game) = service::current(&ix.ctx.db, guild).await? else {
        return Ok("Aucun match ouvert aux pronostics. Un gradé l'ouvre avec `/halamadrid match`.".into());
    };
    if !game.open(DbTime::now()) {
        return Ok("Trop tard : les pronostics sont fermés depuis le coup d'envoi. ⏱️".into());
    }
    let buteur = clean_scorer(buteur);
    service::set_prono(&ix.ctx.db, &game.id, &ix.user_id().to_string(), madrid, adverse, buteur.as_deref()).await?;
    refresh(&ix.ctx, guild, &game.id).await;
    let scorer = buteur.map(|b| format!(", premier buteur **{b}**")).unwrap_or_default();
    Ok(format!(
        "Pronostic enregistré : **Real Madrid {madrid} – {adverse} {}**{scorer}. Modifiable jusqu'au coup d'envoi <t:{}:R>.",
        game.adversaire,
        game.kickoff.unix()
    ))
}

fn outcome_label(outcome: Outcome, rival: &str) -> String {
    match outcome {
        Outcome::Victoire => "victoire du Real".to_owned(),
        Outcome::Nul => "match nul".to_owned(),
        Outcome::Defaite => format!("victoire de {rival}"),
    }
}

fn tendencies_text(pronos: &[Prono], rival: &str) -> String {
    match service::tendencies(pronos) {
        None => "Aucun pronostic pour l'instant : à toi de jouer !".to_owned(),
        Some([win, draw, loss]) => {
            format!("**{}** pronostic(s) · Real {win} % · nul {draw} % · {rival} {loss} %", pronos.len())
        }
    }
}

fn bets_text(bets: &[(Outcome, i64)], rival: &str) -> String {
    if bets.is_empty() {
        return "Aucun pari : `/halamadrid parier`.".to_owned();
    }
    let pot: i64 = bets.iter().map(|(_, mise)| mise).sum();
    let odds: Vec<String> = Outcome::ALL
        .iter()
        .map(|outcome| {
            let odd = service::odds(bets, *outcome).map_or_else(|| "—".to_owned(), |o| format!("{o:.2}"));
            format!("{} × {odd}", outcome_label(*outcome, rival))
        })
        .collect();
    format!("Cagnotte **{}** · {}", money(pot), odds.join(" · "))
}

/// Tendances en barres : une ligne par issue.
fn tendencies_bars(pronos: &[Prono], rival: &str) -> String {
    match service::tendencies(pronos) {
        None => "Aucun pronostic pour l'instant : à toi de jouer !".to_owned(),
        Some([win, draw, loss]) => format!(
            "`Real    ` {}\n`Nul     ` {}\n`{:<8.8}` {}\n-# {} pronostic(s)",
            cards::bar(win),
            cards::bar(draw),
            rival,
            cards::bar(loss),
            pronos.len()
        ),
    }
}

/// Message du match : affiche, score en direct, pronostics, paris et score final.
fn match_message(game: &Match, pronos: &[Prono], bets: &[(Outcome, i64)], now: DbTime) -> Outgoing {
    let kickoff = game.kickoff;
    let competition = cards::league_label(game.league.as_deref());
    let logo = game.rival_logo.as_deref();
    let mut embed = match game.settled() {
        Some((madrid, adverse)) => {
            let scorers = game.buteurs.as_deref().filter(|b| !b.trim().is_empty()).unwrap_or("—");
            cards::base(competition, MADRID, logo)
                .title("🏁 Terminé")
                .description(format!(
                    "{}\n{}",
                    cards::scoreline(madrid, adverse, &game.adversaire),
                    cards::spoiler(&format!(
                        "{}\n⚽ {scorers}",
                        outcome_label(Outcome::of(madrid, adverse), &game.adversaire)
                    ))
                ))
        }
        None if game.open(now) => cards::base(competition, MADRID, logo)
            .title(format!("Real Madrid – {}", game.adversaire))
            .description(
                "Pronostics et paris ouverts jusqu'au coup d'envoi : **Mon prono** ci-dessous, `/halamadrid parier`.\n-# Score exact 3 pts · bonne issue 1 pt · buteur madrilène annoncé qui marque +1",
            )
            .field(cards::inline("🗓️ Coup d'envoi", cards::when(kickoff))),
        None => {
            let state = match (game.live_status.as_deref(), game.live_clock.as_deref()) {
                (Some("HT"), _) => "⏸️ Mi-temps".to_owned(),
                (Some("FT"), _) => "🏁 Terminé, résultat en cours de validation".to_owned(),
                (_, Some(clock)) if !clock.is_empty() => format!("🔴 EN DIRECT · {clock}"),
                _ => "🔴 EN DIRECT".to_owned(),
            };
            cards::base(competition, MADRID, logo).title(state).description(format!(
                "{}\n🔒 Pronostics et paris fermés · paris en direct : `/halamadrid prochainbut`",
                cards::scoreline(game.live_madrid, game.live_adverse, &game.adversaire)
            ))
        }
    };
    embed = embed
        .field(field("📊 Pronostics", tendencies_bars(pronos, &game.adversaire), false))
        .field(field("💰 Paris ($ fictifs)", bets_text(bets, &game.adversaire), false))
        .footer(footer(cards::FOOTER))
        .timestamp(crate::discord::embed::timestamp(kickoff));
    let cri = || {
        Component::Button(
            button(encode_custom_id(NAMESPACE, "cri", &game.id, None), "Lancer un cri", ButtonStyle::Secondary)
                .with_emoji("📣"),
        )
    };
    let madridista = || {
        Component::Button(
            button(encode_custom_id(NAMESPACE, "madridista", &game.id, None), "Madridista", ButtonStyle::Secondary)
                .with_emoji("⚪"),
        )
    };
    let mut components = Vec::new();
    if game.open(now) {
        components.push(row(vec![
            Component::Button(
                button(encode_custom_id(NAMESPACE, "prono", &game.id, None), "Mon prono", ButtonStyle::Primary)
                    .with_emoji("⚽"),
            ),
            cri(),
            madridista(),
        ]));
    } else if game.settled().is_none() {
        components.push(row(vec![cri(), madridista()]));
    }
    Outgoing::embeds(vec![embed.build()]).with_components(components)
}

async fn bet_pairs(ctx: &Ctx, match_id: &str) -> Result<Vec<(Outcome, i64)>> {
    Ok(service::bets(&ctx.db, match_id).await?.iter().map(|b| (b.outcome(), b.mise)).collect())
}

/// Met à jour le message du match (sans bloquer : un message supprimé n'empêche rien).
pub async fn refresh(ctx: &Ctx, guild: &str, match_id: &str) {
    let result: Result<()> = async {
        let Some(game) = service::get(&ctx.db, guild, match_id).await? else {
            return Ok(());
        };
        let (Some(channel), Some(message)) =
            (parse_id(&game.channel_id), game.message_id.as_deref().and_then(parse_id))
        else {
            return Ok(());
        };
        let pronos = service::pronos(&ctx.db, &game.id).await?;
        let bets = bet_pairs(ctx, &game.id).await?;
        messages::edit(ctx, channel, message, &match_message(&game, &pronos, &bets, DbTime::now())).await?;
        Ok(())
    }
    .await;
    if let Err(error) = result {
        tracing::warn!(guild, ?error, "clásico : message du match introuvable");
    }
}

async fn open_match(ix: &Ix, options: &Options, guild: &str) -> Result<()> {
    let Some(channel_id) = ix.channel_id() else {
        return ix.reply(Reply::ephemeral(GUILD_ONLY)).await;
    };
    if let Some(game) = service::current(&ix.ctx.db, guild).await? {
        return ix
            .reply(Reply::ephemeral(format!(
                "Le match contre **{}** n'est pas encore réglé : saisis d'abord `/halamadrid resultat`.",
                game.adversaire
            )))
            .await;
    }
    let heure = options.string("heure").unwrap_or_else(|| DEFAULT_KICKOFF.to_owned());
    let Some(resolved) = time::resolve_event_date(options.string("date").as_deref(), &heure) else {
        return ix.reply(Reply::ephemeral("Date ou heure invalide. Formats attendus : `JJ/MM/AAAA` et `HH:MM`.")).await;
    };
    let rival = options.string("adversaire").map(|a| a.trim().to_owned()).filter(|a| !a.is_empty());
    let rival = rival.unwrap_or_else(|| DEFAULT_RIVAL.to_owned());
    ix.defer(true).await?;
    let id =
        service::create(&ix.ctx.db, guild, &rival, resolved.date, &channel_id.to_string(), &ix.user_id().to_string())
            .await?;
    let game = service::get(&ix.ctx.db, guild, &id).await?.expect("match créé");
    let sent = messages::send(&ix.ctx, channel_id, &match_message(&game, &[], &[], DbTime::now())).await?;
    service::set_message(&ix.ctx.db, &id, &sent.id.to_string()).await?;
    let rolled = if resolved.rolled_to_tomorrow { " (heure passée : **demain**)" } else { "" };
    ix.edit_text(format!(
        "Match publié : pronostics et paris ouverts jusqu'à <t:{}:t>{rolled}. ¡Vamos! ⚪",
        resolved.date.unix()
    ))
    .await
}

async fn list(ix: &Ix, guild: &str) -> Result<()> {
    let Some(game) = service::current(&ix.ctx.db, guild).await? else {
        return ix.reply(Reply::ephemeral("Aucun match en cours.")).await;
    };
    let pronos = service::pronos(&ix.ctx.db, &game.id).await?;
    let bets = service::bets(&ix.ctx.db, &game.id).await?;
    let me = ix.user_id().to_string();
    let mut text = tendencies_text(&pronos, &game.adversaire);
    if game.open(DbTime::now()) {
        // Avant le coup d'envoi, chacun ne voit que le sien : pas de copie de dernière minute.
        if let Some(mine) = pronos.iter().find(|p| p.user_id == me) {
            text.push_str(&format!("\nTon pronostic : **{} – {}**", mine.madrid, mine.adverse));
        }
        if let Some(mine) = bets.iter().find(|b| b.user_id == me) {
            text.push_str(&format!(
                "\nTon pari : **{}** sur {}",
                money(mine.mise),
                outcome_label(mine.outcome(), &game.adversaire)
            ));
        }
        text.push_str("\n-# Le détail s'affiche au coup d'envoi.");
    } else {
        let lines: Vec<String> = pronos
            .iter()
            .map(|p| {
                let scorer = p.buteur.as_deref().map(|b| format!(" ({b})")).unwrap_or_default();
                let bet = bets
                    .iter()
                    .find(|b| b.user_id == p.user_id)
                    .map(|b| format!(" · 💰 {} sur {}", money(b.mise), outcome_label(b.outcome(), &game.adversaire)))
                    .unwrap_or_default();
                format!("<@{}> **{} – {}**{scorer}{bet}", p.user_id, p.madrid, p.adverse)
            })
            .collect();
        if !lines.is_empty() {
            text.push_str(&format!("\n\n{}", lines.join("\n")));
        }
    }
    let embed = EmbedBuilder::new()
        .color(MADRID)
        .title(format!("⚽ Pronostics · Real Madrid – {}", game.adversaire))
        .description(truncate(&text, DESCRIPTION_LIMIT))
        .build();
    ix.reply(Reply::embed(embed).hidden()).await
}

async fn bet(ix: &Ix, options: &Options, guild: &str) -> Result<()> {
    let Some(game) = service::current(&ix.ctx.db, guild).await? else {
        return ix.reply(Reply::ephemeral("Aucun match ouvert aux paris.")).await;
    };
    if !game.open(DbTime::now()) {
        return ix.reply(Reply::ephemeral("Les paris sont fermés depuis le coup d'envoi. ⏱️")).await;
    }
    let Some(outcome) = options.string("issue").as_deref().and_then(Outcome::parse) else {
        return ix.reply(Reply::ephemeral("Issue inconnue.")).await;
    };
    let mise = options.integer("mise").unwrap_or(0);
    let user = ix.user_id().to_string();
    match service::place_bet(&ix.ctx.db, guild, &game.id, &user, outcome, mise).await? {
        Err(message) => ix.reply(Reply::ephemeral(message)).await,
        Ok(remaining) => {
            refresh(&ix.ctx, guild, &game.id).await;
            let warning = if outcome == Outcome::Defaite {
                "\n-# Parier contre le Real ? Le mur de la honte t'attend. 👀"
            } else {
                ""
            };
            ix.reply(Reply::ephemeral(format!(
                "Pari enregistré : **{}** sur {}. Il te reste **{}**.{warning}",
                money(mise),
                outcome_label(outcome, &game.adversaire),
                money(remaining)
            )))
            .await
        }
    }
}

async fn wallet(ix: &Ix, guild: &str) -> Result<()> {
    let mine = service::balance(&ix.ctx.db, guild, &ix.user_id().to_string()).await?;
    let richest: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT "userId", "balance" FROM "MadridWallet" WHERE "guildId" = ? ORDER BY "balance" DESC LIMIT 5"#,
    )
    .bind(guild)
    .fetch_all(&ix.ctx.db)
    .await?;
    let mut text =
        format!("Ta cagnotte : **{}** (argent fictif, départ à {}).", money(mine), money(service::STARTING_BALANCE));
    if !richest.is_empty() {
        let lines: Vec<String> = richest
            .iter()
            .enumerate()
            .map(|(i, (user, balance))| format!("`{}.` <@{user}> — {}", i + 1, money(*balance)))
            .collect();
        text.push_str(&format!("\n\n💎 **Les plus riches**\n{}", lines.join("\n")));
    }
    let embed = EmbedBuilder::new().color(MADRID).title("💰 Cagnotte du Clásico").description(text).build();
    ix.reply(Reply::embed(embed).hidden()).await
}

/// Détails d'un but : buteur, minute, penalty, contre son camp.
#[derive(Clone, Debug, Default)]
pub struct GoalInfo<'a> {
    pub scorer: Option<&'a str>,
    pub minute: Option<&'a str>,
    pub penalty: bool,
    pub own_goal: bool,
    /// Photo du buteur.
    pub headshot: Option<&'a str>,
}

/// Ligne « ⚽ Mbappé 23' (penalty) ».
fn goal_line(goal: &GoalInfo<'_>) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(scorer) = goal.scorer {
        parts.push(scorer.to_owned());
    }
    if let Some(minute) = goal.minute.filter(|m| !m.is_empty()) {
        parts.push(minute.to_owned());
    }
    if goal.penalty {
        parts.push("(penalty)".to_owned());
    }
    if goal.own_goal {
        parts.push("(contre son camp)".to_owned());
    }
    (!parts.is_empty()).then(|| format!("⚽ {}", parts.join(" ")))
}

/// Annonce un but (commande `/golazo` ou suivi automatique) dans le fil du match :
/// photo du buteur, ping du rôle Madridista pour un but du Real, « qui l'avait annoncé »,
/// puis règlement des paris « prochain but » de la manche.
pub async fn announce_goal(
    ctx: &Ctx,
    game: &Match,
    madrid: i64,
    adverse: i64,
    madrid_scored: bool,
    goal: GoalInfo<'_>,
) {
    let Some(channel) = parse_id(game.live_channel()) else { return };
    // Tout ce qui trahit le match est masqué (spoiler) : titre et couleur neutres,
    // score, camp, buteur, gagnants des paris et photo derrière un clic.
    let scorer = goal.scorer.filter(|_| !goal.own_goal);
    let side = if madrid_scored { "Real Madrid" } else { game.adversaire.as_str() };
    let cheer = if madrid_scored { pick(BUTS) } else { pick(BUTS_ENCAISSES) };
    let detail = goal_line(&goal).map(|line| format!("\n{line}")).unwrap_or_default();
    let mut embed = cards::base("But", cards::NEUTRAL, None)
        .title("⚽ BUT !")
        .description(cards::scoreline(madrid, adverse, &game.adversaire))
        .field(field("Le but", cards::spoiler(&format!("**{side}** — {cheer}{detail}")), false));
    if madrid_scored
        && let Some(scorer) = scorer
        && let Ok(pronos) = service::pronos(&ctx.db, &game.id).await
    {
        let seers: Vec<String> = pronos
            .iter()
            .filter(|p| p.buteur.as_deref().is_some_and(|b| service::scorer_matches(b, scorer)))
            .map(|p| format!("<@{}>", p.user_id))
            .collect();
        if !seers.is_empty() {
            embed = embed.field(field("🔮 L'avaient annoncé", cards::spoiler(&seers.join(", ")), false));
        }
    }
    // Manche « prochain but » : buts marqués avant celui-ci.
    let round = (madrid + adverse - 1).max(0);
    let result = if madrid_scored { service::NextGoal::Real } else { service::NextGoal::Adverse };
    if let Some(line) = settle_live_round(ctx, game, round, result).await {
        embed = embed.field(field("🎲 Prochain but", cards::spoiler(&line), false));
    }
    let embeds = [embed.footer(footer(cards::FOOTER)).build()];
    // Ping Madridista à chaque but, des deux côtés : la notification ne dit pas qui a marqué.
    let role = match service::auto(&ctx.db, &game.guild_id).await {
        Ok(auto) => auto.madridista_role_id.as_deref().and_then(parse_id::<twilight_model::id::marker::RoleMarker>),
        Err(_) => None,
    };
    let content = role.map(|role| format!("<@&{role}> ⚽ But !"));
    let allowed =
        messages::Mentions::Explicit { users: Vec::new(), roles: role.into_iter().collect(), everyone: false }
            .to_allowed();
    // Photo du buteur en pièce jointe masquée (une image d'embed ne peut pas l'être).
    let photo = match goal.headshot {
        Some(url) => spoiler_photo(url).await,
        None => None,
    };
    let uploads: Vec<_> = photo.into_iter().collect();
    let mut request = ctx.http.create_message(channel).embeds(&embeds).allowed_mentions(Some(&allowed));
    if let Some(content) = &content {
        request = request.content(content);
    }
    if !uploads.is_empty() {
        request = request.attachments(&uploads);
    }
    if let Err(error) = request.await {
        tracing::warn!(?error, "clásico : annonce d'un but");
    }
}

/// Photo téléchargée et jointe sous un nom `SPOILER_…` (masquée par Discord) ; `None` si indisponible.
async fn spoiler_photo(url: &str) -> Option<twilight_model::http::attachment::Attachment> {
    const MAX_BYTES: usize = 4 * 1024 * 1024;
    let response = crate::util::http::client().get(url).timeout(Duration::from_secs(5)).send().await.ok()?;
    let bytes = response.error_for_status().ok()?.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return None;
    }
    let extension =
        if url.to_lowercase().ends_with(".jpg") || url.to_lowercase().ends_with(".jpeg") { "jpg" } else { "png" };
    Some(twilight_model::http::attachment::Attachment::from_bytes(
        format!("SPOILER_buteur.{extension}"),
        bytes.to_vec(),
        0,
    ))
}

/// Règle une manche « prochain but » ; renvoie la ligne des gagnants (None : aucun pari).
pub async fn settle_live_round(ctx: &Ctx, game: &Match, round: i64, result: service::NextGoal) -> Option<String> {
    let settled = match service::settle_live_round(&ctx.db, &game.guild_id, &game.id, round, result).await {
        Ok(settled) if !settled.is_empty() => settled,
        Ok(_) => return None,
        Err(error) => {
            tracing::warn!(?error, "clásico : règlement d'un pari « prochain but »");
            return None;
        }
    };
    let winners: Vec<String> = settled
        .iter()
        .filter(|(_, mise, gain)| gain > mise)
        .take(5)
        .map(|(user, mise, gain)| format!("<@{user}> +{}", money(gain - mise)))
        .collect();
    Some(if winners.is_empty() { "Personne ne l'avait vu venir. 🫣".to_owned() } else { winners.join(" · ") })
}

pub async fn handle_golazo(ix: &Ix, options: &Options) -> Result<()> {
    if !allowed(ix, Perm::ClasicoGerer).await? {
        return Ok(());
    }
    let guild = ix.guild_id().expect("vérifié").to_string();
    let Some(game) = service::current(&ix.ctx.db, guild.as_str()).await? else {
        return ix.reply(Reply::ephemeral("Aucun match en cours : `/halamadrid match` d'abord.")).await;
    };
    let adverse_goal = options.boolean("adverse").unwrap_or(false);
    let (madrid, adverse) = if adverse_goal {
        (game.live_madrid, (game.live_adverse + 1).min(MAX_GOALS))
    } else {
        ((game.live_madrid + 1).min(MAX_GOALS), game.live_adverse)
    };
    ix.defer(true).await?;
    service::set_live(&ix.ctx.db, &game.id, madrid, adverse, None).await?;
    let scorer = clean_scorer(options.string("buteur").as_deref());
    let goal = GoalInfo { scorer: scorer.as_deref(), ..GoalInfo::default() };
    announce_goal(&ix.ctx, &game, madrid, adverse, !adverse_goal, goal).await;
    refresh(&ix.ctx, &guild, &game.id).await;
    ix.edit_text(format!("Score en direct : **{madrid} – {adverse}**.")).await
}

/// Podium de la soirée : points, puis pronostic le plus ancien (déjà trié par le service).
fn podium(pronos: &[Prono]) -> String {
    const MEDALS: [&str; 3] = ["🥇", "🥈", "🥉"];
    let lines: Vec<String> = pronos
        .iter()
        .filter(|p| p.points.unwrap_or(0) > 0)
        .take(3)
        .zip(MEDALS)
        .map(|(p, medal)| format!("{medal} <@{}> — {} pt(s)", p.user_id, p.points.unwrap_or(0)))
        .collect();
    if lines.is_empty() { "Personne n'a marqué de point ce soir. 😬".to_owned() } else { lines.join("\n") }
}

fn result_embed(
    game: &Match,
    pronos: &[Prono],
    bets: &[service::Bet],
    madrid: i64,
    adverse: i64,
    clowns: usize,
) -> Embed {
    let exact: Vec<String> = pronos
        .iter()
        .filter(|p| (p.madrid, p.adverse) == (madrid, adverse))
        .map(|p| format!("<@{}>", p.user_id))
        .collect();
    let exact = if exact.is_empty() { "Personne 🫠".to_owned() } else { exact.join(", ") };
    let headline = match Outcome::of(madrid, adverse) {
        Outcome::Victoire => "¡HALA MADRID! 🎉⚪👑",
        Outcome::Nul => "Match nul. On reviendra plus forts. ⚪",
        Outcome::Defaite => "Soirée difficile. Le Real revient toujours. ⚪",
    };
    let scorers = game.buteurs.as_deref().filter(|b| !b.trim().is_empty());
    // Résultat masqué (spoiler) : titre et couleur neutres, tout ce qui trahit le score derrière un clic.
    let summary = match scorers {
        Some(scorers) => format!("{headline}\n⚽ {scorers}"),
        None => headline.to_owned(),
    };
    let mut embed = cards::base(cards::league_label(game.league.as_deref()), MADRID, game.rival_logo.as_deref())
        .title(format!("🏁 Résultat · Real Madrid – {}", game.adversaire))
        .description(format!("{}\n{}", cards::scoreline(madrid, adverse, &game.adversaire), cards::spoiler(&summary)))
        .field(cards::inline("🎯 Score exact", cards::spoiler(&exact)))
        .field(cards::inline("🏆 Podium du soir", cards::spoiler(&podium(pronos))));
    if let Some(worst) = service::worst(pronos, madrid, adverse).filter(|p| p.points == Some(0)) {
        embed = embed.field(field(
            "🙈 Pire prono de la soirée",
            cards::spoiler(&format!("<@{}> avec **{} – {}**", worst.user_id, worst.madrid, worst.adverse)),
            false,
        ));
    }
    let winners: Vec<String> = bets
        .iter()
        .filter(|b| b.gain.unwrap_or(0) > b.mise)
        .take(5)
        .map(|b| format!("<@{}> +{}", b.user_id, money(b.gain.unwrap_or(0) - b.mise)))
        .collect();
    if !bets.is_empty() {
        embed = embed.field(field(
            "💰 Paris",
            cards::spoiler(&if winners.is_empty() {
                "Pas de gagnant : mises rendues ou perdues.".to_owned()
            } else {
                winners.join("\n")
            }),
            false,
        ));
    }
    if clowns > 0 {
        embed = embed.field(field(
            "🤡 Culés repentis",
            cards::spoiler(&format!("{clowns} membre(s) portent le rôle clown pour 24 h.")),
            false,
        ));
    }
    embed
        .footer(footer("Classement et badges : /halamadrid classement"))
        .timestamp(crate::discord::embed::now())
        .build()
}

async fn settle(ix: &Ix, options: &Options, guild: &str) -> Result<()> {
    let Some(game) = service::current(&ix.ctx.db, guild).await? else {
        return ix.reply(Reply::ephemeral("Aucun match à régler.")).await;
    };
    let started = DbTime::now().0 >= game.kickoff.0;
    let (madrid, adverse) = match (options.integer("madrid"), options.integer("adverse")) {
        (Some(madrid), Some(adverse)) => (madrid, adverse),
        (None, None) if started => (game.live_madrid, game.live_adverse),
        _ => {
            return ix
                .reply(Reply::ephemeral("Indique les deux scores (`madrid` et `adverse`), ou laisse-les vides après le coup d'envoi pour reprendre le score en direct."))
                .await;
        }
    };
    let buteurs = options.string("buteurs").map(|b| b.trim().to_owned()).filter(|b| !b.is_empty());
    // Rôle de l'option, sinon celui du mode automatique.
    let role = match options.role("role_clown") {
        Some(role) => Some(role),
        None => service::auto(&ix.ctx.db, guild).await?.clown_role_id.as_deref().and_then(parse_id),
    };
    ix.defer(false).await?;
    let embed = settle_match(&ix.ctx, guild, &game, madrid, adverse, buteurs.as_deref(), role).await?;
    ix.edit(Reply::embed(embed)).await
}

/// Règle le match (points, gains, rôle clown) et renvoie la carte du résultat.
/// Partagé par `/halamadrid resultat` et le résultat automatique.
pub async fn settle_match(
    ctx: &Ctx,
    guild: &str,
    game: &Match,
    madrid: i64,
    adverse: i64,
    buteurs: Option<&str>,
    clown_role: Option<twilight_model::id::Id<twilight_model::id::marker::RoleMarker>>,
) -> Result<Embed> {
    service::settle(&ctx.db, game, madrid, adverse, buteurs).await?;
    // Paris « prochain but » encore ouverts : plus de but, ceux qui l'avaient dit gagnent.
    for round in service::open_live_rounds(&ctx.db, &game.id).await? {
        settle_live_round(ctx, game, round, service::NextGoal::Aucun).await;
    }
    refresh(ctx, guild, &game.id).await;
    let pronos = service::pronos(&ctx.db, &game.id).await?;
    let bets = service::bets(&ctx.db, &game.id).await?;
    let clowns = match clown_role {
        Some(role) if Outcome::of(madrid, adverse) != Outcome::Defaite => {
            give_clown_role(ctx, guild, role, &pronos, &bets).await
        }
        _ => 0,
    };
    let game = service::get(&ctx.db, guild, &game.id).await?.unwrap_or_else(|| game.clone());
    Ok(result_embed(&game, &pronos, &bets, madrid, adverse, clowns))
}

/// Rôle clown pour 24 h à ceux qui ont annoncé (ou parié sur) la victoire de l'adversaire.
async fn give_clown_role(
    ctx: &Ctx,
    guild: &str,
    role: twilight_model::id::Id<twilight_model::id::marker::RoleMarker>,
    pronos: &[Prono],
    bets: &[service::Bet],
) -> usize {
    let Some(guild_id) = parse_id(guild) else { return 0 };
    let mut users: Vec<&str> = pronos.iter().filter(|p| p.adverse > p.madrid).map(|p| p.user_id.as_str()).collect();
    users.extend(bets.iter().filter(|b| b.outcome() == Outcome::Defaite).map(|b| b.user_id.as_str()));
    users.sort_unstable();
    users.dedup();
    let until = DbTime(chrono::Utc::now() + chrono::Duration::hours(CLOWN_HOURS));
    let mut given = 0;
    for user in users {
        let Some(user_id) = parse_id(user) else { continue };
        match ctx.http.add_guild_member_role(guild_id, user_id, role).await {
            Ok(_) => {
                given += 1;
                if let Err(error) = service::add_clown(&ctx.db, guild, user, &role.to_string(), until).await {
                    tracing::warn!(?error, "clásico : échéance du rôle clown");
                }
            }
            Err(error) => tracing::warn!(?error, "clásico : rôle clown refusé (permission « Gérer les rôles » ?)"),
        }
    }
    given
}

async fn wall(ix: &Ix, guild: &str) -> Result<()> {
    let Some(game) = service::last_settled(&ix.ctx.db, guild).await? else {
        return ix.reply(Reply::ephemeral("Aucun match réglé pour l'instant.")).await;
    };
    let (madrid, adverse) = game.settled().unwrap_or_default();
    if Outcome::of(madrid, adverse) == Outcome::Defaite {
        return ix.reply(Reply::text("Pas de mur ce soir… on n'en parle plus. 😶")).await;
    }
    let pronos = service::pronos(&ix.ctx.db, &game.id).await?;
    let bets = service::bets(&ix.ctx.db, &game.id).await?;
    let mut lines: Vec<String> = pronos
        .iter()
        .filter(|p| p.adverse > p.madrid)
        .map(|p| format!("🤡 <@{}> voyait {} gagner **{} – {}**", p.user_id, game.adversaire, p.adverse, p.madrid))
        .collect();
    lines.extend(
        bets.iter()
            .filter(|b| b.outcome() == Outcome::Defaite)
            .map(|b| format!("💸 <@{}> a misé {} contre le Real", b.user_id, money(b.mise))),
    );
    let text = if lines.is_empty() {
        "Le mur est vide : personne n'a trahi la Casa Blanca. 👑".to_owned()
    } else {
        lines.join("\n")
    };
    let embed = EmbedBuilder::new()
        .color(BARCA)
        .title(format!("🧱 Mur de la honte · Real Madrid {madrid} – {adverse} {}", game.adversaire))
        .description(truncate(&text, DESCRIPTION_LIMIT))
        .footer(footer("¡Hala Madrid y nada más!"))
        .build();
    ix.reply(Reply::embed(embed)).await
}

/// Badges de la saison : meneur de chaque catégorie (au moins 1).
fn badges(rows: &[service::Standing]) -> HashMap<&str, String> {
    let mut badges: HashMap<&str, String> = HashMap::new();
    let mut award = |key: fn(&service::Standing) -> i64, badge: &str| {
        let best = rows.iter().map(key).max().unwrap_or(0);
        if best > 0 {
            for row in rows.iter().filter(|r| key(r) == best) {
                badges.entry(row.user_id.as_str()).or_default().push_str(badge);
            }
        }
    };
    award(|r| r.exact, " 🎯");
    award(|r| r.scorers, " 🔮");
    award(|r| r.optimist, " 🤡");
    badges
}

async fn standings(ix: &Ix, guild: &str) -> Result<()> {
    const MEDALS: [&str; 3] = ["🥇", "🥈", "🥉"];
    let rows = service::standings(&ix.ctx.db, guild).await?;
    let text = if rows.is_empty() {
        "Aucun match réglé pour l'instant.".to_owned()
    } else {
        let badges = badges(&rows);
        let lines: Vec<String> = rows
            .iter()
            .take(15)
            .enumerate()
            .map(|(rank, row)| {
                let place = MEDALS.get(rank).map_or_else(|| format!("`{}.`", rank + 1), |m| (*m).to_owned());
                let badge = badges.get(row.user_id.as_str()).map(String::as_str).unwrap_or_default();
                format!(
                    "{place} <@{}>{badge} — **{} pt(s)** · {} exact(s) · {} match(s)",
                    row.user_id, row.points, row.exact, row.played
                )
            })
            .collect();
        format!(
            "{}\n\n-# 🎯 Nostradamus : le plus de scores exacts · 🔮 Voyant : le plus de buteurs trouvés · 🤡 Optimiste culé : le plus de victoires adverses annoncées à tort",
            lines.join("\n")
        )
    };
    let embed = EmbedBuilder::new().color(MADRID).title("🏆 Classement des pronostiqueurs").description(text).build();
    ix.reply(Reply::embed(embed)).await
}

// ---------------------------------------------------------------------------
// Homme du match

fn motm_message(players: &[String], tally: &[i64], match_id: &str, closes_at: DbTime, open: bool) -> Outgoing {
    let total: i64 = tally.iter().sum();
    let best = tally.iter().copied().max().unwrap_or(0);
    let lines: Vec<String> = players
        .iter()
        .zip(tally)
        .map(|(player, votes)| {
            let crown = if !open && best > 0 && *votes == best { " 👑" } else { "" };
            format!("**{player}** — {votes} voix{crown}")
        })
        .collect();
    let state = if open {
        format!("Vote ouvert jusqu'à <t:{}:t> (<t:{}:R>) · {total} voix", closes_at.unix(), closes_at.unix())
    } else {
        format!("Vote terminé · {total} voix")
    };
    let embed = EmbedBuilder::new()
        .color(MADRID)
        .title("⭐ Homme du match")
        .description(format!("{}\n\n{state}", lines.join("\n")))
        .build();
    let components = if open {
        players
            .chunks(5)
            .enumerate()
            .map(|(chunk, names)| {
                row(names
                    .iter()
                    .enumerate()
                    .map(|(i, name)| {
                        let index = (chunk * 5 + i).to_string();
                        Component::Button(button(
                            encode_custom_id(NAMESPACE, "motm", match_id, Some(&index)),
                            truncate(name, 80),
                            ButtonStyle::Secondary,
                        ))
                    })
                    .collect())
            })
            .collect()
    } else {
        Vec::new()
    };
    Outgoing::embeds(vec![embed]).with_components(components)
}

async fn open_motm(ix: &Ix, options: &Options, guild: &str) -> Result<()> {
    let Some(channel_id) = ix.channel_id() else {
        return ix.reply(Reply::ephemeral(GUILD_ONLY)).await;
    };
    let game = match service::current(&ix.ctx.db, guild).await? {
        Some(game) => game,
        None => match service::last_settled(&ix.ctx.db, guild).await? {
            Some(game) => game,
            None => return ix.reply(Reply::ephemeral("Aucun match : `/halamadrid match` d'abord.")).await,
        },
    };
    let players = service::split_players(&options.string("joueurs").unwrap_or_default());
    if players.len() < 2 {
        return ix.reply(Reply::ephemeral("Indique au moins deux joueurs, séparés par des virgules.")).await;
    }
    let minutes = options.integer("minutes").unwrap_or(15);
    let closes_at = DbTime(chrono::Utc::now() + chrono::Duration::minutes(minutes));
    ix.defer(true).await?;
    service::open_motm(&ix.ctx.db, &game.id, &players.join(", "), &channel_id.to_string(), closes_at).await?;
    let message = motm_message(&players, &vec![0; players.len()], &game.id, closes_at, true);
    let sent = messages::send(&ix.ctx, channel_id, &message).await?;
    service::set_motm_message(&ix.ctx.db, &game.id, &sent.id.to_string()).await?;
    // Clôture à l'échéance (un redémarrage entre-temps laisse le vote fermé, sans annonce).
    let ctx = std::sync::Arc::clone(&ix.ctx);
    let match_id = game.id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(u64::try_from(minutes).unwrap_or(15) * 60)).await;
        if let Err(error) = close_motm(&ctx, &match_id).await {
            tracing::warn!(?error, "clásico : clôture du vote de l'homme du match");
        }
    });
    ix.edit_text(format!("Vote publié, clôture dans {minutes} min. ⭐")).await
}

async fn close_motm(ctx: &Ctx, match_id: &str) -> Result<()> {
    let Some(motm) = service::motm(&ctx.db, match_id).await? else { return Ok(()) };
    let players = motm.players();
    let tally = service::motm_tally(&ctx.db, match_id, players.len()).await?;
    let Some(channel) = parse_id(&motm.channel_id) else { return Ok(()) };
    if let Some(message) = motm.message_id.as_deref().and_then(parse_id) {
        messages::edit(ctx, channel, message, &motm_message(&players, &tally, match_id, motm.closes_at, false)).await?;
    }
    let best = tally.iter().copied().max().unwrap_or(0);
    let winners: Vec<&str> =
        players.iter().zip(&tally).filter(|(_, v)| best > 0 && **v == best).map(|(p, _)| p.as_str()).collect();
    let text = if winners.is_empty() {
        "⭐ Homme du match : personne n'a voté. 🫥".to_owned()
    } else {
        format!("⭐ **Homme du match : {}** ({best} voix) ! 👑", winners.join(" et "))
    };
    messages::send(ctx, channel, &Outgoing::text(text)).await?;
    Ok(())
}

async fn vote_motm(ix: &Ix, custom_id: &CustomId<'_>) -> Result<()> {
    let Some(guild_id) = ix.guild_id() else { return Ok(()) };
    let Some(motm) = service::motm(&ix.ctx.db, custom_id.id).await? else {
        return ix.reply(Reply::ephemeral("Vote introuvable.")).await;
    };
    if DbTime::now().0 >= motm.closes_at.0 {
        return ix.reply(Reply::ephemeral("Le vote est terminé.")).await;
    }
    let players = motm.players();
    let Some(index) = custom_id.payload.and_then(|p| p.parse::<usize>().ok()).filter(|i| *i < players.len()) else {
        return ix.reply(Reply::ephemeral("Joueur inconnu.")).await;
    };
    // Le vote doit appartenir à un match de ce serveur.
    if service::get(&ix.ctx.db, &guild_id.to_string(), custom_id.id).await?.is_none() {
        return ix.reply(Reply::ephemeral("Vote introuvable.")).await;
    }
    service::vote_motm(&ix.ctx.db, custom_id.id, &ix.user_id().to_string(), index as i64).await?;
    let tally = service::motm_tally(&ix.ctx.db, custom_id.id, players.len()).await?;
    ix.update(Reply { message: motm_message(&players, &tally, custom_id.id, motm.closes_at, true), ephemeral: false })
        .await?;
    ix.followup(Reply::ephemeral(format!("Ta voix va à **{}**. ⭐", players[index]))).await
}

// ---------------------------------------------------------------------------
// Boutons et formulaire

pub async fn button_pressed(ix: &Ix, custom_id: &CustomId<'_>) -> Result<()> {
    if !allowed(ix, Perm::ClasicoJouer).await? {
        return Ok(());
    }
    match custom_id.action {
        "cri" => cri(ix).await,
        "madridista" => toggle_madridista(ix).await,
        "motm" => vote_motm(ix, custom_id).await,
        "prono" => {
            let goals = |id: &str, placeholder: &str| {
                let mut input = text_input(id, TextInputStyle::Short);
                input.placeholder = Some(placeholder.to_owned());
                input.max_length = Some(2);
                Component::TextInput(input)
            };
            let mut scorer = text_input("buteur", TextInputStyle::Short);
            scorer.required = Some(false);
            scorer.max_length = Some(MAX_SCORER as u16);
            scorer.placeholder = Some("Vinícius Jr".to_owned());
            ix.modal(
                encode_custom_id(NAMESPACE, "prono-modal", custom_id.id, None),
                "Mon prono du Clásico".to_owned(),
                vec![
                    label("Buts du Real Madrid", None, goals("madrid", "3")),
                    label("Buts de l'adversaire", None, goals("adverse", "1")),
                    label(
                        "Premier buteur madrilène",
                        Some("Facultatif : +1 point s'il marque"),
                        Component::TextInput(scorer),
                    ),
                ],
            )
            .await
        }
        _ => Ok(()),
    }
}

pub async fn prono_modal(ix: &Ix, modal: &ModalInteractionData) -> Result<()> {
    let Some(guild_id) = ix.guild_id() else {
        return ix.reply(Reply::ephemeral(GUILD_ONLY)).await;
    };
    let (Some(madrid), Some(adverse)) =
        (parse_goals(&modal_text(modal, "madrid")), parse_goals(&modal_text(modal, "adverse")))
    else {
        return ix.reply(Reply::ephemeral(format!("Score invalide : un nombre de buts entre 0 et {MAX_GOALS}."))).await;
    };
    let buteur = modal_text(modal, "buteur");
    let text = submit(ix, &guild_id.to_string(), madrid, adverse, Some(&buteur)).await?;
    ix.reply(Reply::ephemeral(text)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(kickoff_in_hours: i64) -> Match {
        Match {
            id: "m".into(),
            guild_id: "1".into(),
            adversaire: DEFAULT_RIVAL.into(),
            kickoff: DbTime(chrono::Utc::now() + chrono::Duration::hours(kickoff_in_hours)),
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
        }
    }

    #[test]
    fn lines_fit_and_cheers_are_spotted() {
        assert!(CRIS.len() >= 5 && CHAMBRAGES.len() >= 5);
        assert!(CRIS.iter().chain(CHAMBRAGES).chain(BUTS).chain(BUTS_ENCAISSES).all(|line| line.chars().count() < 200));
        assert_eq!(lowercase_first("Le Barça"), "le Barça");
        assert!(is_barca_cheer("Allez VISCA BARÇA !!"));
        assert!(is_barca_cheer("força barça"));
        assert!(!is_barca_cheer("hala madrid"));
    }

    #[test]
    fn goals_are_bounded() {
        assert_eq!(parse_goals(" 3 "), Some(3));
        assert_eq!(parse_goals("21"), None);
        assert_eq!(parse_goals("-1"), None);
        assert_eq!(clean_scorer(Some("  ")), None);
        assert_eq!(clean_scorer(Some(" Mbappé ")).as_deref(), Some("Mbappé"));
    }

    #[test]
    fn podium_skips_empty_scores() {
        let prono = |user: &str, points| Prono {
            user_id: user.into(),
            madrid: 2,
            adverse: 1,
            buteur: None,
            points: Some(points),
        };
        assert_eq!(podium(&[prono("1", 4), prono("2", 1), prono("3", 0)]), "🥇 <@1> — 4 pt(s)\n🥈 <@2> — 1 pt(s)");
        assert!(podium(&[prono("1", 0)]).starts_with("Personne"));
    }

    #[test]
    fn match_message_follows_the_evening() {
        let upcoming = game(3);
        assert_eq!(match_message(&upcoming, &[], &[], DbTime::now()).components.len(), 1);
        let mut live = game(-1);
        live.live_madrid = 2;
        live.live_clock = Some("67'".into());
        let message = match_message(&live, &[], &[], DbTime::now());
        let embed = &message.embeds[0];
        assert_eq!(embed.title.as_deref(), Some("🔴 EN DIRECT · 67'"));
        assert!(embed.description.as_deref().unwrap_or_default().contains("2 – 0"));
        // Pendant le match : cri et Madridista, plus de prono.
        assert_eq!(message.components.len(), 1);
        live.live_status = Some("HT".into());
        assert_eq!(match_message(&live, &[], &[], DbTime::now()).embeds[0].title.as_deref(), Some("⏸️ Mi-temps"));
        live.score_madrid = Some(2);
        live.score_adverse = Some(0);
        assert!(match_message(&live, &[], &[], DbTime::now()).components.is_empty());
    }

    #[test]
    fn badges_go_to_category_leaders() {
        let row = |user: &str, exact, scorers, optimist| service::Standing {
            user_id: user.into(),
            points: 0,
            exact,
            scorers,
            optimist,
            played: 1,
        };
        let rows = [row("a", 2, 0, 0), row("b", 1, 1, 0), row("c", 0, 0, 1)];
        let badges = badges(&rows);
        assert_eq!(badges.get("a").map(String::as_str), Some(" 🎯"));
        assert_eq!(badges.get("b").map(String::as_str), Some(" 🔮"));
        assert_eq!(badges.get("c").map(String::as_str), Some(" 🤡"));
    }

    #[test]
    fn motm_buttons_split_in_rows_of_five() {
        let players: Vec<String> = (0..7).map(|i| format!("J{i}")).collect();
        let message = motm_message(&players, &[0; 7], "m", DbTime::now(), true);
        assert_eq!(message.components.len(), 2);
        assert!(motm_message(&players, &[3, 1, 0, 0, 0, 0, 0], "m", DbTime::now(), false).components.is_empty());
    }
}
