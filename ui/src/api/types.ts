/**
 * Contrat de l'API JSON du bot (`/api/*`), source de vérité partagée par le
 * serveur Rust (src/web/api) et l'interface React.
 *
 * Conventions
 * - JSON en camelCase ; le serveur sérialise avec `#[serde(rename_all = "camelCase")]`.
 * - Dates et heures : chaînes ISO 8601 UTC (`2026-10-18T17:20:13Z`), affichées
 *   dans le fuseau du lecteur. Jours (filtres, séries) : `YYYY-MM-DD`, jour de Paris.
 * - Montants : entiers en dollars du jeu (cotisations, coffres) ; nombres décimaux
 *   pour les transactions (prix historiques).
 * - Identifiants Discord : chaînes (snowflakes > 2^53).
 * - Pagination : `?page=` (1 = première page) → `Page<T>`.
 * - Erreurs : statut HTTP + `ApiError`. 401 = pas connecté (`loginUrl` fourni),
 *   403 = pas le niveau requis, 404 = introuvable, 400/422 = saisie refusée
 *   (`error` lisible, en français, affichable tel quel).
 * - Écritures (POST/PUT/PATCH/DELETE) : corps JSON, en-têtes obligatoires
 *   `Content-Type: application/json` et `X-Requested-With: fetch` (protection CSRF,
 *   en plus du cookie `SameSite=Lax`). Réponse : l'objet créé ou modifié, ou 204.
 * - Exports CSV : liens GET simples (`…/export.csv` avec les mêmes filtres que la liste).
 * - Droits : chaque route vérifie la permission indiquée (`Perm`) ; le front s'en
 *   sert seulement pour masquer ce qui serait refusé.
 */

// ---------------------------------------------------------------------------
// Commun

export type ApiError = { error: string; loginUrl?: string }

export type Page<T> = { items: T[]; total: number; page: number; pageSize: number }

/** Membre Discord résolu par le bot (nom affiché sur le serveur). */
export type MemberRef = { id: string; name: string; avatarUrl: string | null }

export type Role = { id: string; name: string; color: number | null }

/** Niveaux de permission, du plus ouvert au plus restreint. */
export type Level = 'membre' | 'grade' | 'lead' | 'admin'

/** Clés d'action (voir src/modules/permissions/mod.rs, macro `perms!`). */
export type Perm =
  | 'presence.creer' | 'presence.stats' | 'presence.gerer' | 'presence.reset' | 'presence.blocklist'
  | 'absence.autrui'
  | 'transaction.utiliser' | 'transaction.supprimer' | 'transaction.historique' | 'transaction.modeles'
  | 'vehicule.utiliser'
  | 'numero.ajouter_autrui' | 'numero.supprimer_autrui'
  | 'recrue.suivi' | 'recrue.avis' | 'recrue.veto' | 'recrue.reset'
  | 'cotisation.ajouter' | 'cotisation.gerer' | 'cotisation.stats'
  | 'coffre.voir' | 'coffre.gerer'
  | 'contact.voir' | 'contact.ajouter' | 'contact.modifier' | 'contact.notes'
  | 'membre.fiche'
  | 'site.stats' | 'site.journal'
  | 'm2t.utiliser'
  | 'clasico.jouer' | 'clasico.gerer'

// ---------------------------------------------------------------------------
// Session — GET /api/me (toujours 200, même sans connexion)

export type Me = {
  /** `null` : pas connecté. */
  user: { id: string; name: string; avatarUrl: string | null } | null
  /** Connexion Discord configurée (DISCORD_CLIENT_SECRET + WEB_URL). */
  loginEnabled: boolean
  /** `/login?next=…` à suivre pour se connecter (redirection serveur). */
  loginUrl: string
  /** Staff des tickets (accès à /api/tickets, /api/tickets/staff). */
  tickets: boolean
  guilds: GuildAccess[]
}

