@AGENTS.md

## Spécifique à Claude Code

Le harnais du dépôt (`.claude/`) travaille avec toi :

- **Avant une modification** (`guard.mjs`) : refuse d'écrire dans `.env`, `Cargo.lock`, les fichiers générés
  (`ui/openapi.json`, `ui/src/routeTree.gen.ts`, `ui/src/api/generated/`) et les migrations déjà sur `main`.
  Le message dit quoi faire à la place.
- **Après chaque modification** (`format.mjs`) : le fichier est formaté (rustfmt, Oxfmt). Une erreur de syntaxe
  remonte tout de suite.
- **Avant de rendre la main** (`verify.mjs`) : Clippy et les tests Rust si du Rust a changé, types, lint et tests de
  l'interface si `ui/` a changé. Un échec te renvoie au travail ; corrige la cause, ne contourne pas
  (`CLAUDE_SKIP_VERIFY=1` est réservé à l'humain).

Compétences (`.claude/skills/`) : `verifier`, `module-discord`, `route-api`, `migration`, `page-site`, `steward`.
Charge celle qui correspond avant d'attaquer la tâche. Sous-agent `relecteur` (`.claude/agents/`) : lance-le sur ton
diff avant de conclure un changement non trivial.
