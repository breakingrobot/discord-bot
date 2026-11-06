---
name: migration
description: Changer le schéma de la base SQLite (nouvelle table, colonne, index) avec une migration sqlx sûre pour la production.
---

# Migration de base

1. Numéro suivant : `ls migrations/` → `NNNN_sujet.sql` (4 chiffres, sujet en snake_case français).
2. **Jamais** de modification d'une migration déjà sur `main` : elle a pu tourner en production (le hook la refuse).
   Une erreur dans une migration fusionnée se corrige par une nouvelle migration.
3. Style de l'existant : noms entre guillemets en camelCase (`"guildId"`), `CREATE TABLE IF NOT EXISTS`,
   `CREATE INDEX IF NOT EXISTS`, commentaire en tête qui dit à quoi sert le changement, un commentaire par colonne
   non évidente.
4. SQLite : `ALTER TABLE … ADD COLUMN` seulement (pas de `DROP COLUMN` sur les données anciennes, pas de changement
   de type) ; une colonne `NOT NULL` ajoutée a un `DEFAULT`. Clé étrangère : `REFERENCES … ON DELETE CASCADE` (les
   clés étrangères sont activées).
5. Mets à jour les structures `FromRow` et la liste des colonnes lues (`*_COLUMNS`) ; toute requête reste bornée par
   `"guildId"`.
6. Vérifie qu'une base neuve démarre : `cargo test` (les tests qui ouvrent `crate::db::open_main` appliquent toutes
   les migrations).
