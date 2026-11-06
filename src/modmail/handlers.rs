//! Événements Discord du module ticket (`src/main.js` et modules de modmailbot).

use super::{
    Modmail,
    models::{ACCIDENTAL_THREAD_MESSAGES, message_type},
    store,
    thread::{Moderator, NewThreadOptions, StaffReply, mention_roles},
};
use crate::{
    context::Ctx,
    discord::messages::{Mentions, parse_id},
};
use anyhow::Result;
use twilight_gateway::Event;
use twilight_model::{
    channel::{
        Message,
        message::{EmojiReactionType, MessageType},
    },
    gateway::GatewayReaction,
    guild::Permissions,
    id::{
        Id,
        marker::{GuildMarker, UserMarker},
    },
};

/// Noms de permissions Eris (`inboxServerPermission`) → permissions Discord.
fn permission_from_name(name: &str) -> Option<Permissions> {
    Some(match name {
        "createInstantInvite" => Permissions::CREATE_INVITE,
        "kickMembers" => Permissions::KICK_MEMBERS,
        "banMembers" => Permissions::BAN_MEMBERS,
        "administrator" => Permissions::ADMINISTRATOR,
        "manageChannels" => Permissions::MANAGE_CHANNELS,
        "manageGuild" => Permissions::MANAGE_GUILD,
        "addReactions" => Permissions::ADD_REACTIONS,
        "viewAuditLog" | "viewAuditLogs" => Permissions::VIEW_AUDIT_LOG,
        "voicePrioritySpeaker" => Permissions::PRIORITY_SPEAKER,
        "voiceStream" | "stream" => Permissions::STREAM,
        "viewChannel" | "readMessages" => Permissions::VIEW_CHANNEL,
        "sendMessages" => Permissions::SEND_MESSAGES,
        "sendTTSMessages" => Permissions::SEND_TTS_MESSAGES,
        "manageMessages" => Permissions::MANAGE_MESSAGES,
        "embedLinks" => Permissions::EMBED_LINKS,
        "attachFiles" => Permissions::ATTACH_FILES,
        "readMessageHistory" => Permissions::READ_MESSAGE_HISTORY,
        "mentionEveryone" => Permissions::MENTION_EVERYONE,
        "useExternalEmojis" | "externalEmojis" => Permissions::USE_EXTERNAL_EMOJIS,
        "viewGuildInsights" => Permissions::VIEW_GUILD_INSIGHTS,
        "voiceConnect" => Permissions::CONNECT,
        "voiceSpeak" => Permissions::SPEAK,
        "voiceMuteMembers" => Permissions::MUTE_MEMBERS,
        "voiceDeafenMembers" => Permissions::DEAFEN_MEMBERS,
        "voiceMoveMembers" => Permissions::MOVE_MEMBERS,
        "voiceUseVAD" => Permissions::USE_VAD,
        "changeNickname" => Permissions::CHANGE_NICKNAME,
        "manageNicknames" => Permissions::MANAGE_NICKNAMES,
        "manageRoles" => Permissions::MANAGE_ROLES,
        "manageWebhooks" => Permissions::MANAGE_WEBHOOKS,
        "manageEmojis" | "manageEmojisAndStickers" | "manageExpressions" | "manageGuildExpressions" => {
            Permissions::MANAGE_GUILD_EXPRESSIONS
        }
        "useApplicationCommands" | "useSlashCommands" => Permissions::USE_SLASH_COMMANDS,
        "manageEvents" => Permissions::MANAGE_EVENTS,
        "manageThreads" => Permissions::MANAGE_THREADS,
        "moderateMembers" => Permissions::MODERATE_MEMBERS,
        _ => return None,
    })
}

impl Modmail {
    /// Permissions d'un membre du serveur inbox, calculées depuis le cache des rôles.
    fn member_permissions(
        &self,
        ctx: &Ctx,
        user_id: Id<UserMarker>,
        roles: &[Id<twilight_model::id::marker::RoleMarker>],
    ) -> Permissions {
        let guild_id = self.config.inbox_server_id;
        if ctx.cache.guild(guild_id).is_some_and(|g| g.owner_id() == user_id) {
            return Permissions::all();
        }
        let everyone =
            ctx.cache.role(guild_id.cast()).map(|r| r.resource().permissions).unwrap_or_else(Permissions::empty);
        let permissions = roles
            .iter()
            .filter_map(|id| ctx.cache.role(*id))
            .fold(everyone, |acc, role| acc | role.resource().permissions);
        if permissions.contains(Permissions::ADMINISTRATOR) { Permissions::all() } else { permissions }
    }