export type GuildAccess = {
  id: string
  name: string
  iconUrl: string | null
  /** « Gérer le serveur » : réglages (permissions, objectifs, alertes, état). */
  isAdmin: boolean
  /** Membre du groupe (rôle membre, gradé ou lead) : absences, numéros, saisies à son nom. */
  inGroup: boolean
  /** Actions autorisées sur ce serveur. */
  perms: Perm[]
}

// ---------------------------------------------------------------------------
// Membres — sélecteurs et fiche

/** GET /api/guilds/:g/members?q=texte → 25 au plus (membres en cache du bot). */
export type MemberSearch = MemberRef[]

/**
 * GET /api/guilds/:g/members/:userId — fiche d'un membre.
 * Perm : `membre.fiche` (gradés par défaut) ; chacun peut voir sa propre fiche.
 * Une section est `null` quand le module n'a rien (ou n'est pas configuré).
 */
export type MemberProfile = {
  member: MemberRef
  joinedAt: string | null
  roles: Role[]
  presence: {
    present: number
    retard: number
    absent: number
    /** Pourcentage arrondi (présent + retard) / total, `null` sans présence clôturée. */
    rate: number | null
    recent: { id: string; title: string; date: string; status: 'PRESENT' | 'ABSENT' | 'RETARD' | 'NON_REPONDU' }[]
  } | null
  absences: { from: string; to: string; motif: string; current: boolean }[]
  cotisations: {
    totals: CotisationTotals
    /** 12 dernières semaines, lundi de Paris, les semaines vides comprises. */
    weekly: WeeklyAmount[]
    recent: CotisationEntry[]
    /** Objectif de la semaine en cours, `null` sans objectif configuré. */
    objective: ObjectiveProgress | null
  } | null
  coffres: {
    /** Personnages du jeu reliés à ce membre. */
    characters: { gameId: string; name: string }[]
    deposited: number
    withdrawn: number
    recent: CoffreMovement[]
  } | null
  recrue: {
    /** Semaines d'avis, de la plus récente à la plus ancienne. */
    weeks: { weekStart: string; pour: number; contre: number; blanc: number; veto: { by: MemberRef; motif: string } | null }[]
  } | null
  phone: string | null
  vehicles: { plate: string; model: string | null }[]
}

// ---------------------------------------------------------------------------
// Statistiques du serveur — GET /api/guilds/:g/stats (Perm `site.stats`)

export type GuildStats = {
  figures: { key: string; label: string; value: string; hint: string | null }[]
  attendance: { member: MemberRef; present: number; retard: number; absent: number; percent: number }[]
  /** Taux de présence moyen par semaine (12 dernières, lundi de Paris). */
  attendanceWeekly: { weekStart: string; rate: number | null; presences: number }[]
  recruits: { member: MemberRef; pour: number; contre: number; blanc: number; veto: { by: MemberRef; motif: string } | null }[]
  absencesCurrent: { member: MemberRef; from: string; to: string; motif: string }[]
  /** Présent seulement sur le serveur du module ticket et pour son staff. */
  tickets: TicketStats | null
}

// ---------------------------------------------------------------------------
// Transactions — Perm `transaction.historique`
// GET /api/guilds/:g/transactions?groupe&statut&type&du&au&page
// GET /api/guilds/:g/transactions/export.csv?mêmes filtres

export type TransactionStatus =
  | 'PREPARATION' | 'EN_ATTENTE_RDV' | 'AVANCE_ATTENTE_PAIEMENT' | 'LIVRE_PAYE' | 'ANNULEE' | 'CLOTUREE'

export type Transaction = {
  id: string
  createdAt: string
  groupe: string
  type: 'ACHAT' | 'VENTE'
  status: TransactionStatus
  statusLabel: string
  prix: number
  creator: MemberRef
  items: { nom: string; quantite: number; prixUnitaire: number | null }[]
  /** Note libre saisie sur Discord. */
  note?: string | null
}

export type TransactionsResponse = {
  /** Annulées comptées dans `count`, pas dans les montants. */
  totals: { count: number; achats: number; ventes: number; solde: number; inProgress: number }
  /** Du plus récent au plus ancien, 12 au plus. */
  months: { month: string /* YYYY-MM */; achats: number; ventes: number; count: number }[]
  statuses: { value: TransactionStatus; label: string }[]
  list: Page<Transaction>
}

