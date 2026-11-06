# Bot Discord

Bot Discord pour un groupe RP, écrit en Rust ([Twilight](https://twilight.rs)) avec SQLite et livré sous
forme d'image Docker.

| Module | En bref |
|---|---|
| **Présences** | Appels aux événements avec boutons Présent / Absent / Retard, rappels automatiques, statistiques d'assiduité, blocklist. |
| **Absences** | Déclaration d'absences, rôle d'absence posé et retiré automatiquement. |
| **Transactions** | Fiches de transactions avec statuts et objets, tableau de bord permanent. |
| **Véhicules** | Registre des plaques par propriétaire, consultation de l'API GLife. |
| **Recrues** | Suivi des périodes d'essai : avis hebdomadaires des gradés, vétos des leads, salons d'essai privés. |
| **Numéros** | Annuaire téléphonique du serveur. |
| **Coffres** | Stock et historique des coffres de propriété, lus automatiquement dans les messages du jeu. |
| **Cotisations** | Ce que les membres rapportent au groupe (argent propre et sale), statistiques pour les gradés et leads. |
| **Contacts** | Répertoire des groupes, entreprises et contacts du jeu (allié, neutre, hostile), notes réservées au staff. |
| **Tickets** | Modmail compatible [Dragory/modmailbot](https://github.com/Dragory/modmailbot) : tickets par message privé, boutons de prise en charge et de réponse, fils de forum en option. |
| **Site web** | Interface React : statistiques et graphiques, fiches des membres, cotisations, coffres, contacts (saisie comprise), permissions, configuration, logs des tickets. Connexion Discord. |

<img src="docs/images/statistiques.png" alt="Statistiques d'un serveur" width="520"> <img src="docs/images/mobile-log-sombre.png" alt="Log d'un ticket sur mobile" width="180">

## Démarrage rapide

1. **Créer le bot** sur le portail Discord, activer les intents et l'inviter : [docs/discord.md](docs/discord.md).
2. **Déployer** l'image (Northflank ou Docker) avec un volume sur `/app/data` et le port `8890` public :
   [docs/deploiement.md](docs/deploiement.md), qui contient une checklist complète.
3. **Configurer les modules** sur Discord (`/config presence`, `/config recrue`…) et, si besoin, qui peut faire
   quoi (`/config permission`) : [docs/modules.md](docs/modules.md).

Le bot crée ses bases et déploie ses slash commands tout seul au démarrage.

## Documentation

| Page | Contenu |
|---|---|
| [Discord](docs/discord.md) | Application, token, intents, invitation et permissions |
| [Déploiement](docs/deploiement.md) | Checklist, Northflank, Docker, variables d'environnement, reprise des données |
| [Site web](docs/site-web.md) | HTTPS, connexion Discord, pages (tickets, logs, statistiques), sécurité |
| [Modules](docs/modules.md) | Configuration et utilisation de chaque module |
| [Tickets](docs/tickets.md) | Fonctionnement, mise en place, boutons, commandes, reprise de modmailbot |
| [Options des tickets](docs/tickets-options.md) | Toutes les options reconnues, avec leurs valeurs par défaut |
| [Dépannage](docs/depannage.md) | Problèmes fréquents et leurs causes |
| [Développement](docs/developpement.md) | Mise en route avec mise (Windows compris), tâches, qualité et agents de code, structure du code |