    /// `isStaff` : propriétaire du serveur, ou une entrée de `inboxServerPermission`
    /// (identifiant d'utilisateur ou de rôle, ou nom de permission).
    pub fn is_staff_member(
        &self,
        ctx: &Ctx,
        user_id: Id<UserMarker>,
        roles: &[Id<twilight_model::id::marker::RoleMarker>],
    ) -> bool {
        if self.config.inbox_server_permission.is_empty() {
            return true;
        }
        if ctx.cache.guild(self.config.inbox_server_id).is_some_and(|g| g.owner_id() == user_id) {
            return true;
        }
        let permissions = self.member_permissions(ctx, user_id, roles);
        self.config.inbox_server_permission.iter().any(|entry| {
            if super::util::is_snowflake(entry) {
                entry == &user_id.to_string() || roles.iter().any(|r| r.to_string() == *entry)
            } else {
                permission_from_name(entry).is_some_and(|p| permissions.contains(p))
            }
        })
    }

    pub fn is_staff(&self, ctx: &Ctx, msg: &Message) -> bool {
        let roles = msg.member.as_ref().map(|m| m.roles.as_slice()).unwrap_or_default();
        msg.member.is_some() && self.is_staff_member(ctx, msg.author.id, roles)
    }

    pub async fn on_event(&self, ctx: &Ctx, event: &Event) {
        let result = match event {
            Event::MessageCreate(message) => self.on_message(ctx, &message.0).await,
            Event::MessageUpdate(update) => self.on_message_update(ctx, &update.0).await,
            Event::MessageDelete(deleted) => self.on_message_delete(ctx, &deleted.id.to_string()).await,
            Event::ChannelDelete(channel) => self.on_channel_delete(ctx, channel.guild_id, channel.id).await,
            Event::ThreadDelete(thread) => self.on_channel_delete(ctx, Some(thread.guild_id), thread.id).await,
            Event::MemberAdd(member) => self.on_member_add(ctx, member.guild_id, &member.member.user).await,
            Event::MemberRemove(removed) => self.on_member_remove(ctx, removed.guild_id, removed.user.id).await,
            Event::BanAdd(ban) => self.on_ban(ctx, ban.guild_id, ban.user.id, true).await,
            Event::BanRemove(ban) => self.on_ban(ctx, ban.guild_id, ban.user.id, false).await,
            Event::ReactionAdd(reaction) => self.on_reaction(ctx, &reaction.0, true).await,
            Event::ReactionRemove(reaction) => self.on_reaction(ctx, &reaction.0, false).await,
            Event::TypingStart(typing) => self.on_typing(ctx, typing.channel_id, typing.guild_id, typing.user_id).await,
            _ => Ok(()),
        };
        if let Err(error) = result {
            tracing::error!(?error, "erreur du module ticket");
        }
    }

    async fn on_message(&self, ctx: &Ctx, msg: &Message) -> Result<()> {
        if msg.author.id == ctx.bot_user_id {
            return Ok(());
        }

        if msg.guild_id.is_none() {
            // Message privé : relais vers le ticket (et commande `!close` éventuelle).
            self.on_direct_message(ctx, msg).await?;
            if !msg.author.bot {
                self.handle_command(ctx, msg).await?;
            }
            return Ok(());
        }

        if msg.guild_id == Some(self.config.inbox_server_id) {
            self.on_inbox_message(ctx, msg).await?;
        }
        if !msg.author.bot && !msg.content.is_empty() {
            self.handle_command(ctx, msg).await?;
            self.on_snippet(ctx, msg).await?;
        }
        if msg.guild_id.is_some_and(|g| self.config.is_main_server(g)) {
            self.on_bot_mention(ctx, msg).await?;
        }
        Ok(())
    }

