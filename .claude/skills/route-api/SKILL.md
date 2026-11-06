---
name: route-api
description: Ajouter ou modifier une route de l'API JSON du site (src/web/api) avec sa spécification OpenAPI, ses types et requêtes côté interface, la maquette et les tests.
---

# Route de l'API

Modèle : `src/web/api/contacts.rs` (lecture, création, modification, suppression, notes).

1. **Handler** dans le fichier du domaine : `session.require(&guild, Perm::…)` (ou `member_of`, `require_admin`) ;
   paramètres via `ApiQuery`, corps via `JsonBody` ; validation → `ApiError::invalid("message en français")` ;
   réponses `json`, `created`, `no_content` ; membres via `Members::new(&ctx, guild_id)` (jamais d'appel HTTP par
   ligne) ; dates en ISO via `iso_db`.
2. **Écriture** : `require_member` si l'action enregistre au nom de quelqu'un, `audit::record_for`, rafraîchissement
   Discord (mêmes fonctions que la commande équivalente) : le site et Discord doivent rester identiques.
3. **Spécification** : `#[utoipa::path(...)]` (tag, `operation_id` en camelCase, réponses 401/403/422), schémas
   `#[derive(ToSchema)]` avec `#[serde(rename_all = "camelCase")]`, ajout dans `openapi.rs` (`paths(...)`), route dans
   `router()` de `src/web/api/mod.rs`. Puis `mise run openapi`.
4. **Interface** : type dans `ui/src/api/types*.ts`, entrée `Fits<…>` dans `contract-check.ts`, requête ou mutation
   dans `ui/src/api/queries*.ts` (clé `['guild', guildId, <domaine>, …]`, invalidation au succès), route dans la
   maquette (`ui/src/mocks/`).
5. **Tests** : contrôle d'accès dans `src/web/api/tests.rs` si le domaine en a ; validation pure testée ; côté
   interface, un test de composant pour l'écran qui l'utilise.
6. **Doc** : `docs/site-web.md` (page, droits).

Ne modifie jamais `ui/openapi.json` ni `ui/src/api/generated/` à la main.
