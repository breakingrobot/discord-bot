---
name: module-discord
description: Ajouter ou modifier une slash command, un bouton, un formulaire ou un module entier du bot Discord (définition, aiguillage, permissions, journal, tests, doc).
---

# Commande ou module Discord

Prends exemple sur un module récent et complet : `src/modules/contacts/` (CRUD + annuaire), `src/modules/halamadrid/`
(boutons, formulaires, tâche de fond), `src/modules/m2t.rs` (commande minimale).

## Étapes

1. **Permission** : nouvelle ligne dans `perms!` (`src/modules/permissions/mod.rs`) : clé `module.action`, module
   affiché, libellé, niveau par défaut (≥ `Membre`), stricte ou non. Ajoute-la au type `Perm` de
   `ui/src/api/types.ts`, aux maquettes (`ui/src/mocks/fixtures.ts` : `ALL_PERMS` et la liste des permissions), au
   tableau de `docs/modules.md`, puis `mise run openapi`.
2. **Définition** (`commands.rs`) : `CommandBuilder` / `SubCommandBuilder`, descriptions ≤ 100 caractères, options
   avec `min_value`/`max_value`/`max_length`/`choices`, autocomplétion quand l'utilisateur doit retaper un nom.
3. **Aiguillage** (`src/discord/commands.rs`) : `definitions()`, `route()` (commande, autocomplétion, composant via
   `NAMESPACE`, formulaire via l'action du `custom_id`).
4. **Handler** : `permissions::require(ix, Perm::…)` en premier ; `require_member` avant d'enregistrer quoi que ce soit
   au nom de quelqu'un ; `ix.defer` si c'est long ; réponses éphémères pour les retours personnels ; erreurs qui
   disent quoi faire.
5. **Écriture** : `audit::record` pour une action sensible ; rafraîchis le tableau de bord ou le message concerné ;
   SQL dans `service.rs`, toujours borné par `"guildId"`.
6. **Composants** : `custom_id` via `encode_custom_id(NAMESPACE, action, id, payload)` ; revérifie les droits et
   l'appartenance au serveur au clic (le `custom_id` vient du client).
7. **Tests** : logique pure (mise en forme, parsing, barèmes) ; le test `command_definitions_are_valid` couvre les
   limites Discord ; une base temporaire (`crate::db::open_main`) pour le SQL non trivial.
8. **Doc** : section du module dans `docs/modules.md` (commandes, droits, effets).

Rappel : les commandes doivent être redéployées sur Discord pour apparaître (au démarrage, `DEPLOY_COMMANDS`).
