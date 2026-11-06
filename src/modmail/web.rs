//! Serveur web du module ticket : logs (HTML lisible ou texte brut au format
//! modmailbot), pièces jointes stockées localement, politique de confidentialité.

use super::{
    formatters::{LogOptions, format_log, shown_in_simple_view, summarise_embeds},
    models::{ThreadMessage, message_type},
    store,
    util::parse_db_time,
};
use crate::{
    context::Context,
    web::{
        self, MaybeViewer,
        markdown::{self, Resolver},
    },
};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, Uri, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
};
use chrono::{DateTime, Datelike, Utc};
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

/// Style d'un message selon son type. Les classes sont écrites en entier : le
/// compilateur Tailwind les lit ici (`@source`).
#[derive(Clone, Copy)]
pub(crate) enum Tone {
    User,
    Staff,
    Internal,
    Command,
    System,
    Danger,
}

impl Tone {
    /// Clé du type de message dans l'API (`LogEntry.kind`).
    pub fn key(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Staff => "staff",
            Self::Internal => "internal",
            Self::Command => "command",
            Self::System => "system",
            Self::Danger => "danger",
        }
    }
}

pub(crate) struct FileLink {
    pub(crate) url: String,
    pub(crate) name: String,
}

pub(crate) struct Edit {
    pub(crate) before: String,
    pub(crate) after: String,
}

/// Message du log, mis en forme : commun à la page HTML et à l'API.
pub(crate) struct Entry {
    pub(crate) id: i64,
    /// Premier message d'un nouveau jour : libellé du jour.
    pub(crate) day: Option<String>,
    pub(crate) tone: Tone,
    pub(crate) author: String,
    pub(crate) role: Option<String>,
    pub(crate) badge: &'static str,
    pub(crate) number: Option<i64>,
    pub(crate) created: Option<DateTime<Utc>>,
    pub(crate) body_html: String,
    pub(crate) monospace: bool,
    pub(crate) edit: Option<Edit>,
    pub(crate) images: Vec<String>,
    pub(crate) files: Vec<FileLink>,
    pub(crate) embeds: Option<String>,
    pub(crate) verbose_ids: Option<String>,
}

const MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

fn day_label(date: &DateTime<Utc>) -> String {
    format!("{} {} {}", date.day(), MONTHS[date.month0() as usize], date.year())
}

fn is_image(url: &str) -> bool {
    let path = url.split(['?', '#']).next().unwrap_or_default().to_lowercase();
    [".png", ".jpg", ".jpeg", ".gif", ".webp", ".avif"].iter().any(|ext| path.ends_with(ext))
}

fn file_name(url: &str) -> String {
    url.split(['?', '#']).next().unwrap_or(url).rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(url).to_owned()
}

fn entry_for(message: &ThreadMessage, verbose: bool, resolver: &dyn Resolver) -> Entry {
    let render_body = |text: &str| markdown::render(text, resolver);
    let (tone, badge) = match message.message_type {
        message_type::FROM_USER => (Tone::User, "Utilisateur"),
        message_type::TO_USER if message.is_anonymous != 0 => (Tone::Staff, "Staff · anonyme"),
        message_type::TO_USER => (Tone::Staff, "Staff"),
        message_type::CHAT => (Tone::Internal, "Note interne"),
        message_type::COMMAND => (Tone::Command, "Commande"),
        message_type::SYSTEM_TO_USER => (Tone::System, "Bot → utilisateur"),
        message_type::REPLY_EDITED => (Tone::System, "Réponse modifiée"),
        message_type::REPLY_DELETED => (Tone::Danger, "Réponse supprimée"),
        message_type::LEGACY => (Tone::System, "Ancien format"),
        _ => (Tone::System, "Bot"),
    };

    let created = parse_db_time(&message.created_at);
    let mut author = if message.user_name.is_empty() { "Modmail".to_owned() } else { message.user_name.clone() };
    let mut body = message.body().to_owned();
    let mut edit = None;
    match message.message_type {
        message_type::REPLY_EDITED => {
            let original = message.original().unwrap_or_default();
            author = original.user_name.clone();
            let new_body =
                message.metadata_value("newBody").and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default();
            edit = Some(Edit { before: render_body(original.body()), after: render_body(&new_body) });
        }
        message_type::REPLY_DELETED => {
            let original = message.original().unwrap_or_default();
            author = original.user_name.clone();
            body = original.body().to_owned();
        }
        _ => {}
    }

    let (images, files): (Vec<String>, Vec<String>) =
        message.attachment_links().into_iter().partition(|url| is_image(url));
    let embeds = [
        summarise_embeds(message.metadata_value("embeds").as_ref(), "Embed envoyé"),
        summarise_embeds(message.metadata_value("forwardedEmbeds").as_ref(), "Embed transféré"),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n");

    let verbose_ids = verbose
        .then(|| {
            let mut parts = Vec::new();
            if let Some(id) = &message.dm_channel_id {
                parts.push(format!("DM CHA {id}"));
            }
            if let Some(id) = &message.dm_message_id {
                parts.push(format!("DM MSG {id}"));
            }
            parts.join(" · ")
        })
        .filter(|s| !s.is_empty());

    Entry {
        id: message.id,
        day: None,
        tone,
        role: (message.message_type == message_type::TO_USER).then(|| message.role_name.clone()).flatten(),
        author,
        badge,
        number: if matches!(message.message_type, message_type::TO_USER) { message.message_number } else { None },
        created,
        body_html: render_body(&body),
        monospace: matches!(message.message_type, message_type::COMMAND | message_type::LEGACY),
        edit,
        images,
        files: files.into_iter().map(|url| FileLink { name: file_name(&url), url }).collect(),
        embeds: (!embeds.is_empty()).then_some(embeds),
        verbose_ids,
    }
}

/// Messages du log mis en forme, avec un séparateur au premier message de chaque jour.
pub(crate) fn build_entries(messages: &[ThreadMessage], options: LogOptions, resolver: &dyn Resolver) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut last_day = None;
    for message in messages.iter().filter(|m| !options.simple || shown_in_simple_view(m)) {
        let mut entry = entry_for(message, options.verbose, resolver);
        if let Some(date) = parse_db_time(&message.created_at) {
            let day = day_label(&date);
            if last_day.as_ref() != Some(&day) {
                entry.day = Some(day.clone());
                last_day = Some(day);
            }
        }
        entries.push(entry);
    }
    entries
}

