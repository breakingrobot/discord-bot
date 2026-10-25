//! Permissions par action. Chaque action (`contact.ajouter`, `coffre.voir`…) a
//! un niveau : membres du groupe, gradés, leads ou « Gérer le serveur ». Les
//! niveaux s'emboîtent (un lead est aussi gradé et membre) et chaque serveur
//! peut changer celui d'une action avec `/config permission`.
//!
//! Le rôle **membre** est le passage obligé : sans lui (ou sans rôle gradé ou
//! lead), aucune action n'est ouverte, et rien n'est enregistré au nom de la
//! personne ([`in_group`]). Seul « Gérer le serveur » passe outre, pour
//! administrer ; les actions strictes (avis et vétos des recrues) restent
//! réservées aux rôles. « Gérer les messages » fait d'un membre un gradé.

pub mod commands;
pub mod service;

use crate::discord::{
    interaction::{Ix, Reply},
    site,
};
use anyhow::Result;
use twilight_model::{
    guild::Permissions,
    id::{
        Id,
        marker::{GuildMarker, UserMarker},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Membre,
    Grade,
    Lead,
    Admin,
}

impl Level {
    pub const ALL: [Self; 4] = [Self::Membre, Self::Grade, Self::Lead, Self::Admin];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Membre => "membre",
            Self::Grade => "grade",
            Self::Lead => "lead",
            Self::Admin => "admin",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.as_str() == raw.trim())
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Membre => "Membres du groupe",
            Self::Grade => "Gradés",
            Self::Lead => "Leads",
            Self::Admin => "« Gérer le serveur »",
        }
    }

    /// Niveau d'une personne, pour les messages de refus (« Ton niveau : gradé »).
    pub fn singular(self) -> &'static str {
        match self {
            Self::Membre => "membre du groupe",
            Self::Grade => "gradé",
            Self::Lead => "lead",
            Self::Admin => "« Gérer le serveur »",
        }
    }

    /// Qui passe, pour les messages de refus.
    pub fn who(self) -> &'static str {
        match self {
            Self::Membre => "aux membres du groupe, gradés et leads",
            Self::Grade => "aux gradés et leads",
            Self::Lead => "aux leads",
            Self::Admin => "à « Gérer le serveur »",
        }
    }
}

macro_rules! perms {
    ($($variant:ident => $key:literal, $module:literal, $label:literal, $default:ident, $strict:literal;)*) => {
        /// Action soumise à permission.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Perm {
            $($variant,)*
        }

        impl Perm {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)*];

            pub fn key(self) -> &'static str {
                match self { $(Self::$variant => $key,)* }
            }

            /// Module, pour regrouper l'affichage.
            pub fn module(self) -> &'static str {
                match self { $(Self::$variant => $module,)* }
            }

            pub fn label(self) -> &'static str {
                match self { $(Self::$variant => $label,)* }
            }

            pub fn default_level(self) -> Level {
                match self { $(Self::$variant => Level::$default,)* }
            }

            /// Strict : seuls les rôles comptent, pas les permissions Discord.
            pub fn strict(self) -> bool {
                match self { $(Self::$variant => $strict,)* }
            }
        }
    };
}

