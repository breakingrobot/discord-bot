# Déploiement

Le bot est livré sous forme d'image Docker (`Dockerfile` à la racine). Au démarrage, il crée ou met à jour
ses bases, déploie les slash commands, se connecte à Discord et démarre son [site web](site-web.md).
Toutes ses données sont dans **`/app/data`** : un seul volume à monter.

## Checklist de mise en production

1. Application Discord créée, intents activés, bot invité : [Discord](discord.md).
2. Service Northflank avec le volume `/app/data` et le port `8890` public : [ci-dessous](#northflank).
3. Variables renseignées (au minimum `DISCORD_TOKEN`, `DISCORD_GUILD_IDS`, `WEB_URL`,
   `DISCORD_CLIENT_SECRET`) : [variables](#variables-denvironnement).
4. Redirection OAuth2 `<WEB_URL>/auth/callback` ajoutée dans le portail Discord : [site web](site-web.md#2-connexion-discord).
5. Tickets (facultatif) : variables `MM_*` : [Tickets](tickets.md).
6. Après le déploiement, dans les logs du service : `connecté à Discord`, `commandes déployées sur la
   guilde …` et `serveur web démarré`. Puis ouvrir `WEB_URL` et se connecter.
7. Sur Discord, configurer chaque module avec `/config <module>` : [Modules](modules.md).

## Northflank

1. **Create new → Service → Combined service** (build et exécution dans le même service).
2. **Repository** : ce dépôt et la branche à déployer. **Build type** : *Dockerfile*, chemin `/Dockerfile`,
   contexte `/`.
3. **Environment variables** : voir [l'exemple complet](#exemple-complet). Créer un *Secret group* pour
   `DISCORD_TOKEN` et `DISCORD_CLIENT_SECRET` et le lier au service.
4. **Volumes → Add volume** : chemin de montage **`/app/data`**, la plus petite taille suffit (les bases
   pèsent quelques Mo).
5. **Ports & DNS → Add port** : port `8890`, protocole **HTTP**, **Publicly expose** coché. Reporter
   l'adresse `https://….code.run` attribuée dans `WEB_URL` (et `MM_URL`). Domaine personnalisé :
   [site web](site-web.md#1-adresse-publique-en-https).
6. **Health checks** : HTTP, port `8890`, chemin `/healthz`.
7. **Resources** : le plus petit plan suffit. Le bot est réglé pour une petite empreinte mémoire (2 threads,
   petites connexions SQLite, un seul client HTTP, `MALLOC_ARENA_MAX=2` dans l'image) ; le cache Discord
   (membres, rôles, salons) grandit avec la taille des serveurs.

Un volume limite le service à **une instance**, ce qui est voulu : un bot Discord ne doit tourner qu'en un
seul exemplaire. Pour mettre à jour, pousser sur la branche suivie : Northflank reconstruit l'image et
redéploie ; les migrations s'appliquent au démarrage.

## Docker

```bash
docker build -t bot-discord .
docker run -d --name bot-discord --restart unless-stopped \
  --env-file .env -v bot-data:/app/data -p 8890:8890 bot-discord
```

## En local

Prérequis : [Rust](https://rustup.rs) stable.

```bash
cp .env.example .env      # renseigner au moins DISCORD_TOKEN et DISCORD_GUILD_IDS
cargo run --release
```

Les données sont alors dans `./data`, le site sur <http://localhost:8890>. Pour tester la connexion
Discord en local : `WEB_URL=http://localhost:8890` et la redirection `http://localhost:8890/auth/callback`
dans le portail.

## Variables d'environnement

| Variable | Défaut | Description |
|---|---|---|
| `DISCORD_TOKEN` | — | **Requis.** Token du bot ([Discord](discord.md#application-et-token)). |
| `DISCORD_GUILD_IDS` | *(vide)* | Serveurs autorisés, séparés par des virgules. Le bot quitte les autres serveurs et y déploie ses commandes immédiatement. Vide : aucune restriction, commandes globales (jusqu'à une heure de propagation). **À renseigner en production.** |
| `WEB_URL` | `MM_URL` | Adresse publique du site, sans `/` final. |
| `DISCORD_CLIENT_SECRET` | — | Active la [connexion Discord](site-web.md#2-connexion-discord) du site. |
| `PORT` | `8890` | Port du site (l'option `port` du module ticket a priorité). |
| `WEB_UI_DIR` | `/app/ui` (image), sinon `ui/dist` | Dossier de l'interface compilée. Rien à régler avec l'image Docker. |
| `DATABASE_URL` | `file:./data/bot.db` | Base SQLite principale. |
| `DEPLOY_COMMANDS` | `true` | Déployer les slash commands au démarrage. `bot-discord deploy-commands` les déploie puis s'arrête. |
| `RUST_LOG` | `info` | Détail des logs : `error`, `warn`, `info`, `debug`. Par module : `info,bot_discord::modmail=debug`. |
| `TOKIO_WORKER_THREADS` | `2` | Threads de travail. 2 suffisent pour un bot ; plus de threads = plus de mémoire. |
| `MM_*`, `MODMAIL_CONFIG` | — | [Module ticket](tickets.md). |
Les heures saisies dans les commandes sont toujours interprétées en heure de Paris, quel que soit `TZ`.

### Exemple complet

Bot avec site web et tickets (les identifiants s'obtiennent par clic droit → *Copier l'identifiant*, voir
[Discord](discord.md#copier-un-identifiant)) :

```bash
# Secret group
DISCORD_TOKEN=…
DISCORD_CLIENT_SECRET=…

# Bot et site web
DISCORD_GUILD_IDS=111111111111111111
WEB_URL=https://bot.mon-domaine.fr

# Tickets
MM_MAIN_SERVER_ID=111111111111111111
MM_INBOX_SERVER_ID=111111111111111111
MM_LOG_CHANNEL_ID=222222222222222222
MM_NEW_THREAD_CATEGORY_ID=333333333333333333
MM_INBOX_SERVER_PERMISSION=444444444444444444
MM_MENTION_ROLE=444444444444444444
MM_URL=https://bot.mon-domaine.fr
MM_CLOSE_MESSAGE=Ce ticket est fermé. Écris-nous à nouveau si besoin.
```

## Contenu de `/app/data`

| Fichier | Contenu |
|---|---|
| `bot.db` | Base principale (présences, transactions, véhicules, recrues, numéros, prises en charge des tickets) |
| `modmail.sqlite` | Base des tickets, au schéma de modmailbot |
| `logs/`, `attachments/` | Seulement avec `logStorage = attachment` ou `attachmentStorage = local` |
| `config.ini` | Facultatif : configuration des tickets si `MODMAIL_CONFIG=/app/data/config.ini` |

Sauvegarder le volume (snapshots de volume Northflank) suffit à tout sauvegarder.

## Reprendre les données de l'ancien bot (TypeScript)

Copier son fichier `dev.db` dans le volume `/app/data` **avant le premier démarrage**. Le bot le renomme
en `bot.db` et le met à jour sans perte : tables `Prospect*` renommées en `Recrue*` (avis, vétos et salons
d'essai conservés), dates normalisées.

- `/prospect` est remplacée par `/recrue` (option `role_prospect` → `role_recrue`) ;
- les messages déjà publiés (présences, tableaux de bord, avis de la semaine) restent fonctionnels.

Pour reprendre une installation modmailbot : [Tickets](tickets.md#reprendre-une-installation-modmailbot).
