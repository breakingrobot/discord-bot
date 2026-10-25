//! `/m2t` : relance publique dans le salon courant, réservée aux gradés et leads
//! (`m2t.utiliser`, réglable avec `/config permission`).

use crate::{
    discord::{
        interaction::{GUILD_ONLY, Ix, Reply},
        messages::{Mentions, Outgoing},
    },
    modules::permissions::{self, Perm},
};
use anyhow::Result;
use twilight_model::{
    application::command::{Command, CommandType},
    id::{Id, marker::UserMarker},
};
use twilight_util::builder::command::CommandBuilder;

/// Personne relancée.
const TARGET: u64 = 994_351_106_605_973_524;

fn target() -> Id<UserMarker> {
    Id::new(TARGET)
}

/// Message publié dans le salon.
fn message() -> String {
    format!("<@{TARGET}> t’es co?")
}

pub fn definition() -> Command {
    CommandBuilder::new("m2t", "Relance la personne visée dans ce salon (gradés et leads)", CommandType::ChatInput)
        .build()
}

pub async fn handle(ix: &Ix) -> Result<()> {
    if ix.guild_id().is_none() {
        return ix.reply(Reply::ephemeral(GUILD_ONLY)).await;
    }
    if !permissions::require(ix, Perm::M2t).await? {
        return Ok(());
    }
    // Seule la personne visée est notifiée.
    let mentions = Mentions::Explicit { users: vec![target()], roles: Vec::new(), everyone: false };
    ix.reply(Reply { message: Outgoing::text(message()).with_mentions(mentions), ephemeral: false }).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pings_only_the_target() {
        assert_eq!(message(), "<@994351106605973524> t’es co?");
        assert_eq!(target().get(), TARGET);
    }

    #[test]
    fn is_a_plain_command() {
        let command = definition();
        assert_eq!(command.name, "m2t");
        assert!(command.options.is_empty());
    }
}