perms! {
    PresenceCreer => "presence.creer", "Présences", "Créer, relancer et clôturer une présence", Membre, false;
    PresenceStats => "presence.stats", "Présences", "Statistiques et historique d'une présence", Membre, false;
    PresenceGerer => "presence.gerer", "Présences", "Forcer une réponse, annuler une présence, planning", Grade, false;
    PresenceReset => "presence.reset", "Présences", "Réinitialiser les statistiques de présence", Admin, false;
    Blocklist => "presence.blocklist", "Présences", "Ajouter ou retirer un membre de la blocklist", Grade, false;
    AbsenceAutrui => "absence.autrui", "Absences", "Absence d'un autre membre, panneau des absences", Grade, false;
    TransactionUtiliser => "transaction.utiliser", "Transactions", "Créer, compléter, changer le statut, annuler", Membre, false;
    TransactionSupprimer => "transaction.supprimer", "Transactions", "Supprimer définitivement une transaction", Grade, false;
    TransactionHistorique => "transaction.historique", "Transactions", "Historique et export sur le site", Admin, false;
    TransactionModeles => "transaction.modeles", "Transactions", "Créer, lister et supprimer les modèles de transaction", Grade, false;
    Vehicule => "vehicule.utiliser", "Véhicules", "Enregistrer, consulter, modifier, retirer un véhicule", Membre, false;
    NumeroAjouterAutrui => "numero.ajouter_autrui", "Numéros", "Enregistrer le numéro d'un autre membre", Admin, false;
    NumeroSupprimerAutrui => "numero.supprimer_autrui", "Numéros", "Supprimer le numéro d'un autre membre", Grade, false;
    RecrueSuivi => "recrue.suivi", "Recrues", "Liste, détail et historique des avis, ajout et retrait, salon d'essai", Grade, true;
    RecrueAvis => "recrue.avis", "Recrues", "Donner un avis sur une recrue", Grade, true;
    RecrueVeto => "recrue.veto", "Recrues", "Poser ou lever un véto", Lead, true;
    RecrueReset => "recrue.reset", "Recrues", "Forcer une nouvelle semaine d'avis", Admin, false;
    CotisationAjouter => "cotisation.ajouter", "Cotisations", "Déclarer ses cotisations", Membre, false;
    CotisationGerer => "cotisation.gerer", "Cotisations", "Cotisations des autres : déclarer, consulter, supprimer", Grade, false;
    CotisationStats => "cotisation.stats", "Cotisations", "Statistiques (Discord et site)", Grade, false;
    CoffreVoir => "coffre.voir", "Coffres", "Stock et historique (Discord et site)", Grade, false;
    CoffreGerer => "coffre.gerer", "Coffres", "Relier un personnage, nommer une propriété", Grade, false;
    ContactVoir => "contact.voir", "Contacts", "Consulter le répertoire", Membre, false;
    ContactAjouter => "contact.ajouter", "Contacts", "Ajouter un contact", Membre, false;
    ContactModifier => "contact.modifier", "Contacts", "Modifier ou supprimer un contact", Grade, false;
    ContactNotes => "contact.notes", "Contacts", "Lire et écrire les notes du staff (Discord et site)", Grade, false;
    MembreFiche => "membre.fiche", "Membres", "Fiche d'un membre (présences, cotisations, coffres, recrue…)", Grade, false;
    SiteStats => "site.stats", "Site", "Statistiques du serveur sur le site", Admin, false;
    Journal => "site.journal", "Site", "Journal du staff sur le site", Admin, false;
    M2t => "m2t.utiliser", "Divers", "Relancer la personne visée avec /m2t", Grade, false;
    ClasicoJouer => "clasico.jouer", "Clásico", "/halamadrid (cri, pronostics, classement) et /putabarca", Membre, false;
    ClasicoGerer => "clasico.gerer", "Clásico", "Ouvrir un match aux pronostics et saisir le résultat", Grade, false;
}

impl Perm {
    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|p| p.key() == raw.trim())
    }
}

/// Ce qu'une personne détient sur un serveur.
#[derive(Clone, Debug, Default)]
pub struct Holder {
    pub roles: Vec<String>,
    pub manage_guild: bool,
    pub manage_messages: bool,
}

/// Rôles des niveaux sur un serveur.
#[derive(Clone, Debug, Default)]
pub struct Roles {
    pub member: Option<String>,
    pub grade: Option<String>,
    pub lead: Option<String>,
}

impl Roles {
    fn has(&self, roles: &[String], role: Option<&String>) -> bool {
        role.is_some_and(|r| roles.contains(r))
    }

    /// Fait partie du groupe : rôle membre, gradé ou lead.
    pub fn in_group(&self, roles: &[String]) -> bool {
        [&self.member, &self.grade, &self.lead].into_iter().any(|role| self.has(roles, role.as_ref()))
    }

    /// Niveau le plus haut atteint par une personne ; `None` hors du groupe.
    pub fn level_of(&self, holder: &Holder, strict: bool) -> Option<Level> {
        let has = |role: &Option<String>| self.has(&holder.roles, role.as_ref());
        if !strict && holder.manage_guild {
            Some(Level::Admin)
        } else if !self.in_group(&holder.roles) {
            None
        } else if has(&self.lead) {
            Some(Level::Lead)
        } else if has(&self.grade) || (!strict && holder.manage_messages) {
            Some(Level::Grade)
        } else {
            Some(Level::Membre)
        }
    }