    /// Message dans un salon de ticket : commande (enregistrée comme telle),
    /// réponse automatique (`alwaysReply`) ou discussion interne du staff.
    async fn on_inbox_message(&self, ctx: &Ctx, msg: &Message) -> Result<()> {
        let Some(thread) = store::find_by_channel_id(&self.db, &msg.channel_id.to_string()).await? else {
            return Ok(());
        };
        let is_command =
            msg.content.starts_with(&self.config.prefix) || msg.content.starts_with(&self.config.snippet_prefix);
        if !msg.author.bot && is_command {
            self.save_chat_message(&thread, msg, message_type::COMMAND).await
        } else if !msg.author.bot && self.config.always_reply {
            if !self.is_staff(ctx, msg) {
                return Ok(());
            }
            let moderator = Moderator {
                user: msg.author.clone(),
                nick: msg.member.as_ref().and_then(|m| m.nick.clone()),
                roles: msg.member.as_ref().map(|m| m.roles.clone()).unwrap_or_default(),
            };
            let reference = msg.reference.as_ref().and_then(|r| r.message_id);
            let replied = self
                .reply_to_user(
                    ctx,
                    &thread,
                    &moderator,
                    StaffReply {
                        text: msg.content.trim(),
                        attachments: &msg.attachments,
                        anonymous: self.config.always_reply_anon,
                        reference,
                    },
                )
                .await?;
            if replied {
                let _ = ctx.http.delete_message(msg.channel_id, msg.id).await;
            }
            Ok(())
        } else {
            self.save_chat_message(&thread, msg, message_type::CHAT).await
        }
    }

    async fn on_direct_message(&self, ctx: &Ctx, msg: &Message) -> Result<()> {
        if msg.author.bot || !matches!(msg.kind, MessageType::Regular | MessageType::Reply) {
            return Ok(());
        }
        if store::is_blocked(&self.db, &msg.author.id.to_string()).await? {
            if let Some(reply) = &self.config.blocked_reply {
                let _ = ctx.http.create_message(msg.channel_id).content(reply).await;
            }
            return Ok(());
        }

        // File unique : plusieurs messages rapprochés ne créent pas plusieurs tickets.
        let _queue = self.dm_queue.lock().await;
        let existing = store::find_open_by_user_id(&self.db, &msg.author.id.to_string()).await?;
        let is_new = existing.is_none();
        let thread = match existing {
            Some(thread) => thread,
            None => {
                // « ok », « merci »… n'ouvrent pas de ticket.
                let text = msg.content.trim().to_lowercase();
                if self.config.ignore_accidental_threads
                    && !msg.content.is_empty()
                    && ACCIDENTAL_THREAD_MESSAGES.contains(&text.as_str())
                {
                    return Ok(());
                }
                match self.create_new_thread_for_user(ctx, &msg.author, NewThreadOptions::default()).await? {
                    Some(thread) => thread,
                    None => return Ok(()),
                }
            }
        };

        self.receive_user_reply(ctx, &thread, msg, false).await?;

        if is_new
            && let Some(response) = &self.config.response_message
            && let Err(error) = self
                .send_system_message_to_user(
                    ctx,
                    &thread,
                    response,
                    self.config.show_response_message_in_thread_channel,
                )
                .await
        {
            self.post_system_message(
                ctx,
                &thread,
                &format!("**NOTE :** impossible d'envoyer la réponse automatique à l'utilisateur. Erreur : `{error}`"),
            )
            .await?;
        }
        Ok(())
    }

    /// Utilisation d'un snippet (`!!trigger` ou `!!!trigger` en anonyme).
    async fn on_snippet(&self, ctx: &Ctx, msg: &Message) -> Result<()> {
        if !self.config.allow_snippets || msg.guild_id != Some(self.config.inbox_server_id) || !self.is_staff(ctx, msg)
        {
            return Ok(());
        }
        let (prefix, anon) = (&self.config.snippet_prefix, &self.config.snippet_prefix_anon);
        let (used_prefix, mut anonymous) = if anon.len() > prefix.len() {
            if msg.content.starts_with(anon.as_str()) { (anon, true) } else { (prefix, false) }
        } else if msg.content.starts_with(prefix.as_str()) {
            (prefix, false)
        } else {
            (anon, true)
        };
        if !msg.content.starts_with(used_prefix.as_str()) {
            return Ok(());
        }
        if self.config.force_anon {
            anonymous = true;
        }

        let Some(thread) = store::find_by_channel_id(&self.db, &msg.channel_id.to_string()).await? else {
            return Ok(());
        };
        let invoke = &msg.content[used_prefix.len()..];
        let Some(trigger) = invoke.split_whitespace().next() else {
            return Ok(());
        };
        let Some(snippet) = store::snippet(&self.db, trigger).await? else {
            return Ok(());
        };
        let raw_args = invoke.trim_start()[trigger.len()..].trim();
        let rendered = render_snippet(&snippet.body, &snippet_args(raw_args));

        let moderator = Moderator {
            user: msg.author.clone(),
            nick: msg.member.as_ref().and_then(|m| m.nick.clone()),
            roles: msg.member.as_ref().map(|m| m.roles.clone()).unwrap_or_default(),
        };
        let reference = msg.reference.as_ref().and_then(|r| r.message_id);
        if self
            .reply_to_user(
                ctx,
                &thread,
                &moderator,
                StaffReply { text: &rendered, attachments: &[], anonymous, reference },
            )
            .await?
        {
            let _ = ctx.http.delete_message(msg.channel_id, msg.id).await;
        }
        Ok(())
    }

