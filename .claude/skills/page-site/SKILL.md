---
name: page-site
description: Créer ou modifier une page, un composant ou un formulaire du site React (ui/) en respectant la conception atomique, les droits, le responsive et les tests.
---

# Page ou composant du site

Lis d'abord `ui/README.md` (architecture, outils transverses, responsive). Modèle : `ui/src/pages/contacts/`.

1. **Route fine** dans `ui/src/routes/` (paramètres de recherche validés dans `pages/<page>/<page>-search.ts`), qui
   rend `pages/<page>/<Page>Page.tsx` ; la vue (`…View.tsx`) assemble organismes et molécules.
2. **Couches** : atomes (`atoms/`, primitives shadcn/radix) ← molécules ← organismes ← modèles ← pages. Une couche
   n'importe que des couches inférieures. Composants en fonctions fléchées, props typées, 300 lignes max par fichier.
3. **Données** : requêtes TanStack Query de `api/queries*.ts`, clés `['guild', guildId, <domaine>, …]` (le temps réel
   invalide par domaine, `lib/live.ts`). États chargement / vide / erreur / `Forbidden` comme les pages existantes.
4. **Droits** : page déclarée dans `GUILD_PAGES` (`lib/guild.ts`) avec la condition d'affichage ; le serveur vérifie
   de toute façon.
5. **Responsive** : tableau large → cartes sous `sm` (`MobileCards`) ; marges `gutter` ; petits liens `tap` ; aucun
   défilement horizontal (audit : `mise run audit:responsive`).
6. **Textes** : français, vouvoiement sur le site ; dates et montants via `lib/format.ts`.
7. **Tests** : logique pure dans `lib/` + test ; un test de composant (Testing Library) pour l'interaction principale ;
   maquette à jour pour `mise run dev:mock`.
