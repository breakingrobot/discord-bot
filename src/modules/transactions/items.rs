//! Liste d'objets saisie d'un coup (modèles, saisie guidée) :
//! `2x Pistolet @ 15000, Kevlar x5, Munitions`.
//!
//! Entrées séparées par `,`, `;` ou des retours à la ligne, puces ignorées.
//! Quantité en tête (`2x Pistolet`, `2 × Pistolet`) ou en fin (`Kevlar x5`),
//! 1 par défaut ; prix unitaire facultatif après `@` (`@ 15 000`, `@ 1500.50 $` : la virgule sépare les objets).

use regex::Regex;
use std::sync::LazyLock;

pub const MAX_ITEMS: usize = 25;
pub const NAME_LIMIT: usize = 100;
pub const MAX_QUANTITY: i64 = 1_000_000;
/// Prix maximal, unitaire ou total : protège les totaux d'une saisie aberrante.
pub const MAX_PRICE: f64 = 1_000_000_000_000.0;

static BULLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:[•·▪‣◦]\s*|[-*–—]\s+)").expect("regex valide"));
static QUANTITY_FIRST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9][0-9\s]*?)\s*(?:[xX]\s+|×\s*)(\S.*)$").expect("regex valide"));
static QUANTITY_LAST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*?\S)(?:\s+[xX]|\s*×)\s*([0-9][0-9\s]*)$").expect("regex valide"));

#[derive(Clone, Debug, PartialEq)]
pub struct ItemLine {
    pub nom: String,
    pub quantite: i64,
    pub prix_unitaire: Option<f64>,
}

/// Prix saisi : espaces, `_` et `$` ignorés, virgule ou point décimal.
pub fn parse_price(raw: &str) -> Option<f64> {
    let cleaned: String = raw.chars().filter(|c| !c.is_whitespace() && !matches!(c, '_' | '$')).collect();
    let value: f64 = cleaned.replace(',', ".").parse().ok()?;
    (value.is_finite() && (0.0..=MAX_PRICE).contains(&value)).then_some(value)
}

fn parse_quantity(raw: &str) -> Option<i64> {
    let digits: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    digits.parse().ok().filter(|q| (1..=MAX_QUANTITY).contains(q))
}

fn parse_entry(part: &str) -> Result<ItemLine, String> {
    let (left, price) = match part.rsplit_once('@') {
        Some((left, price)) => {
            let price = parse_price(price).ok_or_else(|| format!("« {part} » : prix unitaire illisible."))?;
            (left.trim(), Some(price))
        }
        None => (part, None),
    };
    let (nom, quantite) = if let Some(captures) = QUANTITY_FIRST.captures(left) {
        (captures.get(2).map_or("", |m| m.as_str()).trim(), parse_quantity(&captures[1]))
    } else if let Some(captures) = QUANTITY_LAST.captures(left) {
        (captures.get(1).map_or("", |m| m.as_str()).trim(), parse_quantity(&captures[2]))
    } else {
        (left, Some(1))
    };
    let quantite = quantite.ok_or_else(|| format!("« {part} » : quantité entre 1 et {MAX_QUANTITY}."))?;
    if nom.is_empty() || nom.chars().count() > NAME_LIMIT {
        return Err(format!("« {part} » : nom d'objet vide ou trop long ({NAME_LIMIT} caractères au plus)."));
    }
    Ok(ItemLine { nom: nom.to_owned(), quantite, prix_unitaire: price })
}

/// Lit une liste d'objets ; renvoie un message clair à la première entrée illisible.
pub fn parse_items(raw: &str) -> Result<Vec<ItemLine>, String> {
    let mut items = Vec::new();
    for line in raw.split([',', ';', '\n']).map(str::trim).filter(|l| !l.is_empty()) {
        let part = BULLET.replace(line, "");
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        items.push(parse_entry(part)?);
        if items.len() > MAX_ITEMS {
            return Err(format!("{MAX_ITEMS} objets au plus."));
        }
    }
    Ok(items)
}

fn price_text(value: f64) -> String {
    crate::util::format::format_price(value).replace('\u{202f}', " ").replace(',', ".")
}

/// Forme canonique d'une liste, relue telle quelle par [`parse_items`].
pub fn format_items(items: &[ItemLine]) -> String {
    items
        .iter()
        .map(|item| {
            let price = item.prix_unitaire.map(|p| format!(" @ {}", price_text(p))).unwrap_or_default();
            format!("{}x {}{price}", item.quantite, item.nom)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Prix total calculé depuis les objets : seulement si tous ont un prix unitaire.
pub fn total(items: &[ItemLine]) -> Option<f64> {
    if items.is_empty() {
        return None;
    }
    items.iter().map(|item| item.prix_unitaire.map(|p| p * item.quantite as f64)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(nom: &str, quantite: i64, prix: Option<f64>) -> ItemLine {
        ItemLine { nom: nom.into(), quantite, prix_unitaire: prix }
    }

    #[test]
    fn parses_item_lists() {
        assert_eq!(
            parse_items("2x Pistolet @ 15000, Kevlar x5\n- Munitions 9 mm; 3 × Gilet @ 1 500.50 $").unwrap(),
            [
                item("Pistolet", 2, Some(15000.0)),
                item("Kevlar", 5, None),
                item("Munitions 9 mm", 1, None),
                item("Gilet", 3, Some(1500.5)),
            ]
        );
        assert!(parse_items("").unwrap().is_empty());
        assert_eq!(parse_items("Box 4x4").unwrap(), [item("Box 4x4", 1, None)]);
        assert_eq!(parse_items("Pistolet @ cher").unwrap_err(), "« Pistolet @ cher » : prix unitaire illisible.");
        assert!(parse_items("0x Pistolet").is_err());
        assert!(parse_items(&"x, ".repeat(30)).is_err(), "25 objets au plus");
    }

    #[test]
    fn formats_and_reads_back() {
        let items = vec![item("Pistolet", 2, Some(15000.0)), item("Kevlar", 5, None)];
        let text = format_items(&items);
        assert_eq!(text, "2x Pistolet @ 15 000, 5x Kevlar");
        assert_eq!(parse_items(&text).unwrap(), items);
    }

    #[test]
    fn computes_total_only_when_every_item_has_a_price() {
        assert_eq!(total(&[item("A", 2, Some(10.0)), item("B", 1, Some(5.5))]), Some(25.5));
        assert_eq!(total(&[item("A", 2, Some(10.0)), item("B", 1, None)]), None);
        assert_eq!(total(&[]), None);
        assert_eq!(parse_price("15 000 $"), Some(15000.0));
        assert_eq!(parse_price("-3"), None);
    }
}