    /// Mention du bot sur un serveur principal : alerte dans le salon de logs.
    async fn on_bot_mention(&self, ctx: &Ctx, msg: &Message) -> Result<()> {
        if msg.author.bot || !msg.mentions.iter().any(|m| m.id == ctx.bot_user_id) {
            return Ok(());
        }
        if msg.guild_id == Some(self.config.inbox_server_id) {
            // Même serveur : un membre du staff qui mentionne le bot est ignoré.
            if self.is_staff(ctx, msg) {
                return Ok(());
            }
        } else if let Some(member) = ctx.cache.member(self.config.inbox_server_id, msg.author.id)
            && self.is_staff_member(ctx, msg.author.id, member.roles())
        {
            return Ok(());
        }
        if store::is_blocked(&self.db, &msg.author.id.to_string()).await? {
            return Ok(());
        }

        let guild_id = msg.guild_id.expect("message de serveur");
        let (staff_mention, mentions) = if self.config.ping_on_bot_mention {
            mention_roles(&self.config.mention_role)
        } else {
            (String::new(), Mentions::None)
        };
        let user = format!("**{}** (`{}`)", msg.author.name, msg.author.id);
        let link = format!("https://discord.com/channels/{guild_id}/{}/{}", msg.channel_id, msg.id);
        let content = if self.config.main_server_ids.len() == 1 {
            format!(
                "{staff_mention}Bot mentionné dans <#{}> par {user} : \"{}\"\n\n<{link}>",
                msg.channel_id, msg.content
            )
        } else {
            let guild_name = ctx.cache.guild(guild_id).map(|g| g.name().to_owned()).unwrap_or_default();
            format!(
                "{staff_mention}Bot mentionné dans <#{}> ({guild_name}) par {user} : \"{}\"\n\n<{link}>",
                msg.channel_id, msg.content
            )
        };
        let allowed = mentions.to_allowed();
        for chunk in super::util::chunk_message_lines(&content) {
            ctx.http
                .create_message(self.config.log_channel_id)
                .content(&chunk)
                .allowed_mentions(Some(&allowed))
                .await?;
        }

        if let Some(response) = &self.config.bot_mention_response {
            let allowed =
                Mentions::Explicit { users: vec![msg.author.id], roles: Vec::new(), everyone: false }.to_allowed();
            let text = response.replace("{userMention}", &format!("<@{}>", msg.author.id));
            let _ = ctx.http.create_message(msg.channel_id).content(&text).allowed_mentions(Some(&allowed)).await;
        }

        if self.config.create_thread_on_mention
            && store::find_open_by_user_id(&self.db, &msg.author.id.to_string()).await?.is_none()
            && let Some(thread) = self
                .create_new_thread_for_user(
                    ctx,
                    &msg.author,
                    NewThreadOptions { quiet: true, ignore_requirements: false },
                )
                .await?
        {
            self.post_system_message(
                ctx,
                &thread,
                &format!("Ce ticket a été ouvert suite à une mention du bot dans <#{}>", msg.channel_id),
            )
            .await?;
            self.receive_user_reply(ctx, &thread, msg, false).await?;
        }
        Ok(())
    }

