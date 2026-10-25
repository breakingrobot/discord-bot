-- Soirée Clásico automatique (API publique d'ESPN) : match relié à l'événement
-- de l'API, minute en direct, fil du match, annonces déjà faites, ouverture
-- automatique, rôles clown et Madridista, paris « prochain but ».
ALTER TABLE "MadridMatch" ADD COLUMN "externalId" TEXT;
-- Compétition ESPN de l'événement (ex : esp.1), pour lire compos et statistiques.
ALTER TABLE "MadridMatch" ADD COLUMN "league" TEXT;
ALTER TABLE "MadridMatch" ADD COLUMN "liveClock" TEXT;
-- Logo de l'adversaire (vignette des messages).
ALTER TABLE "MadridMatch" ADD COLUMN "rivalLogo" TEXT;
-- Fil Discord où partent les annonces en direct (NULL : salon du match).
ALTER TABLE "MadridMatch" ADD COLUMN "threadId" TEXT;
-- Annonces déjà publiées, séparées par des virgules (kickoff, ht, ft, lineups, rouge:…).
ALTER TABLE "MadridMatch" ADD COLUMN "announced" TEXT NOT NULL DEFAULT '';
CREATE UNIQUE INDEX IF NOT EXISTS "MadridMatch_guildId_externalId_key"
    ON "MadridMatch"("guildId", "externalId") WHERE "externalId" IS NOT NULL;

-- Salon où publier automatiquement les matchs du Real (NULL : pas d'ouverture automatique).
ALTER TABLE "MadridConfig" ADD COLUMN "autoChannelId" TEXT;
-- Rôle clown donné au résultat (NULL : aucun).
ALTER TABLE "MadridConfig" ADD COLUMN "clownRoleId" TEXT;
-- Rôle « Madridista » que chacun prend ou retire, pingé au coup d'envoi et aux buts du Real.
ALTER TABLE "MadridConfig" ADD COLUMN "madridistaRoleId" TEXT;

-- Paris en direct « prochain but » : une manche par nombre de buts déjà marqués.
CREATE TABLE IF NOT EXISTS "MadridLiveBet" (
    "matchId" TEXT NOT NULL REFERENCES "MadridMatch"("id") ON DELETE CASCADE,
    -- Buts marqués au moment du pari (la manche se règle au but suivant ou à la fin).
    "round" INTEGER NOT NULL,
    "userId" TEXT NOT NULL,
    -- REAL | ADVERSE | AUCUN
    "choice" TEXT NOT NULL,
    "mise" INTEGER NOT NULL,
    "gain" INTEGER,
    PRIMARY KEY ("matchId", "round", "userId")
);
