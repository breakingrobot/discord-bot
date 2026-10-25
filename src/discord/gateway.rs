//! Boucle de réception des événements gateway et aiguillage.

use crate::{
    context::Context,
    discord::{commands, members},
    modules,
};
use std::sync::Arc;
use tokio::sync::watch;
use twilight_gateway::{CloseFrame, Event, EventTypeFlags, Intents, Shard, StreamExt as _};
use twilight_model::id::{Id, marker::RoleMarker};

/// Intents du bot. Les modules à slash commands n'ont besoin que des guildes et
/// des membres ; le flux des messages (serveurs, privés, contenu, réactions…)
/// n'est demandé que si le module ticket est configuré.
pub fn intents(ctx: &Context) -> Intents {
    let mut base = Intents::GUILDS | Intents::GUILD_MEMBERS;
    // Module coffre : messages des serveurs et leur contenu, seulement si l'intent
    // privilégié est activé sur l'application (sinon Discord refuserait la connexion).
    if ctx.message_content {
        base |= Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT;
    }
    match &ctx.modmail {
        Some(modmail) => base | modmail.intents(),
        None => base,
    }
}

/// Seuls ces événements sont désérialisés : les autres sont ignorés dès la
/// réception, sans allocation.
pub fn wanted_events(ctx: &Context) -> EventTypeFlags {
    let base = EventTypeFlags::READY
        | EventTypeFlags::GUILD_CREATE
        | EventTypeFlags::GUILD_DELETE
        | EventTypeFlags::GUILD_UPDATE
        | EventTypeFlags::CHANNEL_CREATE
        | EventTypeFlags::CHANNEL_DELETE
        | EventTypeFlags::CHANNEL_UPDATE
        | EventTypeFlags::ROLE_CREATE
        | EventTypeFlags::ROLE_DELETE
        | EventTypeFlags::ROLE_UPDATE
        | EventTypeFlags::MEMBER_ADD
        | EventTypeFlags::MEMBER_REMOVE
        | EventTypeFlags::MEMBER_UPDATE
        | EventTypeFlags::MEMBER_CHUNK
        | EventTypeFlags::INTERACTION_CREATE
        | EventTypeFlags::USER_UPDATE;
    let base = if ctx.message_content { base | EventTypeFlags::MESSAGE_CREATE } else { base };
    match &ctx.modmail {
        Some(_) => {
            base | EventTypeFlags::MESSAGE_CREATE
                | EventTypeFlags::MESSAGE_UPDATE
                | EventTypeFlags::MESSAGE_DELETE
                | EventTypeFlags::BAN_ADD
                | EventTypeFlags::BAN_REMOVE
                | EventTypeFlags::REACTION_ADD
                | EventTypeFlags::REACTION_REMOVE
                | EventTypeFlags::TYPING_START
                | EventTypeFlags::VOICE_STATE_UPDATE
                | EventTypeFlags::THREAD_CREATE
                | EventTypeFlags::THREAD_DELETE
                | EventTypeFlags::THREAD_UPDATE
        }
        None => base,
    }
}

pub async fn run_shard(ctx: Context, mut shard: Shard, mut shutdown: watch::Receiver<bool>) {
    let wanted = wanted_events(&ctx);
    loop {
        tokio::select! {
            _ = shutdown.changed() => shard.close(CloseFrame::NORMAL),
            item = shard.next_event(wanted) => {
                let Some(item) = item else { break };
                ctx.health.observe_shard(&shard);
                let event = match item {
                    Ok(event) => event,
                    Err(error) => {
                        tracing::warn!(?error, "erreur de réception gateway");
                        continue;
                    }
                };
                if matches!(event, Event::GatewayClose(_)) && *shutdown.borrow() {
                    break;
                }

                // Les anciens rôles d'un membre doivent être lus avant que le
                // cache n'applique la mise à jour.
                let previous_roles: Option<Vec<Id<RoleMarker>>> = match &event {
                    Event::MemberUpdate(update) => ctx
                        .cache
                        .member(update.guild_id, update.user.id)
                        .map(|member| member.roles().to_vec()),
                    _ => None,
                };

                ctx.cache.update(&event);
                tokio::spawn(handle(Arc::clone(&ctx), event, previous_roles));
            }
        }
    }
}

async fn handle(ctx: Context, event: Event, previous_roles: Option<Vec<Id<RoleMarker>>>) {
    match event {
        Event::Ready(ready) => {
            tracing::info!(user = %ready.user.name, guilds = ready.guilds.len(), "connecté à Discord");
            for guild in &ready.guilds {
                if !ctx.config.is_guild_allowed(guild.id) {
                    tracing::warn!(guild_id = %guild.id, "guilde non autorisée, départ");
                    if let Err(error) = ctx.http.leave_guild(guild.id).await {
                        tracing::error!(%error, "impossible de quitter la guilde");
                    }
                }
            }
        }
        Event::GuildCreate(guild) => {
            let guild_id = guild.id();
            if !ctx.config.is_guild_allowed(guild_id) {
                tracing::warn!(%guild_id, "guilde non autorisée, départ immédiat");
                if let Err(error) = ctx.http.leave_guild(guild_id).await {
                    tracing::error!(%error, "impossible de quitter la guilde");
                }
                return;
            }
            members::request_all_members(&ctx, guild_id);
            if ctx.message_content {
                modules::coffres::ingest::catch_up_guild(&ctx, &guild_id.to_string()).await;
            }
        }
        Event::InteractionCreate(interaction) => commands::dispatch(ctx, interaction.0).await,
        Event::MemberUpdate(update) => {
            if let Some(previous) = previous_roles {
                modules::recrues::events::on_member_update(&ctx, &update, &previous).await;
            }
            if let Some(modmail) = &ctx.modmail {
                modmail.on_event(&ctx, &Event::MemberUpdate(update)).await;
            }
        }
        other => {
            if let Event::MessageCreate(message) = &other
                && message.guild_id.is_some()
            {
                modules::coffres::ingest::on_message(&ctx, &message.0).await;
                modules::halamadrid::on_message(&ctx, &message.0).await;
            }
            if let Some(modmail) = &ctx.modmail {
                modmail.on_event(&ctx, &other).await;
            }
        }
    }
}
