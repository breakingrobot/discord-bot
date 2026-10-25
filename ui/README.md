# Interface web (React)

Application monopage du site du bot : accueil et statistiques des serveurs, transactions, cotisations, coffres, contacts,
journal du staff, fiches des membres, permissions, configuration, santé, tickets et logs. Palette de commandes,
mise à jour en direct (SSE), navigation mobile et application installable (PWA). Le bot (axum) sert les
fichiers construits de `ui/dist` et l'API JSON sous `/api`, sur la même origine.

Le contrat de l'API est `src/api/types.ts` ; la spécification OpenAPI du bot (`ui/openapi.json`, aussi servie sur
`/api/openapi.json`) en est la version exportée.

## Pile

| Outil                                                      | Version                                                        | Rôle                                                            |
| ---------------------------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------- |
| React                                                      | 19.3.0                                                         | Interface                                                       |
| Vite                                                       | 8.3.2                                                          | Serveur de développement, compilation (`dist/`)                 |
| TanStack Router                                            | 1.170.41 (+ `@tanstack/router-plugin` 1.168.42)                | Routes par fichiers (`src/routes`), découpage du code par route |
| TanStack Query                                             | 5.104.1                                                        | Toutes les requêtes à l'API                                     |
| Tailwind CSS                                               | 4.3.3                                                          | Styles, jetons du thème dans `src/styles.css`                   |
| shadcn/ui (Radix, paquet `radix-ui` 1.6.7)                 | —                                                              | Primitives (`src/atoms`), écrites dans le style shadcn          |
| TanStack Charts                                            | 1.0.0 (`@tanstack/react-charts` + `@tanstack/charts`)          | Graphiques SVG, chargés seulement par les pages qui en ont      |
| Jotai                                                      | 3.0.1                                                          | État d'interface (thème, tiroir, dernier serveur)               |
| lucide-react                                               | 1.52.0                                                         | Icônes                                                          |
| sonner                                                     | 2.0.8                                                          | Notifications (toasts)                                          |
| React Compiler (`babel-plugin-react-compiler`)             | 1.0.0 (+ `@rolldown/plugin-babel` 0.2.4, `@babel/core` 7.29.7) | Mémoïsation automatique à la compilation (cible React 19)       |
| TypeScript                                                 | 7.0.2 (compilateur natif)                                      | `npm run typecheck` (`tsc --noEmit`) ; Vite transpile           |
| Oxlint                                                     | 1.86.0                                                         | Lint (`.oxlintrc.json`)                                         |
| Oxfmt                                                      | 0.71.0                                                         | Format (`.oxfmtrc.json`)                                        |
| Hey API (`@hey-api/openapi-ts`)                            | 0.0.0-next-20260930190945 (étiquette `next`)                   | Génération du client typé depuis `openapi.json`                 |
| Vitest (+ `@vitest/coverage-v8`)                           | 5.0.3                                                          | Tests (`npm test`)                                              |
| happy-dom                                                  | 20.14.5                                                        | DOM des tests                                                   |
| Testing Library (`react`, `dom`, `user-event`, `jest-dom`) | 16.3.3, 10.4.2, 14.6.7, 7.0.1                                  | Rendu et interactions dans les tests                            |

Versions exactes, publiées au moins deux semaines avant leur adoption. Hey API est pris sur l'étiquette `next` : la
version stable ne prend pas encore TypeScript 7 en charge. Le client fetch de Hey API est généré dans le dossier de
sortie (pas de paquet `@hey-api/client-fetch` à installer).