    /// Message modifié : édition côté utilisateur, ou discussion du staff à mettre à jour.
    async fn on_message_update(&self, ctx: &Ctx, msg: &Message) -> Result<()> {
        if msg.content.is_empty() {
            return Ok(());
        }
        let Some(stored) = store::message_by_dm_id(&self.db, &msg.id.to_string()).await? else {
            return Ok(());
        };
        let Some(thread) = store::find_by_id(&self.db, &stored.thread_id).await? else {
            return Ok(());
        };
        if thread.is_closed() {
            return Ok(());
        }

        if stored.is_from_user() {
            let edit = super::util::disable_link_previews(&format!(
                "**L'utilisateur a modifié son message :**\n`Avant :` {}\n`Après :` {}",
                stored.body(),
                msg.content
            ));
            if self.config.update_messages_live {
                // Les logs gardent l'original : l'édition y est ajoutée comme message système.
                self.add_system_message_to_logs(&thread, &edit).await?;
                let edited = super::models::ThreadMessage { body: Some(msg.content.clone()), ..stored.clone() };
                let content = super::formatters::user_reply_thread_message(&self.config, &edited);
                if let (Some(channel), Some(id)) = (
                    thread.channel_id.as_deref().and_then(parse_id),
                    stored.inbox_message_id.as_deref().and_then(parse_id),
                ) {
                    let _ = ctx.http.update_message(channel, id).content(Some(&content)).await;
                }
            } else {
                self.post_system_message(ctx, &thread, &edit).await?;
            }
        }
        if stored.is_chat() {
            store::update_chat_message(&self.db, &thread.id, &msg.id.to_string(), &msg.content).await?;
        }
        Ok(())
    }

    async fn on_message_delete(&self, ctx: &Ctx, message_id: &str) -> Result<()> {
        let Some(stored) = store::message_by_dm_id(&self.db, message_id).await? else {
            return Ok(());
        };
        let Some(thread) = store::find_by_id(&self.db, &stored.thread_id).await? else {
            return Ok(());
        };
        if thread.is_closed() {
            return Ok(());
        }
        if stored.is_from_user()
            && self.config.update_messages_live
            && let (Some(channel), Some(id)) =
                (thread.channel_id.as_deref().and_then(parse_id), stored.inbox_message_id.as_deref().and_then(parse_id))
        {
            let _ = ctx.http.delete_message(channel, id).await;
        }
        if stored.is_chat() {
            store::delete_chat_message(&self.db, &thread.id, message_id).await?;
        }
        Ok(())
    }

    /// Salon (ou fil de forum) de ticket supprimé à la main : le ticket est fermé.
    async fn on_channel_delete(
        &self,
        ctx: &Ctx,
        guild_id: Option<Id<twilight_model::id::marker::GuildMarker>>,
        channel_id: Id<twilight_model::id::marker::ChannelMarker>,
    ) -> Result<()> {
        if guild_id != Some(self.config.inbox_server_id) {
            return Ok(());
        }
        let Some(thread) =
            store::find_by_channel_and_status(&self.db, &channel_id.to_string(), super::models::thread_status::OPEN)
                .await?
        else {
            return Ok(());
        };
        tracing::info!(user = %thread.user_name, "salon supprimé, fermeture automatique du ticket");
        if let Some(message) = &self.config.close_message {
            let _ = self.send_system_message_to_user(ctx, &thread, message, true).await;
        }
        self.close_thread(ctx, &thread, true, false).await?;
        self.send_close_notification(
            ctx,
            &thread,
            &format!(
                "Ticket modmail n°{} avec {} ({}) fermé automatiquement car le salon a été supprimé",
                thread.number(),
                thread.user_name,
                thread.user_id
            ),
        )
        .await
    }

    async fn on_member_add(
        &self,
        ctx: &Ctx,
        guild_id: Id<GuildMarker>,
        user: &twilight_model::user::User,
    ) -> Result<()> {
        if self.config.enable_greeting {
            self.send_greeting(ctx, guild_id, user.id).await;
        }
        if self.config.notify_on_main_server_join
            && self.config.is_main_server(guild_id)
            && let Some(thread) = store::find_open_by_user_id(&self.db, &user.id.to_string()).await?
        {
            let name = ctx.cache.guild(guild_id).map(|g| g.name().to_owned()).unwrap_or_default();
            self.post_system_message(ctx, &thread, &format!("***L'utilisateur a rejoint le serveur {name}.***"))
                .await?;
        }
        Ok(())
    }

