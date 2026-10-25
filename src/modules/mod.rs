pub mod audit;
pub mod coffres;
pub mod contacts;
pub mod cotisations;
pub mod fiche;
pub mod halamadrid;
pub mod m2t;
pub mod permissions;
pub mod phones;
pub mod presences;
pub mod rappels;
pub mod recrues;
pub mod status;
pub mod transactions;
pub mod vehicles;

/// Motif `LIKE` « contient », avec échappement des jokers saisis par l'utilisateur.
pub fn like_contains(query: &str) -> String {
    let escaped = query.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}
