-- Soirée Clásico (`/halamadrid`, `/golazo`, `/putabarca`) : matchs ouverts aux
-- pronostics, pronostics, paris en argent fictif, homme du match, rôle clown.
CREATE TABLE IF NOT EXISTS "MadridMatch" (
    "id" TEXT NOT NULL PRIMARY KEY,
    "guildId" TEXT NOT NULL,
    -- Adversaire affiché, ex : « FC Barcelone ».
    "adversaire" TEXT NOT NULL,
    -- Coup d'envoi : pronostics et paris se ferment à cette heure.
    "kickoff" DATETIME NOT NULL,
    "channelId" TEXT NOT NULL,
    "messageId" TEXT,
    -- Score en direct (`/golazo` ou suivi automatique).
    "liveMadrid" INTEGER NOT NULL DEFAULT 0,
    "liveAdverse" INTEGER NOT NULL DEFAULT 0,
    -- Statut lu dans l'API de football (IN_PLAY, PAUSED, FINISHED…), NULL sans suivi.
    "liveStatus" TEXT,
    -- Score final, saisi avec `/halamadrid resultat` (NULL : match pas encore réglé).
    "scoreMadrid" INTEGER,
    "scoreAdverse" INTEGER,
    -- Buteurs madrilènes, séparés par des virgules.
    "buteurs" TEXT,
    "createdById" TEXT NOT NULL,
    "createdAt" DATETIME NOT NULL,
    "settledAt" DATETIME
);
CREATE INDEX IF NOT EXISTS "MadridMatch_guildId_idx" ON "MadridMatch"("guildId", "kickoff");

CREATE TABLE IF NOT EXISTS "MadridProno" (
    "matchId" TEXT NOT NULL REFERENCES "MadridMatch"("id") ON DELETE CASCADE,
    "userId" TEXT NOT NULL,
    "madrid" INTEGER NOT NULL,
    "adverse" INTEGER NOT NULL,
    -- Premier buteur madrilène annoncé (bonus s'il marque).
    "buteur" TEXT,
    -- Points gagnés, calculés au résultat.
    "points" INTEGER,
    "updatedAt" DATETIME NOT NULL,
    PRIMARY KEY ("matchId", "userId")
);

-- Argent fictif des paris (jamais lié à un vrai compte).
CREATE TABLE IF NOT EXISTS "MadridWallet" (
    "guildId" TEXT NOT NULL,
    "userId" TEXT NOT NULL,
    "balance" INTEGER NOT NULL,
    PRIMARY KEY ("guildId", "userId")
);

CREATE TABLE IF NOT EXISTS "MadridBet" (
    "matchId" TEXT NOT NULL REFERENCES "MadridMatch"("id") ON DELETE CASCADE,
    "userId" TEXT NOT NULL,
    -- VICTOIRE | NUL | DEFAITE (vu du Real).
    "issue" TEXT NOT NULL,
    "mise" INTEGER NOT NULL,
    -- Gain versé au résultat (mise comprise), 0 si perdu.
    "gain" INTEGER,
    PRIMARY KEY ("matchId", "userId")
);

-- Vote de l'homme du match.
CREATE TABLE IF NOT EXISTS "MadridMotm" (
    "matchId" TEXT NOT NULL PRIMARY KEY REFERENCES "MadridMatch"("id") ON DELETE CASCADE,
    -- Joueurs proposés, séparés par des virgules.
    "joueurs" TEXT NOT NULL,
    "channelId" TEXT NOT NULL,
    "messageId" TEXT,
    "closesAt" DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS "MadridMotmVote" (
    "matchId" TEXT NOT NULL REFERENCES "MadridMatch"("id") ON DELETE CASCADE,
    "userId" TEXT NOT NULL,
    "joueur" INTEGER NOT NULL,
    PRIMARY KEY ("matchId", "userId")
);

-- Rôle « clown » donné pour 24 h aux pronostiqueurs d'une victoire de l'adversaire.
CREATE TABLE IF NOT EXISTS "MadridClown" (
    "guildId" TEXT NOT NULL,
    "userId" TEXT NOT NULL,
    "roleId" TEXT NOT NULL,
    "until" DATETIME NOT NULL,
    PRIMARY KEY ("guildId", "userId", "roleId")
);

-- Réglages : réponses automatiques aux « Visca Barça ».
CREATE TABLE IF NOT EXISTS "MadridConfig" (
    "guildId" TEXT NOT NULL PRIMARY KEY,
    "reactions" BOOLEAN NOT NULL DEFAULT 0
);