    /// `level` requis pour l'action : la personne a-t-elle le droit ?
    pub fn allows(&self, holder: &Holder, level: Level, strict: bool) -> bool {
        self.level_of(holder, strict).is_some_and(|held| held >= level)
    }
}

/// Droits d'une personne sur un serveur, chargés une fois (Discord ou site).
pub struct Grants {
    roles: Roles,
    levels: std::collections::HashMap<String, Level>,
    holder: Holder,
}

impl Grants {
    pub async fn load(db: &sqlx::SqlitePool, guild_id: Id<GuildMarker>, holder: Holder) -> Result<Self> {
        let guild = guild_id.to_string();
        Ok(Self { roles: service::roles(db, &guild).await?, levels: service::levels(db, &guild).await?, holder })
    }

    /// Droits déjà connus (tests).
    #[cfg(test)]
    pub fn from_parts(roles: Roles, levels: std::collections::HashMap<String, Level>, holder: Holder) -> Self {
        Self { roles, levels, holder }
    }

    pub fn level(&self, perm: Perm) -> Level {
        self.levels.get(perm.key()).copied().unwrap_or(perm.default_level())
    }

    pub fn can(&self, perm: Perm) -> bool {
        self.roles.allows(&self.holder, self.level(perm), perm.strict())
    }

    /// Fait partie du groupe (rôle membre, gradé ou lead), sans compter « Gérer le serveur ».
    pub fn in_group(&self) -> bool {
        self.roles.in_group(&self.holder.roles)
    }

    /// Ce qu'il faut savoir pour expliquer un refus.
    pub fn refusal(&self, perm: Perm) -> Refusal {
        Refusal {
            perm,
            required: self.level(perm),
            held: self.roles.level_of(&self.holder, perm.strict()),
            member_role_set: self.roles.member.is_some(),
            manage_guild: self.holder.manage_guild,
        }
    }
}

/// Refus d'une action : niveau requis, niveau de la personne et réglages du serveur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub perm: Perm,
    pub required: Level,
    /// `None` : hors du groupe.
    pub held: Option<Level>,
    pub member_role_set: bool,
    /// La personne a « Gérer le serveur » : elle peut changer le niveau elle-même.
    pub manage_guild: bool,
}

impl Refusal {
    /// Message de refus qui donne la solution : niveau requis, niveau actuel,
    /// rôle membre à régler, commande pour changer le niveau.
    pub fn message(&self) -> String {
        let mut lines = vec![denied(self.perm, self.required)];
        match self.held {
            None if !self.member_role_set && self.manage_guild => lines.push(
                "Le rôle membre du groupe n'est pas configuré : règle-le avec `/config permission role_membre:@…`."
                    .to_owned(),
            ),
            None if !self.member_role_set => lines.push(
                "Le rôle membre du groupe n'est pas configuré sur ce serveur : un admin doit le régler avec \
                 `/config permission role_membre:@…`."
                    .to_owned(),
            ),
            None if self.perm.strict() && self.manage_guild => lines.push(
                "Action stricte : seuls les rôles comptent, « Gérer le serveur » n'y donne pas accès.".to_owned(),
            ),
            None => lines.push("Tu n'as pas le rôle membre du groupe.".to_owned()),
            Some(held) => lines.push(format!("Ton niveau : {}.", held.singular())),
        }
        if self.manage_guild {
            lines.push(format!("Pour changer ce niveau : `/config permission action:{} niveau:…`.", self.perm.key()));
        }
        lines.join("\n")
    }
}

/// Permissions du serveur, pas celles du salon de la commande : « Gérer les
/// messages » accordé dans un seul salon ne fait pas de quelqu'un un gradé.
fn holder(ix: &Ix, guild_id: Id<GuildMarker>) -> Holder {
    let roles = ix.member_roles();
    // Serveur pas encore en cache (démarrage) : permissions calculées par Discord.
    let permissions = if ix.ctx.cache.guild(guild_id).is_some() {
        crate::discord::members::guild_permissions(&ix.ctx, guild_id, ix.user_id(), roles)
    } else {
        ix.interaction.member.as_ref().and_then(|m| m.permissions).unwrap_or_else(Permissions::empty)
    };
    let permissions = if permissions.contains(Permissions::ADMINISTRATOR) { Permissions::all() } else { permissions };
    Holder {
        roles: roles.iter().map(ToString::to_string).collect(),
        manage_guild: permissions.contains(Permissions::MANAGE_GUILD),
        manage_messages: permissions.contains(Permissions::MANAGE_MESSAGES),
    }
}