// ---------------------------------------------------------------------------
// Cotisations
// GET  /api/guilds/:g/cotisations?jours&membre&page   (Perm `cotisation.stats`)
// GET  /api/guilds/:g/cotisations/export.csv?jours&membre
// POST /api/guilds/:g/cotisations  NewCotisation      (`cotisation.ajouter`, pour un autre : `cotisation.gerer`)
// DELETE /api/guilds/:g/cotisations/:id               (auteur, ou `cotisation.gerer`)
// GET/PUT /api/guilds/:g/cotisations/objective        (lecture `cotisation.stats`, écriture admin)

export type CotisationTotals = { count: number; propre: number; sale: number }

export type CotisationEntry = {
  id: string
  date: string /* YYYY-MM-DD */
  member: MemberRef
  propre: number
  sale: number
  description: string
  /** Gradé qui l'a déclarée pour le membre, sinon `null`. */
  declaredBy: MemberRef | null
}

export type WeeklyAmount = { weekStart: string; propre: number; sale: number }

export type CotisationsResponse = {
  totals: CotisationTotals
  members: { member: MemberRef; propre: number; sale: number; count: number }[]
  /** Semaines de la période (ou 12 dernières pour « tout »), vides comprises. */
  weekly: WeeklyAmount[]
  entries: Page<CotisationEntry>
  objective: Objective | null
  /** Semaine en cours, un par membre suivi (rôle membre), triés du plus loin de l'objectif. */
  objectiveProgress: ObjectiveProgress[]
}

export type NewCotisation = {
  /** Absent : soi-même. */
  memberId?: string
  propre: number
  sale: number
  description: string
  /** Absent : aujourd'hui. Jamais dans le futur. */
  date?: string
}

/** Objectif hebdomadaire (lundi → dimanche, heure de Paris). */
export type Objective = {
  amount: number
  /** Ce qui compte : propre + sale, ou une seule des deux. */
  counts: 'total' | 'propre' | 'sale'
  /** Rappel le dimanche aux membres en dessous de l'objectif. */
  reminder: boolean
}

export type ObjectiveProgress = {
  member: MemberRef
  amount: number
  target: number
  reached: boolean
  /** Absence déclarée ou recrue : non concerné cette semaine. */
  exempt: boolean
}

// ---------------------------------------------------------------------------
// Coffres — lecture `coffre.voir`, écriture `coffre.gerer`
// GET /api/guilds/:g/coffres/meta
// GET /api/guilds/:g/coffres/stock?propriete
// GET /api/guilds/:g/coffres/movements?propriete&objet&joueur&type&jours&page   (jours absent = 30, `tout` = tout)
// GET /api/guilds/:g/coffres/players?mêmes filtres sans page
// GET /api/guilds/:g/coffres/series?objet&propriete&jours  — évolution d'un objet
// GET /api/guilds/:g/coffres/export.csv?mêmes filtres
// PUT    /api/guilds/:g/coffres/inventory  CoffreInventory → CoffreInventoryResult  (422 : saisie refusée)
// PUT    /api/guilds/:g/coffres/properties/:propertyId  { name }  (nom vide = retirer le nom)
// PUT    /api/guilds/:g/coffres/players/:gameId          { memberId }
// DELETE /api/guilds/:g/coffres/players/:gameId
// GET/PUT /api/guilds/:g/coffres/alerts                  (lecture `coffre.voir`, écriture admin)

export type CoffreMeta = {
  properties: { id: string; name: string | null }[]
  items: { item: string; label: string }[]
  links: { gameId: string; name: string; member: MemberRef }[]
  /** Salons suivis et webhook retenu (`null` : pas encore vu). */
  channels: { channelId: string; name: string | null; webhookKnown: boolean }[]
}

export type CoffreStock = {
  properties: {
    id: string
    name: string | null
    items: {
      item: string
      label: string
      deposited: number
      withdrawn: number
      /** Somme signée des corrections d'inventaire. */
      adjusted: number
      /** Dépôts − retraits + corrections. */
      quantity: number
      low: boolean
    }[]
  }[]
}

