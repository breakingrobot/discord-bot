# Configurer et utiliser les modules

## Les commandes

| Commande | Pour qui | Contenu |
|---|---|---|
| `/config <module>` | « Gérer le serveur » | Réglages de chaque module, **une fois par serveur**. Sans option, affiche la configuration actuelle. `/config etat` fait le point sur tous les modules ([détails](#état-de-la-configuration)). |
| `/gestion …` | Selon l'action (gradés par défaut) | Actions de staff : votes forcés, annulations, planning des présences, blocklist, panneau des absences… |
| `/presence`, `/absence`, `/transaction`, `/vehicle`, `/recrue`, `/numero`, `/cotisation`, `/contact`, `/coffre` | Selon l'action, voir [Permissions](#permissions) | Utilisation courante. |
| `/fiche [membre]` | Chacun la sienne ; `membre.fiche` pour les autres | [Fiche du membre](#fiche-du-membre). |
| `/rappels` | Membres du groupe | Ses [rappels en message privé](#rappels-en-message-privé). |
| `/m2t` | Gradés et leads (`m2t.utiliser`) | Publie « @… t’es co? » dans le salon où la commande est lancée ; seule la personne visée est notifiée. Le niveau se règle avec `/config permission action:m2t.utiliser`. |
| `/halamadrid`, `/golazo`, `/putabarca` | Membres ; gradés pour ouvrir un match, annoncer un but, saisir le résultat | [Soirée Clásico](#soirée-clásico) : cris, pronostics, paris fictifs, homme du match, classement, chambrage. |
| Clic droit → *Applications* | Voir [plus bas](#actions-au-clic-droit) | Raccourcis sur un membre ou un message. |

`/config` n'apparaît qu'aux membres ayant « Gérer le serveur ». Les autres commandes sont visibles de tous
et vérifient le niveau de chaque action à l'exécution ([Permissions](#permissions)).

Les heures saisies sont en **heure de Paris** ; les dates affichées s'adaptent au fuseau de chacun.

**Autocomplétion** : Discord propose les valeurs existantes dès qu'on tape — présences, transactions, groupes
(répertoire des contacts puis groupes déjà utilisés), objets, propriétés et personnages des coffres (ceux pas
encore reliés d'abord), webhooks, contacts, plaques, cotisations, modèles, actions des permissions — ainsi que les
**dates** des deux semaines à venir (présences, absences) ou passées (cotisations) : `vendredi`, `demain` ou le
début d'une date suffisent. Une saisie libre reste toujours possible. Les propositions respectent les permissions.

**Ouvrir sur le site** : si le [site](site-web.md) est configuré, un bouton 🌐 sous les réponses et tableaux de
bord ouvre la page correspondante, déjà filtrée quand c'est possible (propriété des coffres, contact, membre,
groupe d'une transaction).

**En cas de refus**, le message donne le niveau requis et le tien, et la commande pour s'en sortir : rôle membre
à régler, ou, pour « Gérer le serveur », `/config permission action:… niveau:…` avec un bouton vers la page
Permissions du site. Une erreur inattendue affiche une courte **référence** (ex. `K7M2QD`), écrite aussi dans le
journal du bot : à donner à un admin pour retrouver l'erreur exacte.

## Permissions

Le **rôle membre** est le passage obligé : sans lui (ou sans rôle gradé ou lead), une personne ne peut utiliser
aucune commande du bot, et **rien n'est enregistré à son nom** : cotisation, absence, réponse à une présence,
numéro, véhicule, personnage des coffres. Seul « Gérer le serveur » passe outre, pour administrer. Les recrues
sont suivies par leur propre module (avis, vétos), sans avoir besoin du rôle membre.

```
/config permission role_membre:@Membres     ← à régler en premier
```

Ensuite, chaque action a un **niveau** : membres du groupe, gradés, leads ou « Gérer le serveur ». Les niveaux
s'emboîtent : un lead peut tout ce que peut un gradé, qui peut tout ce que peut un membre.

| Niveau | Qui |
|---|---|
| Membres du groupe | Rôle membre de `/config permission` (ou rôle gradé ou lead). |
| Gradés | Rôle gradé de `/config recrue`, ou un membre qui a « Gérer les messages » sur le serveur (accordée dans un seul salon, elle ne compte pas). |
| Leads | Rôle lead de `/config recrue`. |
| « Gérer le serveur » | Permission Discord. Elle ouvre **toutes** les actions, sauf les actions *strictes*. |

Les actions **strictes** (suivi, avis et vétos des recrues) ne regardent que les rôles : ni « Gérer le
serveur » ni « Gérer les messages » n'y donnent accès.

```
/config permission                                         ← affiche tous les niveaux (✏️ = modifié)
/config permission action:contact.ajouter niveau:Gradés
/config permission action:coffre.voir niveau:Leads
/config permission action:coffre.voir niveau:Par défaut    ← revient au niveau par défaut
```

L'action se choisit dans la liste proposée par Discord. Changer un niveau demande « Gérer le serveur ».
Les pages du site suivent les mêmes réglages (`site.stats`, `cotisation.stats`, `coffre.voir`…).
Présences, absences, recrues, véhicules et numéros se gèrent aussi depuis le site, avec les mêmes actions et les
mêmes effets que les commandes ([pages du site](site-web.md#présences)).

| Action | Module | Ce qu'elle permet | Par défaut |
|---|---|---|---|
| `presence.creer` | Présences | Créer, relancer et clôturer une présence | Membres du groupe |
| `presence.stats` | Présences | Statistiques et historique d'une présence | Membres du groupe |
| `presence.gerer` | Présences | Forcer une réponse, annuler une présence, planning | Gradés |
| `presence.reset` | Présences | Réinitialiser les statistiques de présence | « Gérer le serveur » |
| `presence.blocklist` | Présences | Ajouter ou retirer un membre de la blocklist | Gradés |
| `absence.autrui` | Absences | Absence d'un autre membre, panneau des absences | Gradés |
| `transaction.utiliser` | Transactions | Créer, compléter, changer le statut, annuler | Membres du groupe |
| `transaction.supprimer` | Transactions | Supprimer définitivement une transaction | Gradés |
| `transaction.historique` | Transactions | Historique et export sur le site | « Gérer le serveur » |
| `transaction.modeles` | Transactions | Créer, lister et supprimer les modèles de transaction | Gradés |
| `vehicule.utiliser` | Véhicules | Enregistrer, consulter, modifier, retirer un véhicule | Membres du groupe |
| `numero.ajouter_autrui` | Numéros | Enregistrer le numéro d'un autre membre | « Gérer le serveur » |
| `numero.supprimer_autrui` | Numéros | Supprimer le numéro d'un autre membre | Gradés |
| `recrue.suivi` | Recrues | Liste, détail et historique des avis, ajout et retrait, salon d'essai *(stricte)* | Gradés |
| `recrue.avis` | Recrues | Donner un avis sur une recrue *(stricte)* | Gradés |
| `recrue.veto` | Recrues | Poser ou lever un véto *(stricte)* | Leads |
| `recrue.reset` | Recrues | Forcer une nouvelle semaine d'avis | « Gérer le serveur » |
| `cotisation.ajouter` | Cotisations | Déclarer ses cotisations | Membres du groupe |
| `cotisation.gerer` | Cotisations | Cotisations des autres : déclarer, consulter, supprimer | Gradés |
| `cotisation.stats` | Cotisations | Statistiques (Discord et site) | Gradés |
| `coffre.voir` | Coffres | Stock et historique (Discord et site) | Gradés |
| `coffre.gerer` | Coffres | Relier un personnage, nommer une propriété | Gradés |
| `contact.voir` | Contacts | Consulter le répertoire | Membres du groupe |
| `contact.ajouter` | Contacts | Ajouter un contact | Membres du groupe |
| `contact.modifier` | Contacts | Modifier ou supprimer un contact | Gradés |
| `contact.notes` | Contacts | Lire et écrire les notes du staff (Discord et site) | Gradés |
| `membre.fiche` | Membres | Fiche d'un membre (présences, cotisations, coffres, recrue…) | Gradés |
| `site.stats` | Site | Statistiques du serveur sur le site | « Gérer le serveur » |
| `site.journal` | Site | Journal du staff sur le site | « Gérer le serveur » |
| `m2t.utiliser` | Divers | Relancer la personne visée avec `/m2t` | Gradés |
| `clasico.jouer` | Clásico | `/halamadrid` (cri, pronostics, paris, classement) et `/putabarca` | Membres du groupe |
| `clasico.gerer` | Clásico | Ouvrir un match, `/golazo`, résultat, homme du match, réponses automatiques, mode automatique | Gradés |

Déclarer ses propres absences, répondre aux présences, enregistrer ou supprimer son propre numéro et supprimer
ses propres cotisations demandent seulement d'être membre du groupe.

## Présences

```
/config presence role:@Membres absence_role:@Absent channel:#présences reminder_channel:#rappels absence_channel:#absences rapport_salon:#staff-assiduité seuil_alerte:50
```

| Option | Rôle |
|---|---|
| `role` | **Requis.** Membres concernés par les présences (pingés, comptés dans l'effectif). |
| `absence_role` | Rôle donné pendant une absence déclarée ; ses porteurs sont absents d'office et jamais pingés. |
| `channel` | Salon où sont publiées les présences (défaut : salon de la commande). |
| `reminder_channel` | Salon des rappels automatiques (défaut : salon de la présence). |
| `absence_channel` | Salon où sont annoncées les absences déclarées. |
| `rapport_salon` | Salon du [rapport hebdomadaire](#rapport-hebdomadaire) d'assiduité (non défini : pas de rapport). |
| `seuil_alerte` | Taux de présence (1–100 %, **50** par défaut) sous lequel un membre est signalé dans le rapport. |
| `rapport_desactiver` | `true` arrête le rapport hebdomadaire. |

Utilisation :

- `/presence create heure:21:00 [titre] [date:JJ/MM/AAAA] [description]` : la date vaut aujourd'hui par
  défaut, ou demain si l'heure est déjà passée. Le rôle est pingé, sauf les absents et la blocklist.
- Boutons **Présent / Absent / Retard** (durée demandée) **/ Ping**. L'embed liste les présents, absents,
  retards (🔄 = a changé d'avis) et les membres **sans réponse**.
- Automatique, toutes les 5 minutes : clôture à l'heure de l'événement ; rappel des non-répondants au
  plus toutes les 3 h ; pose et retrait du rôle d'absence.
- `/presence ping`, `close`, `changes` (historique des changements d'avis), `stats [joueur]` (fiche ou
  classement).
- Staff : `/gestion presence forcevote` (saisir la réponse d'un membre, y compris sur une présence
  close 🔒), `annuler` (supprime une présence), `reset` (efface toutes les statistiques, « Gérer le
  serveur »).
- **Blocklist** : `/gestion blocklist ajouter membre raison` : jamais pingé, absent d'office. `retirer`, `liste`.

### Rapport hebdomadaire

Si `rapport_salon` est défini, le bot y publie **chaque lundi à 09:00** (heure de Paris) le bilan de la
semaine écoulée (lundi → dimanche), une seule fois par semaine ; s'il était hors ligne à 09:00, il
l'envoie dès son retour, le lundi même. Le rapport ne pingue personne. Il contient :

- les présences clôturées de la semaine (présents, absents, retards, sans réponse) ;
- le **taux de présence** de la semaine (présents ÷ attendus ; un retard ne compte pas comme présent,
  comme dans `/presence stats`) et son évolution en points par rapport à la semaine précédente ;
- le **top 3** des membres les plus présents ;
- **⚠️ Sous le seuil** : les membres du rôle (hors bots et blocklist) dont le taux sur les **4 dernières
  semaines** est inférieur à `seuil_alerte`. Ceux dont les absences déclarées couvrent plus de la moitié
  des présences de la période restent listés, marqués *(absence déclarée)*.

Les attendus d'une présence sont les membres actuels du rôle déjà arrivés sur le serveur à sa date, plus
ceux qui y ont répondu. Une semaine sans présence clôturée est signalée telle quelle ; s'il n'y en a eu
aucune depuis 4 semaines, le rapport n'est pas envoyé.

### Présences récurrentes

```
/gestion presence planifier jour:mercredi heure:21:00 [titre] [description] [publication:24]
```

La présence est publiée automatiquement chaque semaine, `publication` heures avant l'événement (24 par
défaut), dans le salon des présences, avec le ping habituel. `/gestion presence planning` liste les
plannings, `deplanifier` en supprime un (les présences déjà publiées restent).

## Absences

```
/absence declare motif:"Vacances" fin:20/08/2026 [debut:JJ/MM/AAAA] [membre:@…]
```

Une date couvre la journée entière. Le rôle d'absence est posé dès le début de la période et retiré à la
fin (ou à la fin de la dernière absence si plusieurs se chevauchent). Les présences ouvertes de la
période passent en **Absent**. Déclarer ou clore l'absence d'un autre membre : `absence.autrui` (gradés par défaut).

- `/absence end [membre]` termine l'absence en cours, ou **annule** la prochaine si elle n'a pas commencé.
- `/absence list` liste les absences en cours.
- L'annonce publiée dans le salon des absences est **supprimée automatiquement** quand l'absence se
  termine ou est annulée : le salon ne montre que les absences d'actualité.
- **Panneau** : `/gestion absence panneau [salon]` publie un message avec les boutons **Déclarer une
  absence** (formulaire : motif, dernier jour, premier jour) et **Terminer / annuler mon absence**. Plus
  simple que la commande pour la plupart des membres.

> Si le rôle ne peut pas être posé (permission « Gérer les rôles » absente, ou rôle placé au-dessus de
> celui du bot), l'absence est quand même enregistrée et le problème est signalé.

## Transactions

```
/config transaction dashboard_channel:#transactions ping_role:@Logistique
```

- **Tableau de bord** permanent, mis à jour automatiquement : transactions en cours par statut, total
  engagé, dernières terminées. `clear_ping_role:true` retire le rôle mentionné à la création.
- `/transaction nouvelle [modele]` ouvre un **formulaire** : groupe, achat ou vente (menu), prix total, objets
  (un par ligne) et note. Laissé vide, le prix total est **calculé depuis les objets** s'ils ont tous un prix
  unitaire. Avec un modèle, le formulaire arrive pré-rempli.
- `/transaction create groupe type:Vente prix:1500 [note]` fait la même chose en une commande (le groupe est
  proposé depuis le répertoire des contacts et les groupes déjà utilisés).
- La fiche est publiée dans le salon courant, avec les boutons de statut, **Ajouter un objet**, **Retirer un
  objet** (menu des objets), **Supprimer** et 🌐 vers l'historique du site. Elle affiche la **note** et, si le
  groupe figure au répertoire des contacts, sa **relation** (🟢 allié, ⚪ neutre, 🔴 hostile).
- `add-item`, `status`, `view`, `cancel` sont ouverts à tous ; la suppression définitive se fait avec
  `/gestion transaction supprimer` ou le bouton (`transaction.supprimer`), inscrite au journal du staff dans
  les deux cas. Une transaction **clôturée** n'est plus modifiable.

**Format des objets** (formulaire et modèles) : séparés par des virgules ou des retours à la ligne, quantité
devant (`2x Pistolet`) ou derrière (`Kevlar x5`), 1 par défaut, prix unitaire après `@` (`@ 15000`,
`@ 1500.50` : point pour les décimales).

```
2x Pistolet @ 15000, Kevlar x5, Munitions
```

**Modèles** (`transaction.modeles`, gradés par défaut), 25 par serveur :

```
/transaction modele creer nom:"Armes Ballas" type:Achat [groupe:Ballas] [prix:…] [objets:"2x Pistolet @ 15000, Kevlar x5"]
/transaction modele liste
/transaction modele supprimer modele:…
```
- **Historique** sur le site web (`/stats/<serveur>/transactions`) : filtres, totaux par mois, export
  CSV ([détails](site-web.md#transactions)).
  Création, statut, objets et annulation aussi depuis le site (`transaction.utiliser`), fiche publiée dans le salon
  du tableau de bord.

## Véhicules

```
/config vehicule dashboard_channel:#véhicules
/vehicle add plaque:D6CS3P0G proprietaire:@Membre-ou-@Rôle [modele]
```

Tableau de bord groupé par propriétaire. Le modèle est récupéré via l'API GLife si non précisé (un échec
de l'API n'empêche pas l'ajout). `/vehicle lookup plaque` interroge l'API pour n'importe quelle plaque
(modèle, propriétaire **en jeu**, acheteur, illégal). `owner` transfère, `remove` retire. Les plaques sont
normalisées : `d6cs 3p0g` = `D6CS3P0G`.

## Recrues

```
/config recrue role_recrue:@Recrue role_grade:@Gradé role_lead:@Lead categorie:#Essais salon:#avis-recrues
```

| Option | Rôle |
|---|---|
| `role_recrue` | **Requis.** Rôle des membres en période d'essai : son attribution ajoute le membre au suivi. |
| `role_grade` | **Requis.** Rôle autorisé à donner son avis. |
| `role_lead` | Rôle autorisé à poser un véto (les leads votent aussi). |
| `categorie` | Catégorie où créer les salons d'essai privés. |
| `salon` | Salon où est publié le message d'avis de la semaine. |

- **Chaque semaine** (lundi 00:00), un nouveau message liste les recrues avec le décompte ✅ / ❌ / ⬜.
  Trois menus : donner son avis (vote **et motif obligatoires**, un avis par gradé, modifiable), poser
  un véto, voir le détail. Les semaines passées restent visibles et consultables via `/recrue historique`.
- **Le dimanche à 18:00**, un rappel dans le salon des avis mentionne les gradés qui n'ont pas donné leur
  avis sur toutes les recrues de la semaine (une fois par semaine, rien si tout le monde a voté).
- Recevoir le rôle recrue crée un salon `essai-<pseudo>` visible par la recrue, les gradés et les leads.
  Perdre le rôle retire de la liste mais conserve le salon.
- `/recrue liste`, `avis`, `ajouter`, `retirer`, `salon`, `historique` (gradés) ; `veto poser` / `veto lever`
  (`recrue.veto`, leads par défaut : « Gérer le serveur » ne donne **pas** le véto) ; `/gestion recrue reset` force une nouvelle semaine.

## Numéros

```
/config numero salon:#annuaire
```

Annuaire permanent avec boutons « Ajouter / modifier mon numéro » et « Retirer mon numéro ». Format :
`555` + 4 à 6 chiffres (`555-1234` accepté). Un numéro par membre, unique sur le serveur.
`/numero ajouter numero [membre]` (pour un autre membre : `numero.ajouter_autrui`, « Gérer le serveur » par défaut), `voir`, `supprimer`, `liste`.

## Cotisations

Ce que chaque membre rapporte au groupe, en argent **propre** et/ou **sale**.

```
/cotisation ajouter description:"Vente de pièces" propre:25000 [sale:…] [date:JJ/MM/AAAA] [membre:@…]
```

| Commande | Pour qui | Effet |
|---|---|---|
| `ajouter` | Membres | Déclare une cotisation : description obligatoire, au moins un montant (propre et/ou sale, 100 000 000 au plus chacun), date du jour par défaut (pas dans le futur). Déclarer pour un autre membre : gradés et leads. |
| `historique [membre] [jours]` | Membres (soi) ; gradés et leads (les autres) | Dernières cotisations et totaux propre / sale. |
| `stats [jours] [membre]` | Gradés (`cotisation.stats`) | Totaux du groupe sur toute la période ou les N derniers jours, et classement des membres. |
| `supprimer` | Celui qui l'a déclarée, ou gradés et leads | Supprime une cotisation saisie par erreur (journalisé). |
| `objectif` | Membres | Où tu en es de l'objectif de la semaine. Les gradés (`cotisation.stats`) voient aussi tous les membres suivis. |

Les gradés et leads sont ceux des rôles configurés avec `/config recrue`. `/config cotisation salon:#…`
annonce chaque nouvelle cotisation dans un salon ; `sans_annonce:true` arrête les annonces.

### Objectif hebdomadaire

```
/config cotisation objectif:50000 compte:total rappel:true
```

| Option | Rôle |
|---|---|
| `objectif` | Montant à atteindre par membre chaque semaine (lundi 00:00 → dimanche 23:59, heure de Paris). `0` supprime l'objectif. |
| `compte` | Ce qui compte : **propre + sale** (défaut), argent propre seul ou argent sale seul. |
| `rappel` | `true` (défaut) : rappel le dimanche soir aux membres en dessous. |

- **Membres suivis** : porteurs du rôle membre (`/config permission role_membre:@…`), sinon du rôle des
  présences. Bots exclus.
- **Non concernés** pour la semaine : les membres ayant une absence déclarée qui chevauche la semaine (une
  absence annulée avant son début ne compte pas) et les recrues de la semaine.
- **Le dimanche à 18:00**, un rappel dans le salon des annonces (à défaut, celui du tableau de bord)
  **mentionne** les membres encore en dessous, avec leur montant. Une fois par semaine.
- **Le lundi à 09:00**, le bilan de la semaine écoulée (atteint, pas atteint, non concernés) est publié au
  même endroit, **sans notifier** personne. Un objectif tout juste créé ne fait pas de bilan de la semaine
  précédente. Si le bot était hors ligne à l'heure dite, l'envoi part à son retour, le jour même.

### Tableau de bord

```
/config cotisation tableau:#cotisations
```

Message permanent, mis à jour à chaque cotisation ajoutée ou supprimée et au changement de semaine :
total de la semaine, **classement** (10 premiers) et, si un objectif est fixé, qui l'a atteint et qui
est en dessous. Les mentions sont affichées, jamais notifiées. Changer de salon supprime l'ancien message ;
`sans_tableau:true` le retire.

Sur le site : `/cotisations` (ou `/stats/<serveur>/cotisations`), voir [Site web](site-web.md#cotisations).

## Coffres

Le jeu publie chaque dépôt et retrait des coffres de propriété dans un salon Discord (webhook
« Coffre propriété » : personnage, propriété, objet, quantité, type, heure). Le bot lit ces messages et en
déduit le **stock** de chaque coffre et l'**historique** de chaque joueur.

**Mise en place**

1. Activer *Message Content Intent* dans le portail Discord (onglet **Bot**), puis redémarrer le bot : sans
   lui, Discord ne transmet pas le contenu des encadrés.
2. `/config coffre salon:#logs-coffre` : le salon est suivi, et son historique récent (jusqu'à 5 000
   messages) est importé tout de suite. Plusieurs salons possibles ; `retirer:true` arrête le suivi.
3. Facultatif : `/coffre nommer propriete:8740 nom:"Planque du port"` et
   `/coffre lier id_jeu:340772 membre:@…` (le numéro entre parenthèses après le nom du personnage).

Les messages publiés pendant que le bot était arrêté sont rattrapés à son redémarrage ; un message n'est
jamais compté deux fois.

| Commande | Effet |
|---|---|
| `/coffre stock [propriete]` | Stock par propriété et par objet (⚠️ = sous son seuil). |
| `/coffre historique [propriete] [membre] [objet] [jours]` | Derniers dépôts et retraits (par membre : ses personnages reliés). |
| `/coffre lier id_jeu membre` | Relie un personnage du jeu à un membre Discord (journalisé). Les personnages vus sont proposés, ceux pas encore reliés d'abord. |
| `/coffre nommer propriete nom` | Nom lisible d'une propriété. |
| `/coffre seuil objet min [propriete]` | Seuil de stock bas d'un objet, dans une propriété ou dans toutes ; `min:0` retire le seuil (journalisé). |
| `/coffre seuils` | Seuils réglés, et les propriétés où l'objet est actuellement en dessous. |
| `/coffre inventaire propriete [objet quantite] [liste]` | Saisit la quantité **réellement présente** en jeu ; l'écart devient une correction (voir ci-dessous). Sans objet ni liste : formulaire pour coller l'inventaire du jeu, avec aperçu. |

Réservé aux gradés par défaut (`coffre.voir`, `coffre.gerer`). Le stock est calculé à partir des mouvements lus depuis
le début du suivi : un objet déjà présent avant peut apparaître en négatif après un retrait, jusqu'au premier inventaire.

**Inventaire** (`coffre.gerer`) : le stock d'avant le suivi, ou des objets entrés sans message du jeu, se
rattrapent en comptant ce qu'il y a vraiment dans le coffre :

```
/coffre inventaire propriete:8740 objet:repair_kit quantite:42
/coffre inventaire propriete:8740 liste:repair_kit:42, acetone:2790, red_powder:650
/coffre inventaire propriete:8740                ← formulaire pour coller l'inventaire du jeu
```

**Inventaire collé** : sans `objet` ni `liste`, un formulaire s'ouvre pour coller l'inventaire tel que copié du
jeu (une ligne par objet). Le bot répond en privé avec un **aperçu** : chaque écart (stock connu → quantité
comptée, +/−) et le nombre d'objets inchangés, avec **Valider** et **Annuler**. Rien n'est enregistré avant
« Valider » ; les écarts sont recalculés à ce moment-là (le stock a pu bouger). La saisie attend 15 minutes,
réservée à son auteur. Formats acceptés, mélangeables : `repair_kit:42`, `repair_kit=42`, `repair_kit 42`,
`repair_kit x42`, `42x repair_kit`, `42 × repair_kit`, tabulations, puces ou tirets en début de ligne
(`- repair_kit x42`). Le site lit exactement les mêmes formats.

Le bot n'écrase rien : il enregistre l'**écart** comme une correction signée (« 🧮 +34 (correction) »), avec son
auteur, inscrite au journal du staff. Le stock vaut ensuite dépôts − retraits + corrections, et les mouvements du
jeu s'ajoutent normalement par-dessus. Un objet déjà juste ne produit rien ; recompter plus tard corrige seulement la
nouvelle différence. L'identifiant d'un objet est celui du jeu (`repair_kit`), proposé par l'autocomplétion pour
les objets déjà vus. Sur le site, le bouton **Inventaire** d'une propriété permet de tout saisir d'un coup.

Pourquoi pas une relecture du salon pour « resynchroniser » ? Elle ne retrouverait que les messages encore présents :
un message supprimé avant d'être lu, ou un stock d'avant le webhook, resteraient invisibles. Les mouvements déjà lus
restent en base même si leur message est supprimé ensuite.

**Alertes**

```
/config coffre alertes_salon:#alertes-coffre alertes_role:@Logistique gros_retrait:50 joueur_inconnu:true
```

| Option | Rôle |
|---|---|
| `alertes_salon` | Salon des alertes. `alertes_desactiver:true` les arrête (les réglages sont conservés). |
| `alertes_role` | Rôle mentionné dans chaque alerte : **seule** mention notifiée. |
| `gros_retrait` | Alerte pour un retrait d'au moins cette quantité, en une fois (`0` : désactivé). |
| `joueur_inconnu` | Alerte quand un personnage relié à aucun membre (`/coffre lier`) retire un objet. |

Les seuils se règlent avec `/coffre seuil` : un seuil propre à une propriété remplace, pour elle, celui de
« toutes les propriétés ». L'alerte de stock bas part quand la quantité **passe** sous le seuil (elle
était au moins au seuil avant) : un stock déjà bas ne relance pas d'alerte à chaque retrait.

Un seul message par lot de nouveaux mouvements (un message du jeu, ou tout le rattrapage au redémarrage du
bot) résume les objets passés sous leur seuil et les retraits à surveiller. Le **premier import** de
l'historique d'un salon (`/config coffre salon:#…`) n'envoie aucune alerte.

**Tableau du stock** : `/config coffre tableau:#stock` publie un message permanent, le stock par propriété,
avec les objets sous leur seuil signalés (⚠️) et listés en premier. Il est mis à jour après de nouveaux
mouvements, **au plus une fois par minute** (le jeu publie parfois des dizaines de mouvements d'affilée).
Changer de salon supprime l'ancien message ; `sans_tableau:true` le retire.

**Messages pris en compte** : ceux des **webhooks** qui publient des encadrés « Coffre », y compris plusieurs dans le
même salon (une propriété par webhook, par exemple « Stock gradé » et « Stock vide poche »). Chaque nouveau webhook
est reconnu au premier encadré et **signalé au journal du staff**, avec son nom et son identifiant. Sont ignorés :
les messages des membres et des bots, et les réponses aux commandes d'autres applications (Carl-bot `/embed`…),
pour qu'un membre ne puisse pas imiter un encadré. Créer un webhook dans le salon demande « Gérer les webhooks ».
Une quantité supérieure à 1 000 000 est ignorée.

```
/config coffre                                   ← salons suivis et webhooks reconnus
/config coffre oublier_webhook:123456789012345   ← ne plus lire ce webhook (les mouvements déjà lus restent)
/config coffre salon:#logs-coffre relire:true    ← relit l'historique : ajoute ce qui manque, ne retire rien
```

Un salon suivi **supprimé de Discord** n'est plus suivi automatiquement au démarrage suivant, avec une
entrée au journal du staff (les mouvements déjà lus restent).

`relire` sert après un oubli (bot arrêté longtemps, webhook ignoré) : un message déjà compté n'est jamais recompté,
et un message supprimé du salon ne retire rien de ce qui a déjà été lu.

Sur le site : `/stats/<serveur>/coffres`, voir [Site web](site-web.md#coffres).

## Contacts

Répertoire des autres groupes, entreprises et personnes du jeu, avec leur relation au groupe :
🟢 **allié**, ⚪ **neutre** ou 🔴 **hostile**. Chaque contact a une description publique et, en option, un
téléphone. Les membres du groupe ajoutent des contacts ; les gradés et leads les modifient et y ajoutent
des **notes réservées au staff**.

```
/contact ajouter nom:"Garage Benny's" type:Entreprise relation:Allié [telephone:555-0142] [description:…]
```

| Commande | Pour qui | Effet |
|---|---|---|
| `voir contact` | Membres | Fiche du contact (type, relation, téléphone, description), en privé. Les gradés et leads y voient aussi les 10 dernières notes du staff (auteur et date). |
| `liste [relation] [type]` | Membres | Tous les contacts, groupés par relation, en privé. |
| `ajouter nom type relation [telephone] [description]` | Membres du groupe (`contact.ajouter`) | Ajoute un contact. Nom unique sur le serveur (sans tenir compte des majuscules), 80 caractères au plus ; description 500 au plus. |
| `modifier contact [nom] [type] [relation] [telephone] [description]` | Gradés (`contact.modifier`) | Change les champs indiqués ; `-` efface le téléphone ou la description. |
| `supprimer contact` | Gradés (`contact.modifier`) | Supprime le contact et toutes ses notes. |
| `note contact texte` | Gradés (`contact.notes`) | Ajoute une note du staff (1000 caractères au plus), jamais montrée aux autres membres. |

Le contact se choisit dans la liste proposée par Discord (le nom exact fonctionne aussi). Ajouts,
modifications (avec l'ancienne et la nouvelle relation), suppressions et notes sont inscrits au
[journal du staff](#journal-du-staff).

`/config contact salon:#…` publie un **répertoire permanent** dans un salon, groupé par relation et mis à
jour à chaque changement ; il ne montre jamais les notes du staff. `sans_annuaire:true` le retire.

Sur le site : `/contacts` (ou `/stats/<serveur>/contacts`), avec les notes, voir
[Site web](site-web.md#contacts).

## Journal du staff

Les actions sensibles sont enregistrées : configuration modifiée (`/config`), votes forcés, présences
annulées ou planifiées, statistiques réinitialisées, blocklist, absences déclarées ou clôturées **pour un
autre membre**, transactions supprimées (commande ou bouton), modèles de transaction, recrues ajoutées ou retirées, vétos, numéros modifiés pour un
autre membre, cotisations déclarées pour un autre membre ou supprimées, contacts ajoutés, modifiés ou
supprimés et notes du staff sur les contacts, et, depuis le site, transactions créées, statut changé, objet ajouté
ou transaction annulée.

- Sur le site : `/stats/<serveur>/journal`, filtrable par action (`site.journal`, « Gérer le serveur » par défaut).
- Sur Discord, en option : `/config journal salon:#journal-staff` publie chaque action dans ce salon
  (mentions affichées, jamais notifiées). `/config journal desactiver:true` arrête la publication.

## Actions au clic droit

Clic droit sur un membre ou un message → **Applications** :

| Action | Sur | Effet | Visible par défaut |
|---|---|---|---|
| Présences du membre | Membre | Sa fiche de présence (comme `/presence stats joueur`) | Membres |
| Fiche du membre | Membre | Comme `/fiche` ([détails](#fiche-du-membre)) | `membre.fiche` (gradés par défaut) ; chacun peut voir la sienne |
| Ajouter aux recrues | Membre | L'ajoute aux recrues de la semaine et crée son salon d'essai | `recrue.suivi` (gradés par défaut) |
| Ouvrir un ticket | Membre | Ouvre un ticket avec lui (comme `!newthread`) | « Gérer les messages » ; module ticket requis |
| Signaler au staff | Message | Transmet le message (texte, auteur, lien) dans le ticket de la personne qui signale, ouvert au besoin | Tout le monde ; module ticket requis |

Discord limite à 5 les actions sur un membre : le bot en utilise 4 (3 sans le module ticket).

### Fiche du membre

`/fiche [membre]` ou clic droit → *Fiche du membre* : résumé privé de tous les modules — arrivée sur le serveur,
taux de présence global et sur les **4 dernières semaines** (même règle que le rapport hebdomadaire), absence en
cours ou à venir et absences déclarées sur 90 jours, cotisations (semaine, 30 jours, objectif), transactions
créées sur 30 jours, activité des coffres (personnages reliés, dernier mouvement), recrue, numéro, véhicules, et
le bouton 🌐 vers sa fiche sur le site. Chacun voit la sienne ; celle des autres demande `membre.fiche` (gradés
par défaut). Le détail des avis d'une recrue reste réservé à `recrue.suivi`.

## Rappels en message privé

Désactivés par défaut. « Gérer le serveur » les active :

```
/config rappels actif:true [presence_heures:2] [objectif:true]
```

- **Présence** : quand une présence ouverte commence dans moins de `presence_heures` heures (2 par défaut),
  chaque membre attendu qui n'a pas répondu reçoit un MP (titre, heure, lien vers le message pour répondre).
  Mêmes attendus que les relances : hors absents d'office (rôle d'absence, absence déclarée) et blocklist. Une
  présence publiée depuis moins de 30 minutes attend (sa publication pingue déjà).
- **Objectif de cotisation** : le **samedi à partir de 18:00** (heure de Paris), les membres suivis encore sous
  l'objectif (hors exemptés) reçoivent un MP avec le reste à verser. Il faut un objectif (`/config cotisation
  objectif:…`).

Un seul MP par présence, ou par semaine d'objectif, et par membre, même après un redémarrage du bot. Chaque MP
se termine par « Pour ne plus recevoir ces rappels : `/rappels` ».

```
/rappels                       ← voir ses rappels
/rappels presences:false       ← couper les rappels de présence
/rappels objectif:false        ← couper le rappel de l'objectif
/rappels tout:false            ← tout couper (tout:true réactive)
```

Un membre dont les messages privés sont fermés est ignoré sans erreur, et pas relancé pendant 7 jours (changer
ses choix avec `/rappels` efface ce blocage).

## Soirée Clásico

Pour les soirs de match du Real. Tout se passe dans le salon où le match est ouvert.

| Commande | Pour qui | Effet |
|---|---|---|
| `/halamadrid match [heure] [adversaire] [date]` | Gradés | Publie l'affiche (21:00 et FC Barcelone par défaut) avec les boutons **Mon prono** et **Lancer un cri**. Un seul match non réglé à la fois. |
| `/halamadrid prono madrid adverse [buteur]` | Membres | Pronostic (ou bouton **Mon prono**), modifiable jusqu'au coup d'envoi. Barème : score exact 3 pts, bonne issue 1 pt, premier buteur madrilène annoncé qui marque +1. |
| `/halamadrid parier issue mise` | Membres | Pari en **$ fictifs** (10 000 $ au départ, jamais d'argent réel) sur victoire, nul ou défaite, jusqu'au coup d'envoi. Pari mutuel : les perdants paient les gagnants au prorata des mises ; si personne n'a trouvé, les mises sont rendues. |
| `/halamadrid pronos` · `cagnotte` | Membres | Tendances et cotes (le détail de chacun s'affiche au coup d'envoi) ; sa cagnotte et les plus riches. |
| `/golazo [buteur] [adverse]` | Gradés | Annonce un but (grosse annonce, score en direct sur l'affiche, et qui avait annoncé ce buteur). |
| `/halamadrid resultat [madrid adverse] [buteurs] [role_clown]` | Gradés | Règle le match : points, gains des paris, carte du résultat (score exact, podium, pire prono). Sans score : reprend le score en direct. `role_clown` donne ce rôle pour 24 h à ceux qui ont annoncé ou parié sur la victoire adverse (il faut au bot « Gérer les rôles », rôle sous le sien). |
| `/halamadrid hommedumatch joueurs [minutes]` | Gradés | Vote à boutons (2 à 10 joueurs, 15 min par défaut), résultat annoncé à la fin. |
| `/halamadrid classement` | Membres | Points de la saison et badges : 🎯 Nostradamus (scores exacts), 🔮 Voyant (buteurs trouvés), 🤡 Optimiste culé. |
| `/halamadrid mur` | Membres | Mur de la honte du dernier match : ceux qui ont annoncé ou parié sur la victoire adverse. |
| `/halamadrid cri` · `/putabarca [cule]` | Membres | Cri de supporter ; vanne bon enfant sur le Barça (la personne visée est nommée sans être notifiée). |
| `/halamadrid reactions actif` | Gradés | Le bot répond d'une vanne (et d'un 🤡) aux messages « Visca Barça », une fois par minute et par salon (*Message Content Intent* requis). |
| `/halamadrid prochainbut camp mise` | Membres | Pari en direct, pendant le match : le Real, l'adversaire ou plus aucun but. Réglé au but suivant (ou à la fin pour « plus aucun but »), même pari mutuel. |
| `/halamadrid auto actif [salon] [role_clown] [role_madridista]` | Gradés | Mode automatique (voir ci-dessous). |
| Bouton **Madridista** de l'affiche | Membres | Prend ou retire le rôle Madridista, pingé au coup d'envoi et à chaque but du Real. |

### Mode automatique

Le bot lit l'**API publique d'ESPN** (sans clé ni réglage) chaque minute : LaLiga, Ligue des champions, Coupe et
Supercoupe d'Espagne, Coupe du monde des clubs, amicaux.

- **Ouverture** (serveurs en `/halamadrid auto actif:true`) : chaque match du Real est publié dans le salon choisi
  jusqu'à 12 h avant, avec le logo de l'adversaire ; pronos et paris ouverts. Un seul match non réglé à la fois.
- **Avant le coup d'envoi** : les deux onze de départ dès que l'API les donne (à partir d'1 h 15 avant).
- **En direct**, dans un **fil** ouvert sous l'affiche au coup d'envoi (« Créer des fils publics » requis) : coup
  d'envoi (ping Madridista), chaque but avec buteur, minute, penalty ou contre son camp et **photo du buteur**,
  « qui l'avait annoncé », paris « prochain but » réglés ; buts annulés par la VAR ; cartons rouges ; mi-temps et
  fin avec les **statistiques** (possession, tirs, tirs cadrés, corners, fautes). L'affiche suit le score et la minute.
- **Coup de sifflet final** : résultat automatique avec le score et les buteurs de l'API (points, gains, rôle
  clown, carte du résultat), sans gradé.

Un match ouvert à la main (`/halamadrid match`) est aussi suivi s'il correspond à un match du Real à moins de 2 h
près. `/golazo` et `/halamadrid resultat` restent là si l'API ne répond pas. Les rôles clown et Madridista
demandent au bot « Gérer les rôles », avec ces rôles placés sous le sien.

## État de la configuration

`/config etat` (« Gérer le serveur ») fait le point, en privé, sur chaque module : ✅ prêt, ⚠️ incomplet,
⚪ non utilisé. Pour chaque contrôle, le bot indique ce qui manque et la commande pour le corriger, par
exemple :

| Module | Contrôles |
|---|---|
| Présences | Rôle ciblé configuré, salon des présences, salon des rappels, rapport hebdomadaire |
| Absences | Rôle d'absence, salon des absences |
| Transactions, Véhicules, Numéros | Tableau de bord ou annuaire publié |
| Recrues | Rôles recrue, gradé et lead, catégorie des salons d'essai, salon des avis |
| Cotisations | Salon des annonces, objectif, membres suivis, salon du rappel, tableau de bord |
| Coffres | *Message Content Intent*, salon du webhook suivi, webhook du jeu reconnu, alertes et leurs déclencheurs, tableau du stock |
| Contacts, Journal du staff | Salon de publication |
| Rappels en MP | Activés, objectif de cotisation fixé (pour le rappel du samedi) |
| Permissions | Rôles membre, gradé et lead |
| Tickets | Serveur relié au module, salon des logs (si le module est configuré) |

Un salon ou un rôle réglé puis supprimé est signalé. Les réglages facultatifs (▫️) n'empêchent pas un
module d'être prêt. Le site affiche le même état aux administrateurs.

## Statistiques sur le site

Les membres ayant `site.stats` (« Gérer le serveur » par défaut) retrouvent sur le site du bot (`/stats`, après
[connexion Discord](site-web.md#2-connexion-discord)) : assiduité de chaque membre aux présences, absences
en cours, recrues de la semaine avec leurs avis et vétos, transactions en cours, véhicules, et l'activité des
tickets si le module est configuré.
