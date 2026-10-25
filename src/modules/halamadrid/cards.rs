//! Mise en forme des messages de la soirée : logos, photos, barres de tendance,
//! annonces en direct (coup d'envoi, mi-temps, fin, rouges, compos, statistiques).

use super::espn::{Lineup, RedCard, Stats};
use crate::{
    discord::embed::{EmbedBuilder, EmbedBuilderExt as _, footer, image},
    util::time::DbTime,
};
use twilight_model::channel::message::Embed;
use twilight_util::builder::embed::{EmbedAuthorBuilder, EmbedFieldBuilder};

/// Or du Real.
pub const MADRID: u32 = 0xfebe10;
/// Grenat.
pub const BARCA: u32 = 0xa50044;
/// Rouge des annonces graves (cartons, buts encaissés).
pub const ALERT: u32 = 0xe74c3c;
/// Couleur neutre des annonces masquées (ne trahit pas le camp).
pub const NEUTRAL: u32 = 0x5865f2;
/// Logo du Real (CDN d'ESPN).
pub const REAL_LOGO: &str = "https://a.espncdn.com/i/teamlogos/soccer/500/86.png";
pub const FOOTER: &str = "¡Hala Madrid y nada más!";

/// En-tête « Real Madrid » avec son logo.
pub fn real_author(text: &str) -> twilight_model::channel::message::embed::EmbedAuthor {
    let mut author = EmbedAuthorBuilder::new(text);
    if let Some(icon) = image(REAL_LOGO) {
        author = author.icon_url(icon);
    }
    author.build()
}

/// Base d'un message de la soirée : en-tête Real, couleur, vignette (logo adverse ou photo).
pub fn base(author: &str, color: u32, thumbnail: Option<&str>) -> EmbedBuilder {
    EmbedBuilder::new().author(real_author(author)).color(color).maybe_thumbnail(thumbnail.map(str::to_owned))
}

pub fn inline(name: &str, value: impl Into<String>) -> twilight_model::channel::message::embed::EmbedField {
    EmbedFieldBuilder::new(name, value).inline().build()
}

/// Barre de 10 segments pour un pourcentage : « ▰▰▰▰▰▰▱▱▱▱ 60 % ».
pub fn bar(percent: u32) -> String {
    let filled = ((percent.min(100) + 5) / 10) as usize;
    format!("{}{} {percent} %", "▰".repeat(filled), "▱".repeat(10 - filled))
}

/// Nom lisible d'une compétition ESPN.
pub fn league_label(league: Option<&str>) -> &'static str {
    match league {
        Some("esp.1") => "LaLiga",
        Some("uefa.champions") => "Ligue des champions",
        Some("esp.copa_del_rey") => "Coupe du Roi",
        Some("esp.super_cup") => "Supercoupe d'Espagne",
        Some("fifa.cwc") => "Coupe du monde des clubs",
        Some("club.friendly") => "Match amical",
        _ => "Match du Real",
    }
}

/// Texte masqué (spoiler Discord) : un clic pour le voir. Pour ceux qui regardent en différé.
pub fn spoiler(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() { String::new() } else { format!("||{}||", text.replace("||", "|")) }
}

/// Score mis en avant, masqué : « ## ||Real Madrid 2 – 1 FC Barcelone|| ».
pub fn scoreline(madrid: i64, adverse: i64, rival: &str) -> String {
    format!("## {}", spoiler(&format!("Real Madrid  {madrid} – {adverse}  {rival}")))
}

pub fn kickoff_card(rival: &str, rival_logo: Option<&str>, madridista: Option<&str>) -> (Option<String>, Embed) {
    let embed = base("Coup d'envoi", MADRID, rival_logo)
        .title(format!("C'est parti : Real Madrid – {rival} ! ⚽"))
        .description("Pronostics et paris fermés. Paris en direct : `/halamadrid prochainbut`.")
        .footer(footer(FOOTER))
        .build();
    (madridista.map(|role| format!("<@&{role}> ¡Vamos!")), embed)
}

pub fn halftime_card(madrid: i64, adverse: i64, rival: &str, rival_logo: Option<&str>, stats: &[Stats]) -> Embed {
    let mut embed =
        base("Mi-temps", MADRID, rival_logo).title("⏸️ Mi-temps").description(scoreline(madrid, adverse, rival));
    for field in stats_fields(stats) {
        embed = embed.field(field);
    }
    embed.footer(footer(FOOTER)).build()
}

pub fn full_time_card(madrid: i64, adverse: i64, rival: &str, rival_logo: Option<&str>, stats: &[Stats]) -> Embed {
    let mut embed = base("Coup de sifflet final", MADRID, rival_logo)
        .title("🏁 Fin du match")
        .description(scoreline(madrid, adverse, rival));
    for field in stats_fields(stats) {
        embed = embed.field(field);
    }
    embed.footer(footer(FOOTER)).build()
}

