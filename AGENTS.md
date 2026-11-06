# Guide des agents de code

Bot Discord en Rust (Twilight 0.17, sqlx/SQLite, axum 0.8) pour un groupe RP, avec un site React servi par le bot
(`ui/`). Ce fichier fixe les règles du dépôt pour tout agent (Claude Code, Codex, Cursor…). La documentation
utilisateur est dans `docs/`, l'architecture de l'interface dans `ui/README.md`.

## Commandes (mise)

| Tâche | Commande |
|---|---|
| Tout vérifier comme la CI (obligatoire avant de terminer) | `mise run check` |
| Côté Rust seulement · interface seulement | `mise run check:rust` · `mise run check:ui` |
| Formater | `mise run fmt` |
| Clippy + Oxlint · tests | `mise run lint` · `mise run test` |
| Après tout changement d'API (régénère `ui/openapi.json`) | `mise run openapi` |
| Dépendances Rust (failles, licences, sources) | `mise run deny` |
| Lancer en local | `mise run dev` (bot + interface), `mise run dev:mock` (interface sur maquette) |

Sans mise : `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, puis
dans `ui/` : `npm run format:check`, `lint`, `typecheck`, `check:api`, `check:compiler`, `test`, `build`.

## Carte du code

| Chemin | Rôle |
|---|---|
| `src/discord/` | Gateway, aiguillage des interactions (`commands.rs`), réponses (`interaction.rs`), composants, messages tolérants aux erreurs (`messages.rs`), liens vers le site (`site.rs`) |
| `src/modules/<module>/` | Un module = `commands.rs` (définitions + handlers), `service.rs` (SQL, logique), parfois `interactions.rs`, `cron.rs`, `embed.rs` |
| `src/modules/permissions/` | Niveaux (membre < gradé < lead < admin), macro `perms!`, garde « membre du groupe » |
| `src/modules/audit.rs` | Journal du staff (toute écriture sensible y passe) |
| `src/web/api/` | API JSON du site : une route par fichier de domaine, `openapi.rs` (utoipa), `tests.rs` |
| `src/modmail/` | Tickets (compatible Dragory/modmailbot) |
| `src/util/` | Dates (Europe/Paris), mise en forme, identifiants, client HTTP partagé |
| `migrations/` | SQL appliqué au démarrage, numéroté `NNNN_nom.sql` |
| `ui/` | React 19 + TanStack Router/Query + Tailwind v4, conception atomique (voir `ui/README.md`) |

## Règles d'or

1. **Texte visible en français.** Sur Discord et dans les erreurs d'API : tutoiement, ton direct. Sur le site : le
   vouvoiement déjà en place. Ne reprends jamais le nom du dépôt dans un texte visible (Discord, site, docs) : le
   produit s'appelle « Bot Discord » / « Bot du groupe ».
2. **Permissions.** Toute action passe par une `Perm` (ligne `perms!` dans `src/modules/permissions/mod.rs`, niveau
   par défaut ≥ Membre) et se documente dans le tableau de `docs/modules.md`. Le rôle membre est le passage obligé :
   rien n'est jamais enregistré au nom de quelqu'un hors du groupe (`permissions::require_member`, côté API
   `require_member`). Les actions *strictes* (recrues) ignorent « Gérer le serveur ».
3. **Écritures.** Une écriture sensible journalise (`audit::record`) et rafraîchit ce qu'elle touche (tableau de bord
   Discord, message, événement temps réel via les fonctions `refresh`/`sync_*` existantes). Même comportement depuis
   Discord et depuis le site.
4. **Base.** Nouvelle migration = nouveau fichier au numéro suivant ; ne modifie jamais une migration déjà sur
   `main`. Tables et colonnes en camelCase entre guillemets, comme l'existant. Requêtes toujours bornées au serveur
   (`"guildId" = ?`) : les identifiants circulent en clair dans les messages.
5. **API.** `#[utoipa::path]` + ajout dans `openapi.rs`, puis `mise run openapi` ; types dans `ui/src/api/types*.ts`,
   requêtes dans `ui/src/api/queries*.ts`, entrée dans `contract-check.ts`. Erreurs JSON `{ "error": "…" }` en
   français ; les écritures exigent JSON + `X-Requested-With: fetch` (middleware CSRF). Ne modifie jamais à la main
   `ui/openapi.json`, `ui/src/routeTree.gen.ts` ni `ui/src/api/generated/`.
6. **Discord.** Respecte les limites (25 options, 100 caractères de description, 5 boutons par ligne, embeds,
   Components V2) : le test `command_definitions_are_valid` doit passer. Erreur côté utilisateur : un message qui dit
   quoi faire (commande `/config …` exacte). Les appels REST passent par `discord::messages` (tolérant).
7. **Erreurs Rust.** `anyhow` + `?`. Pas de `unwrap()` hors des tests (lint bloquant) : `expect("raison")` pour un
   invariant réel. Pas de `println!`/`dbg!`/`todo!` (lints bloquants) : `tracing`.
8. **Heures** : toujours Europe/Paris via `util::time` (saisie `JJ/MM/AAAA`, `HH:MM`).
9. **Tests.** Logique pure testée à côté du code (`#[cfg(test)]`) ; base testée avec `crate::db::open_main` sur un
   dossier temporaire ; interface avec Vitest + Testing Library. Un bug corrigé reçoit un test.
10. **Interface.** Composants en fonctions fléchées, couches atomes → molécules → organismes → modèles → pages, route
    fine dans `routes/`, 300 lignes au plus par fichier (lint), clés TanStack `['guild', id, <domaine>, …]`, règles
    responsive de `ui/README.md` (`gutter`, `tap`, `short:`).
11. **Style.** Imite le code voisin : commentaires en français, même densité, mêmes noms. Pas de dépendance nouvelle
    sans nécessité. `rustfmt.toml` (largeur 120) et Oxfmt font foi.
12. **Commits** en français au format `type(module): résumé` (`feat`, `fix`, `refactor`, `docs`, `test`, `chore`),
    sans nom de modèle d'IA. Une PR = un sujet ; la doc (`docs/modules.md`, `docs/site-web.md`, `ui/README.md`) suit
    le code dans la même PR.

## Fini, c'est quand ?

- `mise run check` passe (et `mise run openapi` si l'API a changé, `mise run deny` si une dépendance a changé) ;
- le comportement nouveau est testé, la doc utilisateur à jour ;
- tu as relu ton diff comme un relecteur exigeant (permissions, garde membre, journal, bornage au serveur, limites
  Discord, textes en français).