/** Dépôt, retrait (lus dans le jeu) ou correction d'inventaire (saisie par un gradé). */
export type CoffreKind = 'DEPOSIT' | 'WITHDRAW' | 'ADJUST'

export type CoffreMovement = {
  id: string
  at: string
  kind: CoffreKind
  propertyId: string
  propertyName: string | null
  item: string
  itemLabel: string
  /** Quantité ; signée pour une correction (`ADJUST`). */
  amount: number
  /** Joueur du jeu ; pour une correction, le membre qui a saisi l'inventaire (`gameId` nul). */
  player: { gameId: string | null; name: string; member: MemberRef | null }
}

export type CoffrePlayer = {
  gameId: string | null
  name: string
  member: MemberRef | null
  deposited: number
  withdrawn: number
  count: number
}

/**
 * Un point par jour de la période, jours vides compris ; `adjusted` = corrections signées du jour,
 * `stock` = solde cumulé depuis le début du suivi (corrections comprises).
 */
export type CoffreSeries = { day: string; deposited: number; withdrawn: number; adjusted: number; stock: number }[]

/** Inventaire d'une propriété : quantités réellement comptées en jeu (1 à 300 objets, 0 à 1 000 000). */
export type CoffreInventory = { propertyId: string; items: { item: string; quantity: number }[] }

/** Écart enregistré comme correction (seuls les objets dont la quantité a changé). */
export type CoffreCorrection = { item: string; label: string; before: number; after: number }

export type CoffreInventoryResult = { corrections: CoffreCorrection[] }

export type CoffreAlerts = {
  /** Salon où publier les alertes, `null` : désactivées. */
  channelId: string | null
  /** Rôle mentionné dans les alertes. */
  roleId: string | null
  /** Seuils de stock bas : alerte quand la quantité passe sous `min`. */
  thresholds: { propertyId: string | null /* null = toutes */; item: string; min: number }[]
  /** Un retrait unique au-dessus de cette quantité déclenche une alerte (`null` : désactivé). */
  bigWithdrawal: number | null
  /** Alerte quand un personnage relié à aucun membre retire quelque chose. */
  unknownPlayer: boolean
}

// ---------------------------------------------------------------------------
// Contacts
// GET    /api/guilds/:g/contacts?relation&type&q          (`contact.voir`)
// GET    /api/guilds/:g/contacts/:id                       (`contact.voir` ; notes si `contact.notes`)
// POST   /api/guilds/:g/contacts       ContactInput        (`contact.ajouter`)
// PATCH  /api/guilds/:g/contacts/:id   Partial<ContactInput> (`contact.modifier`) ; `phone`/`description` à null = effacer
// DELETE /api/guilds/:g/contacts/:id                       (`contact.modifier`)
// POST   /api/guilds/:g/contacts/:id/notes  { body }       (`contact.notes`)
// Les écritures mettent à jour le répertoire publié sur Discord et le journal, comme les commandes.

export type ContactKind = 'GROUPE' | 'ENTREPRISE' | 'PERSONNE'
export type Relation = 'ALLIE' | 'NEUTRE' | 'HOSTILE'

export type Contact = {
  id: string
  name: string
  kind: ContactKind
  relation: Relation
  phone: string | null
  description: string | null
  createdBy: MemberRef
  createdAt: string
  updatedAt: string
  /** `null` sans `contact.notes`. */
  notesCount: number | null
}

export type ContactNote = { id: string; author: MemberRef; body: string; createdAt: string }

export type ContactsResponse = {
  counts: { allie: number; neutre: number; hostile: number }
  items: Contact[]
}

/** `notes` vide sans `contact.notes`. */
export type ContactDetail = { contact: Contact; notes: ContactNote[] }

/** Limites : nom 80, téléphone 30, description 500, note 1000 caractères. */
export type ContactInput = {
  name: string
  kind: ContactKind
  relation: Relation
  phone?: string | null
  description?: string | null
}