/// Statistiques côte à côte : Real | adversaire.
pub fn stats_fields(stats: &[Stats]) -> Vec<twilight_model::channel::message::embed::EmbedField> {
    let (Some(real), Some(rival)) = (stats.iter().find(|s| s.madrid), stats.iter().find(|s| !s.madrid)) else {
        return Vec::new();
    };
    let rows: [(&str, &Option<String>, &Option<String>, &str); 5] = [
        ("Possession", &real.possession, &rival.possession, " %"),
        ("Tirs", &real.shots, &rival.shots, ""),
        ("Tirs cadrés", &real.on_target, &rival.on_target, ""),
        ("Corners", &real.corners, &rival.corners, ""),
        ("Fautes", &real.fouls, &rival.fouls, ""),
    ];
    let lines: Vec<String> = rows
        .iter()
        .filter_map(|(label, a, b, unit)| {
            let (a, b) = (a.as_deref()?, b.as_deref()?);
            Some(format!("`{a:>5}{unit}` **{label}** `{b}{unit}`"))
        })
        .collect();
    if lines.is_empty() {
        return Vec::new();
    }
    vec![EmbedFieldBuilder::new("📊 Statistiques · Real | adversaire", spoiler(&lines.join("\n"))).build()]
}

/// Carton rouge : titre neutre, joueur, camp et minute masqués.
pub fn red_card_card(card: &RedCard, rival: &str) -> Embed {
    let side = if card.madrid { "Real Madrid".to_owned() } else { format!("{rival} 😈") };
    let minute = if card.minute.is_empty() { String::new() } else { format!(" · {}", card.minute) };
    base("Carton rouge", NEUTRAL, None)
        .title("🟥 Carton rouge")
        .description(spoiler(&format!("{} ({side}){minute}", card.player)))
        .build()
}

pub fn lineups_card(lineups: &[Lineup], rival_logo: Option<&str>) -> Option<Embed> {
    if lineups.is_empty() {
        return None;
    }
    let mut embed = base("Compositions", MADRID, rival_logo).title("📋 Les onze de départ");
    // Le Real d'abord.
    let mut sorted: Vec<&Lineup> = lineups.iter().collect();
    sorted.sort_by_key(|l| !l.madrid);
    for lineup in sorted {
        let name = if lineup.madrid { "⚪ Real Madrid".to_owned() } else { format!("🔴 {}", lineup.team) };
        embed = embed.field(inline(&name, crate::util::format::truncate(&lineup.starters.join("\n"), 1024)));
    }
    Some(embed.footer(footer(FOOTER)).build())
}

/// Horodatage Discord d'un instant.
pub fn when(time: DbTime) -> String {
    format!("<t:{}:F> · <t:{}:R>", time.unix(), time.unix())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_round_to_ten_segments() {
        assert_eq!(bar(0), "▱▱▱▱▱▱▱▱▱▱ 0 %");
        assert_eq!(bar(58), "▰▰▰▰▰▰▱▱▱▱ 58 %");
        assert_eq!(bar(100), "▰▰▰▰▰▰▰▰▰▰ 100 %");
        assert_eq!(bar(140), "▰▰▰▰▰▰▰▰▰▰ 140 %");
    }

    #[test]
    fn spoilers_hide_text() {
        assert_eq!(spoiler(" Mbappé 23' "), "||Mbappé 23'||");
        assert_eq!(spoiler(""), "");
        assert_eq!(spoiler("a||b"), "||a|b||");
        assert_eq!(scoreline(2, 1, "FC Barcelone"), "## ||Real Madrid  2 – 1  FC Barcelone||");
        let card =
            red_card_card(&RedCard { minute: "70'".into(), player: "Gavi".into(), madrid: false }, "FC Barcelone");
        assert_eq!(card.title.as_deref(), Some("🟥 Carton rouge"));
        assert_eq!(card.description.as_deref(), Some("||Gavi (FC Barcelone 😈) · 70'||"));
    }

    #[test]
    fn stats_need_both_teams() {
        let real = Stats { madrid: true, possession: Some("58".into()), shots: Some("14".into()), ..Stats::default() };
        let rival = Stats { madrid: false, possession: Some("42".into()), shots: None, ..Stats::default() };
        let fields = stats_fields(&[real.clone(), rival]);
        assert_eq!(fields.len(), 1);
        assert!(fields[0].value.contains("Possession") && !fields[0].value.contains("Tirs"));
        assert!(stats_fields(&[real]).is_empty());
    }

    #[test]
    fn lineups_put_real_first() {
        let lineups = [
            Lineup { team: "Barcelona".into(), madrid: false, starters: vec!["1 Ter Stegen".into()] },
            Lineup { team: "Real Madrid".into(), madrid: true, starters: vec!["1 Courtois".into()] },
        ];
        let embed = lineups_card(&lineups, None).unwrap();
        assert_eq!(embed.fields[0].name, "⚪ Real Madrid");
        assert!(lineups_card(&[], None).is_none());
        assert_eq!(league_label(Some("uefa.champions")), "Ligue des champions");
    }
}