**Pourquoi pas TanStack Start (mode SPA) ?** Il compile, mais la page d'amorce qu'il pré-rend contient des scripts en
ligne (données d'hydratation `$_TSR`, restauration du défilement) indispensables au démarrage. La CSP du bot
(`script-src 'self'`) les bloque, et leurs empreintes changent à chaque compilation. Vite + TanStack Router produit
un `index.html` sans script en ligne et ne construit pas de serveur Node inutile.

## Scripts

```sh
npm ci                 # dépendances (package-lock.json versionné)
npm run dev            # développement, API relayée vers le bot (BOT_URL, http://localhost:8890 par défaut)
npm run dev:mock       # développement sans le bot : maquette de l'API
npm run build          # compilation dans dist/
npm run preview:mock   # compilation en maquette (dist-mock/) et aperçu sur http://localhost:4173
npm run typecheck      # tsc --noEmit (TypeScript 7)
npm run lint           # oxlint, avertissements = erreurs
npm run format         # oxfmt (écrit)
npm run format:check   # oxfmt --check
npm run test           # vitest run (une passe, quelques secondes)
npm run test:watch     # vitest en continu
npm run test:coverage  # couverture v8 (coverage/, non versionné)
npm run generate:api   # client Hey API depuis openapi.json → src/api/generated (non versionné)
npm run check:api      # génère, puis vérifie que la spécification convient aux types de src/api/types.ts
npm run check:compiler # le React Compiler compile chaque composant et hook (aucun abandon)
```

`format:check`, `lint`, `typecheck`, `check:api`, `test` et `build` doivent passer avant chaque commit (la CI les
lance tous). Les fichiers générés (`src/routeTree.gen.ts`, `src/api/generated/`) sont exclus du lint et du format ;
`src/routeTree.gen.ts` est versionné (il est réécrit par Vite à chaque lancement).

## TypeScript

`strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `noPropertyAccessFromIndexSignature`,
`noImplicitReturns`, `useUnknownInCatchVariables`, `noImplicitOverride`, `noFallthroughCasesInSwitch`,
`noUnusedLocals`, `noUnusedParameters`, `verbatimModuleSyntax`, `isolatedModules`, `erasableSyntaxOnly` (ni `enum`,
ni `namespace`, ni propriétés de paramètres : des unions de chaînes à la place). Pas de `any`, pas d'assertion non
nulle. Les objets de configuration sont vérifiés par `satisfies` (thème des graphiques, client TanStack Query…), les
variables d'environnement lues sont déclarées dans `src/vite-env.d.ts`.

## React Compiler

`babel-plugin-react-compiler` passe sur tout le code de `src/` (préréglage `reactCompilerPreset` de
`@vitejs/plugin-react` dans `@rolldown/plugin-babel`, cible React 19, `react/compiler-runtime`). Il mémoïse
composants, valeurs calculées et fonctions passées en props : pas de `useMemo`, `useCallback` ni `memo` écrits à la
main. Les définitions des graphiques, par exemple, ne sont recalculées que si leurs données changent.
`npm run check:compiler` échoue si une fonction est abandonnée par le compilateur (règle des hooks enfreinte,
mutation pendant le rendu…) ; dans `dist/`, les morceaux compilés contiennent le cache du runtime (`react.memo_cache_sentinel`).

## Architecture

```
src/
  atoms/        primitives d'interface (shadcn sur Radix) : Button, Input, Card, Dialog, Select, Table…
  molecules/    petites combinaisons : StatCard, Field, MemberAvatar, Pagination, ChartLegend, ThemeToggle…
  organisms/    blocs autonomes : tableaux, graphiques, dialogues, navigation du serveur, en-tête, entrée de log
  templates/    mises en page : RootLayout, GuildLayout (garde : connexion, accès) et GuildShell, PageContainer
  pages/        un dossier par domaine : XxxPage (requêtes, états de chargement), XxxView (rendu), sections
                propres à la page, xxx-search.ts (paramètres d'adresse de la page)
  routes/       déclarations de routes seulement (TanStack Router, routes par fichiers)
  state/        atomes Jotai, un fichier par sujet
  api/          types.ts (contrat), client.ts (fetch, erreurs, 401), queries.ts (options TanStack Query et
                mutations, seul point d'accès à l'API), hey-api.ts, contract-check.ts (spécification ↔ types) ;
                types-site.ts et queries-site.ts : leur suite pour l'accueil, la santé, les suppressions
                annulables et les exports CSV côté client (fichiers limités à 300 lignes) ;
                types-groupe.ts / queries-groupe.ts : suite pour présences, absences, recrues, annuaire et
                écritures des transactions (limite de 300 lignes par fichier)
  lib/          fonctions sans React : format (Intl, « 12 500 $ »), droits par page, paramètres d'adresse,
                constantes partagées (graphiques, contacts, niveaux de permission, tons)
  hooks/        hooks réutilisables : use-debounced, use-live-updates (flux SSE), use-undoable-action
                (« Annuler »)
  mocks/        maquette de l'API (exclue du build)
  test/         préparation des tests (setup.ts, render.tsx)
public/         theme-init.js (thème avant le premier rendu), favicon, manifest.webmanifest, icônes PNG
                (192, 512, maskable, apple-touch-icon), sw.js (service worker), offline.html
```

Une couche n'importe que des couches inférieures : atomes ← molécules ← organismes ← modèles ← pages ← routes.
Un composant n'existe dans une couche partagée que s'il sert ailleurs que sur sa page ; sinon il reste dans
`pages/<domaine>/`.

**Pages et routes.** Un fichier de `src/routes/` ne fait que déclarer la route : chemin, `validateSearch`,
`beforeLoad` (redirections) et `component`. Le composant vient de `src/pages/`, qui lit paramètres et adresse par
`getRouteApi('/serveurs/$guildId/coffres')` (pas d'import du fichier de route, donc pas de cycle). Le découpage
automatique du routeur (`autoCodeSplitting`) place chaque page dans son propre morceau, chargé à la navigation.

**Alias.** `@/atoms`, `@/molecules`, `@/organisms`, `@/templates`, `@/pages` (déclarés dans `tsconfig.json` et
`vite.config.ts`), puis `@/` pour le reste de `src/`. Pas de fichier « barrel » (`index.ts`) : chaque import vise le
fichier du composant, ce qui garde le découpage par page exact.

## Outils transverses

**Palette de commandes** (`organisms/CommandPalette.tsx`, montée dans `RootLayout`) : Ctrl+K / ⌘K ou
`paletteOpenAtom` (`state/palette.ts`). Dialogue Radix, champ `role="combobox"` avec `aria-activedescendant`,
liste `role="listbox"` en groupes. Les entrées sont calculées par `lib/palette.ts` (fonctions pures, testées) selon les
droits du serveur courant (`useParams({ strict: false })`), filtrées sans accents par `lib/text.ts`. Une nouvelle page
apparaît d'elle-même si elle est dans `GUILD_PAGES` ; une nouvelle source se branche dans `allCommands`.

**Mise à jour en direct** : `useLiveUpdates` (monté dans `GuildShell`) ouvre `EventSource` sur
`/api/guilds/:id/events`, reconnecte avec une attente croissante et invalide, pour chaque sujet reçu, les domaines
de `TOPIC_DOMAINS` (`lib/live.ts`). Les clés de requête d'un serveur commencent toutes par
`['guild', guildId, <domaine>, …]` : une nouvelle page range ses requêtes sous son domaine (`presences`,
`absences`, `recrues`, `annuaire`…) et l'ajoute au sujet concerné dans `TOPIC_DOMAINS`. Rien en maquette.

**« Annuler »** : `useUndoableAction` (`hooks/use-undoable-action.ts`) prend `hide` (retire l'élément du cache
TanStack Query, renvoie la remise en état), `commit` (requête réelle, `keepalive` si l'onglet se ferme), `message`
et `onCommitted`. Exemples : `useUndoableDeleteCotisation`, `useUndoableUnlinkPlayer` (`api/queries-site.ts`).

**Exports CSV côté client** : molécule `CsvExportButton` (`filename`, `columns`, `rows`) et util pur `toCsv`
(`lib/csv.ts`) au format des exports du bot : BOM UTF-8, séparateur `;`, CRLF, guillemets RFC 4180, formules
neutralisées (`'=`…), nombres à virgule, dates `csvDateTime` (`JJ/MM/AAAA HH:MM`, heure de Paris) et `csvDay`.
`rows` est soit la liste affichée, soit une fonction asynchrone qui charge toutes les pages (voir
`fetchJournalForExport`). Une liste déjà exportée par le bot garde `CsvButton` (lien `…/export.csv`).

```tsx
const COLUMNS: CsvColumn<Contact>[] = [
  { header: 'Nom', value: (contact) => contact.name },
  { header: 'Ajouté le', value: (contact) => csvDateTime(contact.createdAt) },
]
<CsvExportButton filename="contacts" columns={COLUMNS} rows={query.data?.items ?? []} />
```

**Mobile** : sous `lg`, `GuildTabBar` (quatre raccourcis + « Plus » qui ouvre le tiroir) ; la page garde une marge
basse (`pb-24`). La barre du serveur sous l'en-tête se réduit au nom du serveur, qui ouvre ce même tiroir. Sous `sm`,
un tableau large s'affiche en cartes : `MobileCards` / `MobileCard` (molécule, `flush` dans une carte existante) à
côté du tableau enveloppé dans `hidden sm:block`. Un dialogue devient une feuille ancrée en bas (à portée du pouce) ;
ses boutons prennent toute la largeur.

**Responsive : règles de `styles.css`** (à connaître avant d'écrire une classe) :

- `gutter` : marges latérales de la page (1 → 1,5 → 2 rem), jamais sous l'encoche en paysage (`env(safe-area-inset-*)`).
  À utiliser à la place de `px-4 sm:px-6` sur un conteneur de page.
- `.grid` sans `grid-cols-*` a une colonne `minmax(0, 1fr)` : son contenu ne l'élargit plus au-delà de l'écran. Un
  élément `flex` qui doit tronquer son texte garde `min-w-0`.
- Au doigt (`pointer: coarse`) : boutons, listes déroulantes, champs et groupes de boutons font au moins 44 px ; cases
  et interrupteurs gardent leur taille mais se touchent sur 44 px. Pour un petit lien ou bouton nu, ajouter la classe
  `tap` (zone tactile élargie sans changer la mise en page ; inefficace sur un élément `overflow-hidden`/`truncate`),
  et pour une ligne de navigation `pointer-coarse:min-h-11`.
- `short:` : écran peu haut (téléphone en paysage, `max-height: 32rem`) ; les barres collantes de l'en-tête et du
  serveur y deviennent statiques.
- `ToggleGroup` : colonnes régulières sous `sm`, une ligne qui passe à la ligne dès `sm`. Les animations sont réduites
  quand le système le demande (`prefers-reduced-motion`).

`node scripts/audit-responsive.mjs` (maquette servie sur `localhost:4175`, Playwright installé à part ; voir l'en-tête
du script) parcourt toutes les pages à 320, 360, 390, 768, 1024 et 1440 px et échoue si une page défile
horizontalement ou si un élément sort de l'écran ; il liste aussi les cibles tactiles de moins de 44 px.

**Application installable** : `manifest.webmanifest` (« Bot du groupe »), `theme-color`, icônes, et `sw.js`
enregistré en production seulement (`main.tsx`) : cache des `/assets/` (immuables), page `offline.html` hors ligne,
jamais `/api/`. Changer la constante `VERSION` de `sw.js` vide les anciens caches.

## Conventions

- **Fonctions fléchées partout** : composants (`export const StatCard = ({ … }: StatCardProps) => …`), hooks et
  fonctions utilitaires ; pas de déclaration `function` (règles `func-style: expression` et
  `react/function-component-definition`). Les gardes de type sont aussi des flèches
  (`(value: string): value is Level => …`).
- **Types de retour** : inférés. Ils ne sont écrits que lorsqu'ils font partie du contrat (gardes de type, fonctions
  publiques de `lib/` dont le type documente l'usage, génériques) ; aucune règle ne les impose.
- **Props** : un type `XxxProps` par composant, juste au-dessus de lui ; `type` plutôt qu'`interface` (sauf fusion de
  déclarations : `Register` du routeur, `ImportMetaEnv`).
- **Exports nommés seulement** (`import/no-default-export`) ; seuls les fichiers de configuration d'outils
  (`vite.config.ts`, `vitest.config.ts`, `openapi-ts.config.ts`) exportent par défaut.
- **Imports de types séparés** : `import type { … }` (règles `consistent-type-imports` et
  `consistent-type-specifier-style`), tri par Oxfmt.
- **Fichiers** : un composant par fichier, nommé comme lui en PascalCase (`StatCard.tsx`) ; une famille composée
  shadcn reste ensemble (`Card.tsx` exporte `Card`, `CardHeader`…). Tout le reste en kebab-case
  (`format.ts`, `use-debounced.ts`, `coffres-search.ts`, `guild-nav.ts`). Les noms des fichiers de routes suivent la
  convention de TanStack Router (`serveurs.$guildId.coffres.tsx`).
- **Textes** de l'interface en français ; montants, nombres et dates par `src/lib/format.ts`.

## Lint (Oxlint)

Catégories `correctness`, `suspicious`, `perf`, `pedantic` et `style` en erreur, avertissements refusés
(`--deny-warnings`), extensions `eslint`, `typescript`, `react` (dont les règles des hooks), `jsx-a11y`, `import`,
`unicorn`, `promise`, `oxc`, et `vitest` pour les fichiers de test. Oxlint 1.86 n'a pas de règle propre au React
Compiler : `npm run check:compiler` en tient lieu. Règles écartées ou réglées :

| Règle                                                                                                   | Choix                               | Raison                                                                                                     |
| ------------------------------------------------------------------------------------------------------- | ----------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `react/react-in-jsx-scope`                                                                              | désactivée                          | transformation JSX automatique                                                                             |
| `import/no-named-export`, `import/prefer-default-export`, `import/group-exports`, `import/exports-last` | désactivées                         | exports nommés imposés, déclarés à côté de leur code                                                       |
| `eslint/sort-imports`, `unicorn/number-literal-case`                                                    | désactivées                         | Oxfmt trie les imports et écrit les littéraux ; les deux outils se contrediraient                          |
| `eslint/sort-keys`                                                                                      | désactivée                          | l'ordre des clés suit le sens (formulaires, colonnes, réglages)                                            |
| `eslint/no-magic-numbers`                                                                               | désactivée                          | tailles, seuils et marges d'interface : une constante par nombre nuirait à la lecture                      |
| `eslint/no-ternary`, `eslint/no-nested-ternary`, `unicorn/no-nested-ternary`                            | désactivées                         | chaînes « chargement / erreur / données » en JSX, mises en forme par Oxfmt                                 |
| `eslint/one-var`, `eslint/init-declarations`, `eslint/id-length`, `eslint/capitalized-comments`         | désactivées                         | une déclaration par ligne ; `a`/`b` de tri et `x`/`y` de coordonnées ; commentaires en français libre      |
| `eslint/curly`                                                                                          | `multi-line`                        | `if (…) return` sur une ligne admis, accolades dès que le corps s'étend                                    |
| `eslint/max-lines`                                                                                      | 300 lignes                          | sauf `src/mocks/` (données de la maquette) et tests                                                        |
| `eslint/max-lines-per-function`, `eslint/max-statements`, `import/max-dependencies`                     | désactivées                         | un composant de page est une seule fonction JSX ; découpés par sens, pas par longueur                      |
| `eslint/no-continue`                                                                                    | désactivée                          | boucles de filtrage plus lisibles avec `continue`                                                          |
| `eslint/no-duplicate-imports`                                                                           | types séparés admis                 | cohérent avec `import type` séparé                                                                         |
| `react/jsx-props-no-spreading`                                                                          | désactivée                          | primitives shadcn et `Field` transmettent les props par conception                                         |
| `react/jsx-max-depth`, `react/no-unescaped-entities`, `react/only-export-components`                    | désactivées                         | balisage de mise en page ; apostrophes françaises ; variantes `cva` exportées avec leur composant          |
| greffon `react-perf`                                                                                    | non chargé                          | le React Compiler mémoïse les objets et fonctions passés en props                                          |
| `unicorn/no-null`                                                                                       | désactivée                          | le contrat de l'API et React utilisent `null`                                                              |
| `unicorn/filename-case`                                                                                 | PascalCase ou kebab-case            | composants / reste ; désactivée dans `src/routes/` (noms imposés par le routeur)                           |
| `jsx-a11y/no-autofocus`, `jsx-a11y/prefer-tag-over-role`                                                | désactivées                         | focus initial voulu dans les dialogues ; listes de suggestions ARIA (`listbox`)                            |
| `import/no-namespace`                                                                                   | désactivée pour `contract-check.ts` | comparaison de deux espaces de types complets                                                              |
| `vitest/*` dans les tests                                                                               | quelques-unes désactivées           | titres en français avec noms de composants, `findBy…` de Testing Library, préparation commune (`setup.ts`) |

Les rares désactivations ponctuelles (`oxlint-disable-next-line`) portent leur raison en commentaire.

## Tests

Vitest 5 avec happy-dom et Testing Library, sur la configuration Vite de l'application (routeur, React Compiler,
alias) : on teste le code tel qu'il est livré. Fichiers `*.test.ts(x)` à côté du code testé.

- `lib/` : formats français (« 12 500 $ », dates du jour de Paris, durées relatives), paramètres d'adresse ;
- `api/client.ts` : en-têtes des écritures, message du serveur dans `ApiRequestError`, messages de repli, 401 suivi
  une seule fois vers `loginUrl` en revenant à la page affichée ;
- `state/theme.ts` : valeur brute partagée avec `theme-init.js`, valeur inconnue ignorée ;
- `lib/coffre-inventory.ts` : liste « objet:quantité » de la commande Discord, identifiants d'objets, écarts
  prévisualisés de l'inventaire ;
- molécules et organismes : `StatCard`, validation et envoi du formulaire de contact, niveaux de permission (seules
  les lignes modifiées partent, retour au défaut = `null`), inventaire des coffres (seules les lignes qui changent
  partent, liste collée, erreur 422 affichée dans le dialogue) ;
- `lib/absence-timeline.ts` (frise des absences : fenêtre de cinq semaines, barres découpées, navigation au clavier),
  `lib/presence.ts`, `lib/recrue.ts` ;
- pages Présences, Absences, Recrues, Annuaire (`src/test/app.tsx` : application complète sur la maquette) et
  dialogue « Nouvelle transaction » ;
- une page complète (`JournalPage.test.tsx`) : routeur en mémoire, maquette de l'API (`vi.stubEnv('VITE_MOCK',
'1')`), filtre écrit dans l'adresse ;
- `lib/text.ts`, `lib/csv.ts`, `lib/live.ts`, `lib/palette.ts` : recherche sans accents, échappement CSV, sujet du
  direct → clés à recharger, entrées de la palette selon les droits ;
- `hooks/use-undoable-action.ts` (minuteurs simulés : expiration, « Annuler », `pagehide`, refus du serveur),
  `CsvExportButton`, et l'accueil avec la palette (`CommandPalette.test.tsx` : Ctrl+K, flèches, Entrée, Échap).

`src/test/render.tsx` fournit un client TanStack Query neuf et le contexte d'un serveur ; `src/test/setup.ts` complète
happy-dom pour Radix (capture du pointeur, défilement).

## Mode maquette

`VITE_MOCK=1` remplace l'API par un faux serveur en mémoire (`src/mocks/`) : un groupe RP fictif (membres,
transactions, cotisations, coffres, contacts, journal, tickets, logs). Toutes les routes du contrat répondent, y
compris les écritures (ajout de cotisation, contacts, notes, permissions, objectif, alertes…), gardées en mémoire
jusqu'au rechargement. La condition est remplacée à la compilation : en production, le code de la maquette n'est
pas dans `dist/`.

Scénarios de connexion par paramètre d'adresse (mémorisé pour l'onglet) : `?mock=admin` (par défaut), `?mock=membre`
(simple membre), `?mock=aucun` (connecté sans accès), `?mock=anon` (pas connecté). Les exports CSV sont des liens
directs vers le bot : sans effet en maquette.

## État : qui garde quoi

| Nature de l'état               | Outil                                                     | Exemples                                                                                                                                             |
| ------------------------------ | --------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Données du serveur             | TanStack Query (`src/api/queries.ts`)                     | compte, statistiques, listes, mutations                                                                                                              |
| État partageable par l'adresse | TanStack Router (paramètres validés par `validateSearch`) | filtres, page, période, objet choisi, vue du log (« Échanges seulement », « Détails techniques »)                                                    |
| État d'interface du navigateur | Jotai (`src/state/`)                                      | thème (clé `theme`, valeur brute lue aussi par `public/theme-init.js`), tiroir du serveur sur mobile, dernier serveur ouvert (clé `dernier-serveur`) |
| État local d'un composant      | `useState`                                                | champs d'un formulaire, dialogue ouvert, ligne dépliée                                                                                               |

Jotai utilise le store par défaut (pas de `<Provider>`). Les atomes mémorisés passent par `src/state/storage.ts` :
texte brut, lectures et écritures dans un `try`/`catch` (navigation privée, stockage bloqué), valeurs relues
validées, synchronisation entre onglets. Ne jamais copier une réponse de l'API dans un atome, ni un paramètre
d'adresse. Le scénario de la maquette (`?mock=…`) reste dans `src/mocks/` (sessionStorage) : il est lu par le faux
serveur hors de React et n'existe pas en production.

## Graphiques

Un organisme par graphique (`AttendanceChart`, `MonthlyTransactionsChart`, `WeeklyCotisationsChart`,
`CoffreSeriesChart`), réglages communs dans `src/lib/charts.ts`, légende, infobulle et tableau des valeurs en
molécules. Seules les pages qui en affichent les importent : TanStack Charts reste hors du point d'entrée, dans un
morceau chargé à la demande. Définitions `defineChart` (mémoïsées par le React Compiler), échelles compactes de
`@tanstack/charts` (point, bande, linéaire), couleurs en références aux jetons (`var(--series-1)`…, `var(--line)`) :
le changement de thème ne reconstruit rien. Infobulle de la bibliothèque (souris et clavier) avec un contenu React en
français (`renderTooltipBody`), enveloppe stylée par `.chart` dans `src/styles.css`, légende HTML dès deux séries et
tableau des valeurs (« Afficher les données »). La courbe du stock en escalier utilise une courbe maison
(`ChartCurve`) plutôt que `d3-shape` ; la courbe de présence est tracée en segments droits.

Les pages n'appellent jamais `fetch` : elles importent les options de `src/api/queries.ts`. Passer aux types et
au client générés par Hey API ne touche que `src/api/`.

## Routes

| Adresse                                                        | Page                                                                                        |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `/`                                                            | Connexion, ou tuiles des serveurs et des tickets                                            |
| `/serveurs/:id`                                                | Accueil : tuiles selon les droits                                                           |
| `/serveurs/:id/stats`                                          | Statistiques (`site.stats`)                                                                 |
| `/serveurs/:id/transactions`                                   | Transactions, filtres dans l'adresse, export CSV (`transaction.historique`)                 |
| `/serveurs/:id/cotisations`                                    | Cotisations, objectif, ajout (`cotisation.stats`, `cotisation.ajouter`, `cotisation.gerer`) |
| `/serveurs/:id/coffres`                                        | Stock, évolution d'un objet, joueurs, mouvements (`coffre.voir`, `coffre.gerer`)            |
| `/serveurs/:id/contacts`                                       | Répertoire, fiche et notes, création / modification (`contact.*`)                           |
| `/serveurs/:id/presences`                                      | Présences, assiduité, plannings (`presence.stats`, `presence.creer`, `presence.gerer`)      |
| `/serveurs/:id/absences`                                       | Frise et liste des absences, déclaration (membres du groupe, `absence.autrui`)              |
| `/serveurs/:id/recrues`                                        | Avis, vétos, historique des recrues (`recrue.avis`, `recrue.veto`, `recrue.suivi`)          |
| `/serveurs/:id/annuaire`                                       | Véhicules et numéros (`vehicule.utiliser`, membres du groupe)                               |
| `/serveurs/:id/journal`                                        | Journal du staff (`site.journal`)                                                           |
| `/serveurs/:id/membres`, `/serveurs/:id/membres/:user`         | Recherche et fiche d'un membre (`membre.fiche` ; sa propre fiche pour tous)                 |
| `/serveurs/:id/permissions`, `/serveurs/:id/configuration`     | Réglages (« Gérer le serveur »)                                                             |
| `/serveurs/:id/sante`                                          | Santé du bot, webhooks des coffres, dernières erreurs (« Gérer le serveur »)                |
| `/serveurs/:id/presences`, `…/absences`, `…/recrues`           | Pages provisoires « bientôt » (`pages/soon/`), remplacées par les vraies pages              |
| `/tickets`, `/tickets/staff`, `/logs/:thread`                  | Tickets, statistiques du staff, log d'un ticket                                             |
| `/membres/:user?serveur=:id`                                   | Redirection vers la fiche (liens envoyés par le bot)                                        |
| `/stats…`, `/cotisations`, `/contacts`, `/logs`, `/logs/staff` | Redirections des anciennes adresses                                                         |

## Service par le bot

- Fichiers de `dist/` servis tels quels (`/assets/*` avec un cache long : noms avec empreinte ; `index.html`,
  `theme-init.js`, `favicon.svg`, `sw.js`, `manifest.webmanifest` sans cache long).
- Toute adresse inconnue hors `/api`, `/login`, `/logout`, `/auth/*`, `/healthz` et `/logs/:id?raw=1` renvoie
  `index.html` (routage côté client).
- CSP du bot (`src/web/mod.rs`) : `script-src 'self'` (aucun script en ligne dans `index.html`),
  `manifest-src 'self'`, `worker-src 'self'` (application installable), `style-src 'self' 'unsafe-inline'` (Radix et sonner injectent des `<style>`, par exemple le blocage du défilement
  des dialogues), `connect-src 'self'` pour l'API, `font-src 'self'` pour Inter auto-hébergée, `img-src` limité au
  site et aux CDN de Discord (avatars, pièces jointes).
- Écritures : en-têtes `Content-Type: application/json` et `X-Requested-With: fetch`. Sur 401, l'interface suit
  `loginUrl` en remplaçant `next` par l'adresse affichée.