// ---------------------------------------------------------------------------
// Journal du staff — GET /api/guilds/:g/journal?action&page (Perm `site.journal`)

export type JournalEntry = {
  id: string
  at: string
  actor: MemberRef
  action: string
  label: string
  target: MemberRef | null
  /** Détails rendus (markdown Discord) par le serveur, HTML assaini. */
  detailsHtml: string
}

export type JournalResponse = {
  actions: { key: string; label: string }[]
  list: Page<JournalEntry>
}

// ---------------------------------------------------------------------------
// Permissions — GET /api/guilds/:g/permissions (admin), PUT idem (admin)

export type PermissionsResponse = {
  roles: { member: Role | null; grade: Role | null; lead: Role | null }
  /** Rôles du serveur, pour choisir le rôle membre (le plus haut en premier). */
  guildRoles: Role[]
  actions: {
    key: Perm
    module: string
    label: string
    defaultLevel: Level
    level: Level
    /** Stricte : seuls les rôles comptent (pas « Gérer le serveur »). */
    strict: boolean
  }[]
}

/** Seuls les champs présents changent ; `null` dans `levels` = revenir au défaut. */
export type PermissionsUpdate = {
  memberRoleId?: string | null
  levels?: Partial<Record<Perm, Level | null>>
}

// ---------------------------------------------------------------------------
// État de la configuration — GET /api/guilds/:g/config (admin)

export type ConfigStatus = {
  modules: {
    key: string
    label: string
    /** ok : prêt ; incomplete : il manque quelque chose ; off : module non utilisé. */
    status: 'ok' | 'incomplete' | 'off'
    checks: { label: string; ok: boolean; detail: string | null; /** Commande Discord pour corriger. */ fix: string | null }[]
  }[]
}

// ---------------------------------------------------------------------------
// Tickets (module ticket) — staff des tickets

// GET /api/tickets?q&status&page
export type TicketStatus = 'OPEN' | 'CLOSED' | 'SUSPENDED'

export type TicketRow = {
  id: string
  number: number
  user: { id: string; name: string }
  status: TicketStatus
  openedAt: string
  exchanges: number
  claimedBy: string | null
}

export type TicketsResponse = Page<TicketRow>

// GET /api/tickets/staff
export type TicketStats = {
  open: number
  opened: number
  closed: number
  unanswered: number
  medianFirstResponse: string | null
  staff: { name: string; replies: number; tickets: number; claims: number; firstResponses: number; averageFirstResponse: string | null }[]
}

/**
 * GET /api/logs/:threadId — log d'un ticket. Staff des tickets, ou n'importe qui
 * quand la connexion n'est pas configurée (lien non devinable, comme modmailbot).
 * Texte brut au format modmailbot : GET /logs/:threadId?raw (servi par le bot, hors SPA).
 */
export type TicketLog = {
  number: number
  user: { id: string; name: string }
  status: TicketStatus
  openedAt: string
  counts: { fromUser: number; toUser: number; internal: number }
  rawUrl: string
  entries: LogEntry[]
}

export type LogEntry = {
  id: string
  kind: 'user' | 'staff' | 'internal' | 'command' | 'system' | 'danger'
  /** Premier message d'un nouveau jour (séparateur). */
  newDay: boolean
  author: string
  /** Rôle affiché du staff (réponses). */
  role: string | null
  /** Étiquette : « Anonyme », « Note interne », « Commande », « Supprimé »… */
  badge: string | null
  /** Numéro de réponse du staff (`!edit 3`). */
  number: number | null
  at: string
  /** Markdown Discord rendu par le serveur (mentions résolues), HTML assaini. */
  bodyHtml: string
  monospace: boolean
  edit: { beforeHtml: string; afterHtml: string } | null
  images: string[]
  files: { url: string; name: string }[]
  /** Résumé des embeds, texte. */
  embeds: string | null
  /** Identifiants techniques (vue « détails techniques »). */
  technical: string | null
}

// Présences, absences, recrues, annuaire et écritures des transactions : types-groupe.ts.
export type * from './types-groupe'