/// La personne à l'origine de l'interaction peut-elle faire `perm` ? Hors
/// serveur : jamais (le rôle membre ne peut pas y être vérifié).
pub async fn can(ix: &Ix, perm: Perm) -> Result<bool> {
    let Some(guild_id) = ix.guild_id() else {
        return Ok(false);
    };
    Ok(Grants::load(&ix.ctx.db, guild_id, holder(ix, guild_id)).await?.can(perm))
}

/// Comme [`can`], et répond le refus à la personne. Usage :
/// `if !permissions::require(ix, Perm::X).await? { return Ok(()); }`
pub async fn require(ix: &Ix, perm: Perm) -> Result<bool> {
    let Some(guild_id) = ix.guild_id() else {
        return can(ix, perm).await;
    };
    let grants = Grants::load(&ix.ctx.db, guild_id, holder(ix, guild_id)).await?;
    if grants.can(perm) {
        return Ok(true);
    }
    let refusal = grants.refusal(perm);
    let mut reply = Reply::ephemeral(refusal.message());
    // La page Permissions du site est réservée à « Gérer le serveur ».
    if refusal.manage_guild {
        reply = reply.with_components(site::row_for(&ix.ctx, guild_id, site::Page::Permissions));
    }
    ix.reply(reply).await?;
    Ok(false)
}

pub fn denied(perm: Perm, level: Level) -> String {
    format!("« {} » est réservé {}.", perm.label(), level.who())
}

/// Message quand une personne hors du groupe est concernée par un enregistrement.
pub fn not_member(name: &str) -> String {
    format!("{name} n'a pas le rôle membre : rien n'est enregistré pour cette personne.")
}

/// La personne fait-elle partie du groupe (rôle membre, gradé ou lead) ? Rien
/// n'est enregistré au nom de quelqu'un qui n'en fait pas partie (cotisations,
/// absences, réponses aux présences, numéros, personnages…). Rôles lus dans le
/// cache, sinon auprès de Discord ; inconnu du serveur : non.
pub async fn in_group(ctx: &crate::context::Ctx, guild_id: Id<GuildMarker>, user_id: Id<UserMarker>) -> Result<bool> {
    let roles: Vec<String> = match ctx.cache.member(guild_id, user_id) {
        Some(member) => member.roles().iter().map(ToString::to_string).collect(),
        None => match ctx.http.guild_member(guild_id, user_id).await {
            Ok(response) => response.model().await?.roles.iter().map(ToString::to_string).collect(),
            Err(_) => return Ok(false),
        },
    };
    Ok(service::roles(&ctx.db, &guild_id.to_string()).await?.in_group(&roles))
}

