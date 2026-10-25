-- Matchs ESPN supprimés avec `/halamadrid supprimer` : l'ouverture automatique ne les recrée pas
-- (`/halamadrid espn` peut toujours les recréer à la main).
CREATE TABLE IF NOT EXISTS "MadridDismissed" (
    "guildId" TEXT NOT NULL,
    "externalId" TEXT NOT NULL,
    PRIMARY KEY ("guildId", "externalId")
);