    async fn send_greeting(&self, ctx: &Ctx, guild_id: Id<GuildMarker>, user_id: Id<UserMarker>) {
        let Some(greeting) = self.config.server_greetings.get(&guild_id.to_string()) else {
            return;
        };
        if greeting.message.is_none() && greeting.attachment.is_none() {
            return;
        }
        let Ok(channel) = self.dm_channel(ctx, user_id).await else {
            return;
        };
        let mut uploads = Vec::new();
        if let Some(path) = &greeting.attachment
            && let Ok(bytes) = tokio::fs::read(path).await
        {
            let name = std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "file".to_owned());
            uploads.push(twilight_model::http::attachment::Attachment::from_bytes(name, bytes, 0));
        }
        let mut request = ctx.http.create_message(channel).content(greeting.message.as_deref().unwrap_or_default());
        if !uploads.is_empty() {
            request = request.attachments(&uploads);
        }
        // 50007 : messages privés fermés, cas normal.
        let _ = request.await;
    }

    async fn on_member_remove(&self, ctx: &Ctx, guild_id: Id<GuildMarker>, user_id: Id<UserMarker>) -> Result<()> {
        if !self.config.notify_on_main_server_leave || !self.config.is_main_server(guild_id) {
            return Ok(());
        }
        // Laisse passer un éventuel événement de bannissement (course entre les deux).
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let Some(thread) = store::find_open_by_user_id(&self.db, &user_id.to_string()).await? else {
            return Ok(());
        };
        if self.leave_ignore.lock().expect("leave_ignore").remove(&user_id) {
            return Ok(());
        }
        let name = ctx.cache.guild(guild_id).map(|g| g.name().to_owned()).unwrap_or_default();
        self.post_system_message(ctx, &thread, &format!("***L'utilisateur a quitté le serveur {name}.***"))
            .await
            .map(drop)
    }

    async fn on_ban(&self, ctx: &Ctx, guild_id: Id<GuildMarker>, user_id: Id<UserMarker>, banned: bool) -> Result<()> {
        if !self.config.is_main_server(guild_id) {
            return Ok(());
        }
        if banned && !self.config.notify_on_main_server_leave || !banned && !self.config.notify_on_main_server_join {
            return Ok(());
        }
        let Some(thread) = store::find_open_by_user_id(&self.db, &user_id.to_string()).await? else {
            return Ok(());
        };
        let name = ctx.cache.guild(guild_id).map(|g| g.name().to_owned()).unwrap_or_default();
        let text = if banned {
            self.leave_ignore.lock().expect("leave_ignore").insert(user_id);
            format!("***L'utilisateur a été banni du serveur {name}.***")
        } else {
            format!("***L'utilisateur a été débanni du serveur {name}.***")
        };
        self.post_system_message(ctx, &thread, &text).await.map(drop)
    }

    /// Réaction de l'utilisateur sur une réponse du staff, en message privé.
    async fn on_reaction(&self, ctx: &Ctx, reaction: &GatewayReaction, added: bool) -> Result<()> {
        if reaction.guild_id.is_some()
            || (added && !self.config.notify_on_reaction)
            || (!added && !self.config.notify_on_reaction_removal)
        {
            return Ok(());
        }
        let user = match ctx.cache.user(reaction.user_id).map(|u| u.clone()) {
            Some(user) => user,
            None => ctx.http.user(reaction.user_id).await?.model().await?,
        };
        if user.bot {
            return Ok(());
        }
        let Some(stored) = store::message_by_dm_id(&self.db, &reaction.message_id.to_string()).await? else {
            return Ok(());
        };
        let Some(thread) = store::find_by_id(&self.db, &stored.thread_id).await? else {
            return Ok(());
        };
        if thread.is_closed() || stored.message_type != message_type::TO_USER {
            return Ok(());
        }
        let emoji = match &reaction.emoji {
            EmojiReactionType::Custom { id, name, .. } => format!("<:{}:{id}>", name.as_deref().unwrap_or("_")),
            EmojiReactionType::Unicode { name } => name.clone(),
        };
        let name = self.display_name(&user);
        let number = stored.message_number.unwrap_or_default();
        let text = if added {
            format!("{name} a réagi au message {number} avec {emoji}")
        } else {
            format!("{name} a retiré la réaction {emoji} du message {number}")
        };
        self.post_system_message(ctx, &thread, &text).await.map(drop)
    }

    /// Relais de l'indicateur « en train d'écrire » (`typingProxy`, `typingProxyReverse`).
    async fn on_typing(
        &self,
        ctx: &Ctx,
        channel_id: twilight_model::id::Id<twilight_model::id::marker::ChannelMarker>,
        guild_id: Option<Id<GuildMarker>>,
        user_id: Id<UserMarker>,
    ) -> Result<()> {
        if self.config.typing_proxy && guild_id.is_none() {
            if let Some(thread) = store::find_open_by_user_id(&self.db, &user_id.to_string()).await?
                && let Some(channel) = thread.channel_id.as_deref().and_then(parse_id)
            {
                let _ = ctx.http.create_typing_trigger(channel).await;
            }
        } else if self.config.typing_proxy_reverse && guild_id.is_some() {
            if ctx.cache.user(user_id).is_some_and(|u| u.bot) {
                return Ok(());
            }
            if let Some(thread) = store::find_by_channel_id(&self.db, &channel_id.to_string()).await?
                && let Some(user) = parse_id(&thread.user_id)
                && let Ok(dm) = self.dm_channel(ctx, user).await
            {
                let _ = ctx.http.create_typing_trigger(dm).await;
            }
        }
        Ok(())
    }

    /// Notification de fermeture dans le salon de logs, avec le lien ou le fichier du log.
    pub async fn send_close_notification(&self, ctx: &Ctx, thread: &super::models::Thread, body: &str) -> Result<()> {
        let messages = store::messages(&self.db, &thread.id).await?;
        let count = |kind| messages.iter().filter(|m| m.message_type == kind).count();
        let plural = |n: usize| if n > 1 { "s" } else { "" };
        let (from_user, to_user, chat) =
            (count(message_type::FROM_USER), count(message_type::TO_USER), count(message_type::CHAT));
        let body = format!(
            "{body}\n**{from_user}** message{} de l'utilisateur, **{to_user}** message{} à l'utilisateur et **{chat}** message{} de discussion interne.",
            plural(from_user),
            plural(to_user),
            plural(chat)
        );

        let log_channel = self.config.log_channel_id;
        if let Some(url) = self.log_url(thread).await? {
            ctx.http.create_message(log_channel).content(&format!("{body}\nLogs: {url}")).await?;
            return Ok(());
        }
        if let Some(file) = self.log_file(thread).await? {
            let uploads =
                [twilight_model::http::attachment::Attachment::from_bytes(file.name, file.content.into_bytes(), 0)];
            ctx.http.create_message(log_channel).content(&body).attachments(&uploads).await?;
            return Ok(());
        }
        ctx.http.create_message(log_channel).content(&body).await?;
        Ok(())
    }
}