/// Comme [`in_group`], et répond le refus. Usage :
/// `if !permissions::require_member(ix, cible, &nom).await? { return Ok(()); }`
pub async fn require_member(ix: &Ix, user_id: Id<UserMarker>, name: &str) -> Result<bool> {
    let Some(guild_id) = ix.guild_id() else {
        return Ok(false);
    };
    if in_group(&ix.ctx, guild_id, user_id).await? {
        return Ok(true);
    }
    let message = if user_id == ix.user_id() {
        "Tu n'as pas le rôle membre du groupe : rien n'est enregistré à ton nom.".to_owned()
    } else {
        not_member(name)
    };
    ix.reply(Reply::ephemeral(message)).await?;
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles() -> Roles {
        Roles { member: Some("1".into()), grade: Some("2".into()), lead: Some("3".into()) }
    }

    fn with(roles: &[&str]) -> Holder {
        Holder { roles: roles.iter().map(|r| (*r).to_owned()).collect(), ..Default::default() }
    }

    #[test]
    fn levels_nest() {
        let r = roles();
        assert!(!r.allows(&with(&[]), Level::Membre, false));
        assert!(!r.allows(&with(&["99"]), Level::Membre, false));
        assert!(r.allows(&with(&["1"]), Level::Membre, false));
        assert!(!r.allows(&with(&["1"]), Level::Grade, false));
        assert!(r.allows(&with(&["3"]), Level::Membre, false));
        assert!(r.allows(&with(&["3"]), Level::Lead, false));
        assert!(!r.allows(&with(&["3"]), Level::Admin, false));
    }

    #[test]
    fn discord_permissions_count_unless_strict() {
        let r = roles();
        let admin = Holder { manage_guild: true, ..Default::default() };
        // « Gérer les messages » fait d'un membre un gradé, pas de quelqu'un hors du groupe.
        let moderator = Holder { roles: vec!["1".into()], manage_messages: true, ..Default::default() };
        let outsider = Holder { manage_messages: true, ..Default::default() };
        assert!(!r.allows(&outsider, Level::Membre, false));
        assert!(r.allows(&admin, Level::Admin, false));
        assert!(r.allows(&admin, Level::Lead, false));
        assert!(!r.allows(&admin, Level::Grade, true));
        assert!(r.allows(&moderator, Level::Grade, false));
        assert!(!r.allows(&moderator, Level::Lead, false));
        assert!(!r.allows(&moderator, Level::Grade, true));
        assert!(r.allows(&moderator, Level::Membre, true));
    }

    #[test]
    fn nothing_defaults_to_everyone() {
        // Le rôle membre est le passage obligé : aucune action n'est ouverte hors du groupe.
        assert!(Perm::ALL.iter().all(|p| p.default_level() >= Level::Membre));
        let r = roles();
        for perm in Perm::ALL {
            assert!(!r.allows(&with(&[]), perm.default_level(), perm.strict()), "{}", perm.key());
        }
        assert!(r.in_group(&["2".into()]) && r.in_group(&["3".into()]) && !r.in_group(&["4".into()]));
    }

    #[test]
    fn unconfigured_member_role_lets_nobody_through() {
        let r = Roles { member: None, ..roles() };
        assert!(!r.allows(&with(&["1"]), Level::Membre, false));
        assert!(r.allows(&with(&["2"]), Level::Membre, false));
    }

    #[test]
    fn refusals_explain_the_way_out() {
        let refusal = Refusal {
            perm: Perm::CoffreVoir,
            required: Level::Grade,
            held: Some(Level::Membre),
            member_role_set: true,
            manage_guild: false,
        };
        assert_eq!(
            refusal.message(),
            "« Stock et historique (Discord et site) » est réservé aux gradés et leads.\nTon niveau : membre du groupe."
        );
        let outsider = Refusal { held: None, ..refusal };
        assert!(outsider.message().ends_with("Tu n'as pas le rôle membre du groupe."));
        let unset = Refusal { member_role_set: false, ..outsider };
        assert!(unset.message().contains("un admin doit le régler avec `/config permission role_membre:@…`"));
        // Une personne avec « Gérer le serveur » ne bute que sur les actions strictes.
        let admin = Refusal {
            perm: Perm::RecrueVeto,
            required: Level::Lead,
            held: None,
            member_role_set: true,
            manage_guild: true,
        };
        let message = admin.message();
        assert!(message.contains("Action stricte"));
        assert!(message.ends_with("`/config permission action:recrue.veto niveau:…`."));
    }

    #[test]
    fn grants_describe_the_refusal() {
        let levels = std::collections::HashMap::from([("coffre.voir".to_owned(), Level::Lead)]);
        let grants = Grants::from_parts(roles(), levels, with(&["2"]));
        let refusal = grants.refusal(Perm::CoffreVoir);
        assert_eq!((refusal.required, refusal.held, refusal.member_role_set), (Level::Lead, Some(Level::Grade), true));
    }

    #[test]
    fn keys_are_unique_and_parse_back() {
        for perm in Perm::ALL {
            assert_eq!(Perm::parse(perm.key()), Some(*perm));
        }
        let mut keys: Vec<_> = Perm::ALL.iter().map(|p| p.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), Perm::ALL.len());
        for level in Level::ALL {
            assert_eq!(Level::parse(level.as_str()), Some(level));
        }
    }
}