#[derive(Deserialize)]
struct LogQuery {
    simple: Option<String>,
    verbose: Option<String>,
    raw: Option<String>,
}

pub(crate) fn flag(value: Option<&String>) -> bool {
    value.is_some_and(|v| !v.is_empty() && v != "0" && v != "false")
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "Page Not Found").into_response()
}

async fn serve_log(
    State(ctx): State<Context>,
    viewer: MaybeViewer,
    uri: Uri,
    Path(thread_id): Path<String>,
    Query(query): Query<LogQuery>,
) -> Response {
    let Some(modmail) = ctx.modmail.as_ref() else {
        return not_found();
    };
    // Sans `?raw`, la page est celle de l'interface React (elle lit `/api/logs/:id`).
    if !flag(query.raw.as_ref()) {
        return web::spa::index(&ctx).await;
    }
    // Connexion configurée : logs réservés au staff. Sinon, lien non devinable (modmailbot).
    if web::auth::enabled(&ctx) {
        let Some(viewer) = viewer.0 else {
            let next = uri.path_and_query().map_or("/logs", |p| p.as_str());
            return Redirect::to(&web::auth::login_href(next)).into_response();
        };
        if !web::Access::of(&ctx, viewer.id).await.tickets {
            return (StatusCode::FORBIDDEN, "Accès réservé au staff des tickets.").into_response();
        }
    }
    let Ok(Some(thread)) = store::find_by_id(&modmail.db, &thread_id).await else {
        return not_found();
    };
    let Ok(mut messages) = store::messages(&modmail.db, &thread.id).await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response();
    };
    let options = LogOptions { simple: flag(query.simple.as_ref()), verbose: flag(query.verbose.as_ref()) };

    // Format texte exact de modmailbot, embeds résumés dans le corps.
    for message in &mut messages {
        let extra = [
            summarise_embeds(message.metadata_value("embeds").as_ref(), "User sent an embed"),
            summarise_embeds(message.metadata_value("forwardedEmbeds").as_ref(), "User forwarded an embed"),
        ];
        if extra.iter().any(|s| !s.is_empty()) {
            let body = std::iter::once(message.body().to_owned())
                .chain(extra)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            message.body = Some(body);
        }
    }
    let text = format_log(&thread, &messages, options);
    ([(header::CONTENT_TYPE, "text/plain; charset=UTF-8")], text).into_response()
}

async fn serve_attachment(
    State(ctx): State<Context>,
    Path((attachment_id, filename)): Path<(String, String)>,
) -> Response {
    let Some(modmail) = ctx.modmail.as_ref() else {
        return not_found();
    };
    static FILENAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[0-9a-z._-]+$").expect("regex valide"));
    if !attachment_id.bytes().all(|b| b.is_ascii_digit()) || !FILENAME.is_match(&filename) {
        return not_found();
    }
    let Ok(bytes) = tokio::fs::read(modmail.local_attachment_path(&attachment_id)).await else {
        return not_found();
    };
    let ext = filename.rsplit('.').next().unwrap_or("bin").to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "pdf" => "application/pdf",
        "txt" | "log" => "text/plain; charset=UTF-8",
        _ => "application/octet-stream",
    };
    ([(header::CONTENT_TYPE, mime)], bytes).into_response()
}

async fn serve_privacy_policy() -> Response {
    match tokio::fs::read_to_string("PRIVACY_POLICY.md").await {
        Ok(text) => Html(format!(
            "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
             <title>Privacy policy</title><style>body{{font:16px/1.6 system-ui,sans-serif;max-width:48rem;margin:3rem auto;padding:0 1rem}}</style></head>\
             <body><main>{}</main></body></html>",
            markdown::render(&text, &markdown::NoResolver)
        ))
        .into_response(),
        Err(_) => not_found(),
    }
}

pub fn routes() -> Router<Context> {
    Router::new()
        .route("/logs/{thread_id}", get(serve_log))
        .route("/attachments/{attachment_id}/{filename}", get(serve_attachment))
        .route("/privacy-policy", get(serve_privacy_policy))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_images() {
        assert!(is_image("https://cdn.discordapp.com/a/b/c.PNG?ex=1"));
        assert!(!is_image("https://cdn.discordapp.com/a/b/c.pdf"));
        assert_eq!(file_name("https://x/y/report.pdf?z=1"), "report.pdf");
    }
}