/// Arguments d'un snippet (`!!hi "John Doe" 2`), guillemets respectés.
fn snippet_args(raw: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut has_token = false;
    for ch in raw.chars() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => {
                quote = Some(ch);
                has_token = true;
            }
            None if ch.is_whitespace() => {
                if has_token {
                    args.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            None => {
                current.push(ch);
                has_token = true;
            }
        }
    }
    if has_token {
        args.push(current);
    }
    args
}

/// Remplace `{1}`, `{2}`… par les arguments ; `\{` produit une accolade littérale.
fn render_snippet(body: &str, args: &[String]) -> String {
    static PLACEHOLDER: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"(\\?)\{(\d+)\}").expect("regex valide"));
    let rendered = PLACEHOLDER.replace_all(body, |caps: &regex::Captures<'_>| {
        if !caps[1].is_empty() {
            return caps[0].to_owned();
        }
        let index: usize = caps[2].parse().unwrap_or(0);
        index.checked_sub(1).and_then(|i| args.get(i)).cloned().unwrap_or_else(|| caps[0].to_owned())
    });
    rendered.replace("\\{", "{")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_snippets_with_arguments() {
        let args = snippet_args("\"John Doe\" 2");
        assert_eq!(args, vec!["John Doe".to_owned(), "2".to_owned()]);
        assert_eq!(render_snippet("Hi {1}, you have {2} {3} \\{1}", &args), "Hi John Doe, you have 2 {3} {1}");
    }
}
