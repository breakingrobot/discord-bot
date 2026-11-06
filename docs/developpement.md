# Développement

## Mise en route (Windows, macOS, Linux) avec mise

[mise](https://mise.jdx.dev) installe les bonnes versions des outils (Rust stable avec Clippy et rustfmt, Node 22,
cargo-deny) et porte les tâches du projet (`mise.toml`), identiques sous PowerShell, cmd et bash.

### Windows

1. **Outils de compilation C++** (le bot embarque SQLite et `ring`, compilés en C) : installe les « Build Tools for
   Visual Studio 2022 » avec la charge de travail « Développement Desktop en C++ » :

   ```powershell
   winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
   ```

2. **Git** (fins de ligne gérées par `.gitattributes`, rien à régler) et **mise** :

   ```powershell
   winget install --id Git.Git
   winget install --id jdx.mise
   ```

3. **Activer mise** dans PowerShell (une fois), puis rouvrir le terminal :

   ```powershell
   Add-Content $PROFILE '(&mise activate pwsh) | Out-String | Invoke-Expression'
   ```

   VS Code et RustRover trouvent les outils si le terminal est activé ; sinon utiliser les raccourcis
   `mise activate --shims` (voir la documentation de mise).

### Ensuite, sur tous les systèmes

```bash
git clone <dépôt> && cd <dépôt>
mise trust          # autorise le mise.toml du dépôt
mise install        # Rust stable, Node 22, cargo-deny
mise run setup      # npm ci de l'interface
cp .env.example .env   # PowerShell : Copy-Item .env.example .env — puis renseigner DISCORD_TOKEN
mise run dev        # bot (cargo run) + interface (Vite) en parallèle
```

| Tâche | Effet |
|---|---|
| `mise run dev` · `dev:bot` · `dev:ui` · `dev:mock` | Bot et interface ; interface seule sur la maquette (sans Discord) |
| `mise run check` | Tout ce que vérifie la CI, Rust et interface en parallèle (`check:rust`, `check:ui`) |
| `mise run fmt` · `lint` · `test` | Formatage, Clippy + Oxlint, tests Rust + Vitest |
| `mise run openapi` | Régénère `ui/openapi.json` après un changement d'API et vérifie le client |
| `mise run deny` | Failles connues, licences et sources des dépendances Rust (`deny.toml`) |
| `mise run build` | Interface compilée puis binaire du bot en release |
| `mise tasks` | Toutes les tâches |

Sans mise, les commandes équivalentes :

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

cd ui && npm ci
npm run format:check && npm run lint && npm run typecheck && npm run check:api && npm test && npm run build
```

La CI (`.github/workflows/ci.yml`) vérifie tout cela (Rust, interface, dépendances avec cargo-deny, compilation sous
Windows) et le build Docker.

## Qualité et agents de code

- **Lints Rust** (`Cargo.toml`, section `[lints]`) : `unsafe` interdit ; `dbg!`, `todo!`, `println!` et `unwrap()`
  interdits hors des tests (`clippy.toml` les autorise dans les tests) ; quelques lints d'idiomes. La CI tourne en
  `-D warnings` : tout avertissement bloque.
- **Règles du dépôt pour les agents** : `AGENTS.md` (lu par Claude Code via `CLAUDE.md`, et par Codex, Cursor…).
- **Harnais Claude Code** (`.claude/`) :
  - hooks qui refusent d'écrire dans les fichiers générés, `.env` et les migrations déjà fusionnées (`guard.mjs`) ;
  - formatage de chaque fichier modifié (`format.mjs`) ;
  - Clippy et tests avant de rendre la main (`verify.mjs`, désactivable par `CLAUDE_SKIP_VERIFY=1`) ;
  - compétences pas à pas (`skills/` : vérifier, module Discord, route d'API, migration, page du site, mener une PR) ;
  - sous-agent `relecteur` qui relit un diff selon les règles du projet.
- Réglages personnels hors dépôt : `mise.local.toml`, `.claude/settings.local.json`.

## Organisation du code

| Dossier | Contenu |
|---|---|
| `src/discord/` | Gateway, interactions, réponses, composants, cache des membres |
| `src/modules/` | Un dossier par module à slash commands ; `permissions/`, `status.rs`, `fiche.rs` |
| `src/modmail/` | Tickets : configuration, base, commandes texte, boutons, données des logs |
| `src/web/` | Serveur web : connexion Discord, service de l'interface (`spa.rs`), rendu du markdown Discord |
| `src/web/api/` | API JSON du site (une route par fichier de domaine) et spécification OpenAPI (`openapi.rs`) |
| `src/util/` | Dates (Europe/Paris), mise en forme, identifiants |
| `migrations/` | Migrations SQL de la base principale, appliquées au démarrage |
| `ui/` | Interface React : Vite, TanStack Router et Query, Tailwind CSS v4, shadcn/ui, TanStack Charts, Jotai, React Compiler, Vitest ; conception atomique (`atoms/` → `templates/`), pages séparées des routes ([README](../ui/README.md)) |

**Nouvelle colonne en base** : ajouter `migrations/000N_description.sql`, ne jamais modifier une migration
déjà déployée.

**Interface en local** : `cd ui && npm run dev:mock` (sans le bot, données fictives) ou `npm run dev` (API relayée
vers le bot sur `http://localhost:8890`). Le bot sert `ui/dist` (variable `WEB_UI_DIR`) : `npm run build` avant de le
lancer pour voir le site compilé.

**Changer l'API** : modifier `src/web/api/`, puis régénérer la spécification et vérifier l'interface :

```bash
UPDATE_OPENAPI=1 cargo test openapi_spec_is_up_to_date   # réécrit ui/openapi.json
cd ui && npm run check:api                               # client Hey API ↔ types de l'interface
```

**Essai dans un navigateur** du vrai serveur web, sans Discord (connexion désactivée, 120 s) :

```bash
cd ui && npm run build && cd ..
UI_SMOKE_PORT=8899 cargo test serve_for_browser -- --ignored --nocapture
```

**Markdown Discord** : `src/web/markdown.rs`, testé par `cargo test markdown`.
