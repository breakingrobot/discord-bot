//! Configuration du module ticket, au format de modmailbot : `config.ini`
//! (ou `config.json`) et variables d'environnement `MM_*`. Les noms d'options
//! et leurs valeurs par défaut sont identiques à l'original ; un fichier de
//! configuration modmailbot existant peut être réutilisé tel quel.

use super::{ini, util::convert_delay_string_to_ms};
use anyhow::{Result, bail};
use serde_json::{Map, Value};
use std::{collections::HashMap, path::PathBuf};
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttachmentStorage {
    Original,
    Local,
    Discord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogStorage {
    Local,
    Attachment,
    None,
}

#[derive(Clone, Debug)]
pub struct Greeting {
    pub message: Option<String>,
    pub attachment: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MmConfig {
    pub main_server_ids: Vec<Id<GuildMarker>>,
    pub inbox_server_id: Id<GuildMarker>,
    pub log_channel_id: Id<ChannelMarker>,

    pub prefix: String,
    pub snippet_prefix: String,
    pub snippet_prefix_anon: String,
    pub status: Option<String>,
    pub status_type: String,
    pub status_url: String,
    pub response_message: Option<String>,
    pub close_message: Option<String>,
    pub allow_user_close: bool,
    /// `autoCloseInactive` (propre à ce bot) : inactivité, en ms, au bout de laquelle
    /// l'utilisateur est averti et la fermeture programmée. `None` : désactivé.
    pub auto_close_inactive_ms: Option<i64>,
    /// `autoCloseWarning` (propre à ce bot) : délai entre l'avertissement et la fermeture, en ms.
    pub auto_close_warning_ms: i64,
    /// `autoCloseMessage` (propre à ce bot) : avertissement envoyé à l'utilisateur
    /// (`{inactive}` et `{delay}` remplacés par les durées).
    pub auto_close_message: String,

    pub new_thread_category_id: Option<String>,
    /// `newThreadForumId` (propre à ce bot) : tickets ouverts en fils d'un salon forum.
    pub new_thread_forum_id: Option<Id<ChannelMarker>>,
    pub new_thread_category_from_server: Vec<(String, String)>,
    pub mention_role: Vec<String>,
    pub ping_on_bot_mention: bool,
    pub bot_mention_response: Option<String>,

    pub inbox_server_permission: Vec<String>,
    pub always_reply: bool,
    pub always_reply_anon: bool,
    pub force_anon: bool,
    pub use_nicknames: bool,
    pub use_displaynames: bool,
    pub anonymize_channel_name: bool,
    pub ignore_accidental_threads: bool,
    pub thread_timestamps: bool,
    pub allow_move: bool,
    pub sync_permissions_on_move: bool,
    pub typing_proxy: bool,
    pub typing_proxy_reverse: bool,
    pub mention_user_in_thread_header: bool,
    pub roles_in_thread_header: bool,
    pub allow_staff_edit: bool,
    pub allow_staff_delete: bool,
    pub update_messages_live: bool,
    pub allow_block: bool,
    pub allow_suspend: bool,
    pub allow_snippets: bool,

    pub enable_greeting: bool,
    pub server_greetings: HashMap<String, Greeting>,

    pub required_account_age_hours: Option<f64>,
    pub account_age_denied_message: Option<String>,
    pub required_time_on_server_minutes: Option<f64>,
    pub time_on_server_denied_message: Option<String>,

    pub relay_small_attachments_as_attachments: bool,
    pub small_attachment_limit: u64,
    pub relay_inline_replies: bool,
    pub attachment_storage: AttachmentStorage,
    pub attachment_storage_channel_id: Option<Id<ChannelMarker>>,
    pub attachment_dir: PathBuf,

    pub command_aliases: HashMap<String, String>,
    pub react_on_seen: bool,
    pub react_on_seen_emoji: String,
    pub notify_on_reaction: bool,
    pub notify_on_reaction_removal: bool,
    pub create_thread_on_mention: bool,
    pub block_message: Option<String>,
    pub timed_block_message: Option<String>,
    pub unblock_message: Option<String>,
    pub timed_unblock_message: Option<String>,
    pub blocked_reply: Option<String>,
    pub notify_on_main_server_leave: bool,
    pub notify_on_main_server_join: bool,

    pub allow_inline_snippets: bool,
    pub inline_snippet_start: String,
    pub inline_snippet_end: String,
    pub error_on_unknown_inline_snippet: bool,
    pub allow_changing_display_role: bool,
    pub fallback_role_name: Option<String>,
    pub override_role_name_display: Option<String>,
    pub break_formatting_for_names: bool,
    pub auto_alert: bool,
    pub auto_alert_delay: String,
    pub pin_thread_header: bool,
    pub show_response_message_in_thread_channel: bool,
    pub allow_notes: bool,

    pub log_storage: LogStorage,
    pub log_attachment_directory: PathBuf,
    pub port: u16,
    pub host: String,
    pub url: Option<String>,
    pub sqlite_filename: PathBuf,
}

/// Fichiers recherchés, dans l'ordre de priorité de modmailbot.
const CONFIG_FILES: [&str; 6] =
    ["config.ini", "config.json", "config.ini.ini", "config.ini.txt", "config.json.json", "config.json.txt"];

fn env_key_to_config_key(key: &str) -> String {
    // MM_CLOSE_MESSAGE -> closeMessage ; MM_COMMAND_ALIASES__MV -> commandAliases.mv
    // Même expression que modmailbot (remplacements non chevauchants).
    static CAMEL: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new("([a-z])_([a-z])").expect("regex valide"));
    let lower = key.to_lowercase();
    CAMEL
        .replace_all(&lower, |caps: &regex::Captures<'_>| format!("{}{}", &caps[1], caps[2].to_uppercase()))
        .replacen("__", ".", 1)
}

/// Lit la configuration brute. `None` si aucun fichier ni variable `MM_*` :
/// le module ticket est alors désactivé.
fn load_raw() -> Result<Option<Map<String, Value>>> {
    let requested = std::env::var("MODMAIL_CONFIG").ok().filter(|p| !p.trim().is_empty());
    let file = match requested {
        Some(path) => {
            if !std::path::Path::new(&path).exists() {
                bail!("fichier de configuration modmail introuvable : {path}");
            }
            Some(path)
        }
        None => CONFIG_FILES.iter().find(|f| std::path::Path::new(f).exists()).map(|f| (*f).to_owned()),
    };

    let mut config = match &file {
        Some(path) => {
            let raw = std::fs::read_to_string(path)?;
            tracing::info!(%path, "configuration modmail chargée");
            if path.contains(".ini") { ini::decode(&raw) } else { serde_json::from_str::<Map<String, Value>>(&raw)? }
        }
        None => Map::new(),
    };

    let mut from_env = 0;
    for (key, value) in std::env::vars() {
        // MM_TOKEN sert de repli au token du bot : il n'active pas le module.
        let Some(stripped) = key.strip_prefix("MM_").filter(|k| *k != "TOKEN") else {
            continue;
        };
        let config_key = env_key_to_config_key(stripped);
        let value = if value.contains("||") {
            Value::Array(value.split("||").map(|v| Value::String(v.to_owned())).collect())
        } else {
            Value::String(value)
        };
        config.insert(config_key, value);
        from_env += 1;
    }
    if file.is_none() && from_env == 0 {
        return Ok(None);
    }
    if let Ok(port) = std::env::var("PORT") {
        config.entry("port").or_insert(Value::String(port));
    }

    // Clés avec points -> objets imbriqués (commandAliases.mv -> commandAliases: { mv }).
    let dotted: Vec<String> = config.keys().filter(|k| k.contains('.')).cloned().collect();
    for key in dotted {
        let value = config.remove(&key).expect("clé présente");
        let parts: Vec<&str> = key.split('.').collect();
        let mut cursor = &mut config;
        for part in &parts[..parts.len() - 1] {
            let entry = cursor.entry((*part).to_owned()).or_insert_with(|| Value::Object(Map::new()));
            if !entry.is_object() {
                *entry = Value::Object(Map::new());
            }
            cursor = entry.as_object_mut().expect("objet");
        }
        cursor.insert(parts[parts.len() - 1].to_owned(), value);
    }

    // `option=` sans valeur dans config.ini = option non définie.
    config.retain(|_, value| value.as_str() != Some(""));
    Ok(Some(config))
}

struct Reader<'a> {
    map: &'a Map<String, Value>,
    errors: Vec<String>,
}

impl Reader<'_> {
    fn raw(&self, key: &str) -> Option<&Value> {
        self.map.get(key).filter(|v| !v.is_null())
    }

    fn string(&self, key: &str) -> Option<String> {
        match self.raw(key)? {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Array(items) => items.first().and_then(|v| v.as_str()).map(str::to_owned),
            _ => None,
        }
    }

    fn string_or(&self, key: &str, default: &str) -> String {
        self.string(key).unwrap_or_else(|| default.to_owned())
    }

    /// Texte multiligne : un tableau (`cle[] = …`) est joint par des retours à la ligne.
    fn multiline(&self, key: &str) -> Option<String> {
        match self.raw(key)? {
            Value::Array(items) => Some(items.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n")),
            _ => self.string(key),
        }
    }

    fn list(&self, key: &str) -> Option<Vec<String>> {
        match self.raw(key)? {
            Value::Array(items) => Some(items.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect()),
            Value::String(s) => Some(vec![s.clone()]),
            Value::Number(n) => Some(vec![n.to_string()]),
            _ => None,
        }
    }

    fn boolean(&mut self, key: &str, default: bool) -> bool {
        match self.raw(key) {
            None => default,
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => match s.to_lowercase().as_str() {
                "1" | "true" | "on" | "yes" => true,
                "0" | "false" | "off" | "no" => false,
                other => {
                    self.errors
                        .push(format!("L'option \"{key}\" doit être un booléen. (Valeur actuelle : \"{other}\")"));
                    default
                }
            },
            Some(Value::Number(n)) => n.as_f64() != Some(0.0),
            Some(_) => default,
        }
    }

    fn number(&mut self, key: &str) -> Option<f64> {
        let value = self.raw(key)?;
        let parsed = match value {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse().ok(),
            _ => None,
        };
        if parsed.is_none() {
            self.errors.push(format!("L'option \"{key}\" doit être un nombre."));
        }
        parsed
    }

    /// Durée au format `1h30m` / `7d`, en ms. `0`, `off`, `none` ou `false` : `None`.
    fn delay(&mut self, key: &str, default: Option<&str>) -> Option<i64> {
        let raw = self.string(key).or_else(|| default.map(str::to_owned))?;
        if matches!(raw.trim().to_lowercase().as_str(), "off" | "none" | "false" | "no") {
            return None;
        }
        match convert_delay_string_to_ms(&raw) {
            Some(0) => None,
            Some(ms) => Some(ms),
            None => {
                self.errors.push(format!("The \"{key}\" option must be a delay such as \"7d\" or \"1h30m\"."));
                None
            }
        }
    }

    fn object(&self, key: &str) -> Option<&Map<String, Value>> {
        self.raw(key)?.as_object()
    }
}

impl MmConfig {
    pub fn load() -> Result<Option<Self>> {
        let Some(map) = load_raw()? else {
            return Ok(None);
        };
        let mut r = Reader { map: &map, errors: Vec::new() };

        let main_server_raw = r.list("mainServerId").or_else(|| r.list("mainGuildId")).unwrap_or_default();
        let inbox_raw = r.string("inboxServerId").or_else(|| r.string("mailGuildId"));
        let log_channel_raw = r.string("logChannelId");

        let main_server_ids: Vec<Id<GuildMarker>> =
            main_server_raw.iter().filter_map(|id| id.trim().parse().ok()).collect();
        if main_server_ids.is_empty() {
            r.errors.push("Option obligatoire manquante : \"mainServerId\"".to_owned());
        }
        let inbox_server_id = inbox_raw.as_deref().and_then(|id| id.trim().parse().ok());
        if inbox_server_id.is_none() {
            r.errors.push("Option obligatoire manquante : \"inboxServerId\"".to_owned());
        }
        let log_channel_id = log_channel_raw.as_deref().and_then(|id| id.trim().parse().ok());
        if log_channel_id.is_none() {
            r.errors.push("Option obligatoire manquante : \"logChannelId\"".to_owned());
        }

        if r.string("dbType").is_some_and(|t| t != "sqlite") {
            r.errors.push("Seul dbType = sqlite est pris en charge par ce bot.".to_owned());
        }
        if r.list("plugins").is_some_and(|p| !p.is_empty()) {
            tracing::warn!("modmail : les plugins JavaScript ne sont pas pris en charge et seront ignorés");
        }

        let attachment_storage = match r.string_or("attachmentStorage", "original").as_str() {
            "local" => AttachmentStorage::Local,
            "discord" => AttachmentStorage::Discord,
            "original" => AttachmentStorage::Original,
            other => {
                r.errors.push(format!("Option de stockage des pièces jointes inconnue : {other}"));
                AttachmentStorage::Original
            }
        };
        let attachment_storage_channel_id = r.string("attachmentStorageChannelId").and_then(|id| id.parse().ok());
        if attachment_storage == AttachmentStorage::Discord && attachment_storage_channel_id.is_none() {
            r.errors.push("Option obligatoire manquante : \"attachmentStorageChannelId\"".to_owned());
        }

        let log_storage = match r.string_or("logStorage", "local").as_str() {
            "attachment" => LogStorage::Attachment,
            "none" => LogStorage::None,
            _ => LogStorage::Local,
        };

        // newThreadCategoryId est un raccourci pour categoryAutomation.newThread.
        let automation = r.object("categoryAutomation");
        let new_thread_category_id = r
            .string("newThreadCategoryId")
            .or_else(|| automation.and_then(|a| a.get("newThread")).and_then(|v| v.as_str()).map(str::to_owned));
        let from_server = automation
            .and_then(|a| a.get("newThreadFromServer").or_else(|| a.get("newThreadFromGuild")))
            .and_then(Value::as_object)
            .map(|m| m.iter().filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_owned()))).collect())
            .unwrap_or_default();

        // greetingMessage/greetingAttachment s'appliquent à tous les serveurs
        // principaux qui n'ont pas de message dédié dans serverGreetings.
        let mut server_greetings: HashMap<String, Greeting> = r
            .object("serverGreetings")
            .or_else(|| r.object("guildGreetings"))
            .map(|m| {
                m.iter()
                    .filter_map(|(guild, value)| {
                        let obj = value.as_object()?;
                        let message = match obj.get("message") {
                            Some(Value::Array(items)) => {
                                Some(items.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n"))
                            }
                            Some(Value::String(s)) => Some(s.clone()),
                            _ => None,
                        };
                        let attachment = obj.get("attachment").and_then(|v| v.as_str()).map(str::to_owned);
                        Some((guild.clone(), Greeting { message, attachment }))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let greeting_message = r.multiline("greetingMessage");
        let greeting_attachment = r.string("greetingAttachment");
        if greeting_message.is_some() || greeting_attachment.is_some() {
            for guild in &main_server_ids {
                server_greetings.entry(guild.to_string()).or_insert_with(|| Greeting {
                    message: greeting_message.clone(),
                    attachment: greeting_attachment.clone(),
                });
            }
        }

        let command_aliases = r
            .object("commandAliases")
            .map(|m| {
                m.iter().filter_map(|(alias, cmd)| cmd.as_str().map(|c| (alias.to_lowercase(), c.to_owned()))).collect()
            })
            .unwrap_or_default();

        let log_options = r.object("logOptions");
        let log_attachment_directory = PathBuf::from(
            log_options.and_then(|o| o.get("attachmentDirectory")).and_then(|v| v.as_str()).unwrap_or("data/logs"),
        );
        let sqlite_filename = PathBuf::from(
            r.object("sqliteOptions")
                .and_then(|o| o.get("filename"))
                .and_then(|v| v.as_str())
                .unwrap_or("data/modmail.sqlite"),
        );

        let small_attachment_limit = r.number("smallAttachmentLimit").map_or(2_097_152, |n| n as u64);
        let port = r.number("port").map_or(8890, |n| n as u16);
        let required_account_age_hours = r.number("requiredAccountAge");
        let required_time_on_server_minutes = r.number("requiredTimeOnServer");

        let status_type = r.string_or("statusType", "playing");
        let status_url = r.string_or("statusUrl", "");
        if status_type == "streaming"
            && !regex::Regex::new(r"(?i)^https://(www\.)?twitch.tv/[a-z\d_\-]+/?$")
                .expect("regex valide")
                .is_match(&status_url)
        {
            r.errors.push(
                "Quand statusType vaut \"streaming\", statusUrl doit être une URL de chaîne Twitch valide, par exemple https://www.twitch.tv/Dragory"
                    .to_owned(),
            );
        }

        let config = Self {
            main_server_ids,
            inbox_server_id: inbox_server_id.unwrap_or_else(|| Id::new(1)),
            log_channel_id: log_channel_id.unwrap_or_else(|| Id::new(1)),
            prefix: r.string_or("prefix", "!"),
            snippet_prefix: r.string_or("snippetPrefix", "!!"),
            snippet_prefix_anon: r.string_or("snippetPrefixAnon", "!!!"),
            status: Some(r.string_or("status", "Écris-moi pour contacter le staff !")),
            status_type,
            status_url,
            response_message: Some(r.multiline("responseMessage").unwrap_or_else(|| {
                "Merci pour ton message ! L'équipe de modération te répondra ici dès que possible.".to_owned()
            })),
            close_message: r.multiline("closeMessage"),
            allow_user_close: r.boolean("allowUserClose", false),
            auto_close_inactive_ms: r.delay("autoCloseInactive", None),
            auto_close_warning_ms: r.delay("autoCloseWarning", Some("24h")).unwrap_or(86_400_000),
            auto_close_message: r.multiline("autoCloseMessage").unwrap_or_else(|| {
                "Ton ticket est inactif depuis {inactive}. Il sera fermé dans {delay} sans nouvelle réponse de ta part."
                    .to_owned()
            }),
            new_thread_category_id,
            new_thread_forum_id: r.string("newThreadForumId").and_then(|id| id.parse().ok()),
            new_thread_category_from_server: from_server,
            mention_role: r.list("mentionRole").unwrap_or_else(|| vec!["none".to_owned()]),
            ping_on_bot_mention: r.boolean("pingOnBotMention", true),
            bot_mention_response: r.multiline("botMentionResponse"),
            inbox_server_permission: r
                .list("inboxServerPermission")
                .unwrap_or_else(|| vec!["manageMessages".to_owned()]),
            always_reply: r.boolean("alwaysReply", false),
            always_reply_anon: r.boolean("alwaysReplyAnon", false),
            force_anon: r.boolean("forceAnon", false),
            use_nicknames: r.boolean("useNicknames", false),
            use_displaynames: r.boolean("useDisplaynames", true),
            anonymize_channel_name: r.boolean("anonymizeChannelName", false),
            ignore_accidental_threads: r.boolean("ignoreAccidentalThreads", false),
            thread_timestamps: r.boolean("threadTimestamps", false),
            allow_move: r.boolean("allowMove", false),
            sync_permissions_on_move: r.boolean("syncPermissionsOnMove", true),
            typing_proxy: r.boolean("typingProxy", false),
            typing_proxy_reverse: r.boolean("typingProxyReverse", false),
            mention_user_in_thread_header: r.boolean("mentionUserInThreadHeader", false),
            roles_in_thread_header: r.boolean("rolesInThreadHeader", false),
            allow_staff_edit: r.boolean("allowStaffEdit", true),
            allow_staff_delete: r.boolean("allowStaffDelete", true),
            update_messages_live: r.boolean("updateMessagesLive", false),
            allow_block: r.boolean("allowBlock", true),
            allow_suspend: r.boolean("allowSuspend", true),
            allow_snippets: r.boolean("allowSnippets", true),
            enable_greeting: r.boolean("enableGreeting", false),
            server_greetings,
            required_account_age_hours,
            account_age_denied_message: Some(
                r.multiline("accountAgeDeniedMessage")
                    .unwrap_or_else(|| "Ton compte Discord est trop récent pour contacter le staff.".to_owned()),
            ),
            required_time_on_server_minutes,
            time_on_server_denied_message: Some(r.multiline("timeOnServerDeniedMessage").unwrap_or_else(|| {
                "Tu n'es pas membre du serveur depuis assez longtemps pour contacter le staff.".to_owned()
            })),
            relay_small_attachments_as_attachments: r.boolean("relaySmallAttachmentsAsAttachments", false),
            small_attachment_limit,
            relay_inline_replies: r.boolean("relayInlineReplies", true),
            attachment_storage,
            attachment_storage_channel_id,
            attachment_dir: PathBuf::from(r.string_or("attachmentDir", "data/attachments")),
            command_aliases,
            react_on_seen: r.boolean("reactOnSeen", false),
            react_on_seen_emoji: r.string_or("reactOnSeenEmoji", "📨"),
            notify_on_reaction: r.boolean("notifyOnReaction", false),
            notify_on_reaction_removal: r.boolean("notifyOnReactionRemoval", false),
            create_thread_on_mention: r.boolean("createThreadOnMention", false),
            block_message: r.string("blockMessage"),
            timed_block_message: r.string("timedBlockMessage"),
            unblock_message: r.string("unblockMessage"),
            timed_unblock_message: r.string("timedUnblockMessage"),
            blocked_reply: r.string("blockedReply"),
            notify_on_main_server_leave: r.boolean("notifyOnMainServerLeave", true),
            notify_on_main_server_join: r.boolean("notifyOnMainServerJoin", true),
            allow_inline_snippets: r.boolean("allowInlineSnippets", true),
            inline_snippet_start: r.string_or("inlineSnippetStart", "{{"),
            inline_snippet_end: r.string_or("inlineSnippetEnd", "}}"),
            error_on_unknown_inline_snippet: r.boolean("errorOnUnknownInlineSnippet", true),
            allow_changing_display_role: r.boolean("allowChangingDisplayRole", true),
            fallback_role_name: r.string("fallbackRoleName"),
            override_role_name_display: r.string("overrideRoleNameDisplay"),
            break_formatting_for_names: r.boolean("breakFormattingForNames", true),
            auto_alert: r.boolean("autoAlert", false),
            auto_alert_delay: r.string_or("autoAlertDelay", "2m"),
            pin_thread_header: r.boolean("pinThreadHeader", false),
            show_response_message_in_thread_channel: r.boolean("showResponseMessageInThreadChannel", true),
            allow_notes: r.boolean("allowNotes", true),
            log_storage,
            log_attachment_directory,
            port,
            host: r.string_or("host", "0.0.0.0"),
            url: r.string("url").map(|u| u.trim_end_matches('/').to_owned()),
            sqlite_filename,
        };

        if !r.errors.is_empty() {
            bail!("ATTENTION ! Problèmes dans la configuration modmail :\n- {}", r.errors.join("\n- "));
        }

        // `status = none` / `off` désactive le statut.
        let mut config = config;
        if config.status.as_deref().is_some_and(|s| s.is_empty() || s == "none" || s == "off") {
            config.status = None;
        }
        Ok(Some(config))
    }

    pub fn is_main_server(&self, guild_id: Id<GuildMarker>) -> bool {
        self.main_server_ids.contains(&guild_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{Reader, env_key_to_config_key};

    #[test]
    fn reads_delay_options() {
        let map =
            serde_json::json!({ "autoCloseInactive": "7d", "autoCloseWarning": "off", "bad": "abc", "zero": "0" });
        let mut r = Reader { map: map.as_object().unwrap(), errors: Vec::new() };
        assert_eq!(r.delay("autoCloseInactive", None), Some(7 * 86_400_000));
        assert_eq!(r.delay("autoCloseWarning", Some("24h")), None);
        assert_eq!(r.delay("missing", Some("24h")), Some(86_400_000));
        assert_eq!(r.delay("missing", None), None);
        assert_eq!(r.delay("zero", None), None);
        assert!(r.errors.is_empty());
        assert_eq!(r.delay("bad", None), None);
        assert_eq!(r.errors.len(), 1);
    }

    #[test]
    fn maps_env_keys_like_modmailbot() {
        assert_eq!(env_key_to_config_key("CLOSE_MESSAGE"), "closeMessage");
        assert_eq!(env_key_to_config_key("MAIN_SERVER_ID"), "mainServerId");
        assert_eq!(env_key_to_config_key("COMMAND_ALIASES__MV"), "commandAliases.mv");
        assert_eq!(env_key_to_config_key("SQLITE_OPTIONS__FILENAME"), "sqliteOptions.filename");
    }
}
