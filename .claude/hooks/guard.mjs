// PreToolUse (Edit, Write, MultiEdit) : refuse les écritures qui cassent une règle du dépôt,
// avec ce qu'il faut faire à la place.

import { block, readInput, relative, run } from './lib.mjs'

const input = await readInput()
const file = input.tool_input?.file_path ?? input.tool_input?.notebook_path
if (!file) process.exit(0)
const rel = relative(file)

if (rel === '.env' || rel.startsWith('.env.') && rel !== '.env.example') {
  block(`« ${rel} » contient des secrets : ne l'écris pas. Documente la variable dans .env.example et docs/deploiement.md.`)
}

const generated = {
  'ui/openapi.json': 'régénéré depuis le code : modifie src/web/api/ puis lance `mise run openapi`',
  'ui/src/routeTree.gen.ts': 'régénéré par Vite (TanStack Router) : ajoute ou modifie un fichier de ui/src/routes/',
  'Cargo.lock': 'tenu par cargo : modifie Cargo.toml puis lance `cargo check`',
  'ui/package-lock.json': 'tenu par npm : modifie ui/package.json puis lance `npm install` dans ui/',
}
if (generated[rel]) block(`« ${rel} » est ${generated[rel]}.`)
if (rel.startsWith('ui/src/api/generated/')) block(`« ${rel} » est généré par Hey API : lance \`mise run openapi\`.`)

// Une migration déjà sur main a pu tourner en production : on n'y touche plus.
if (/^migrations\/\d{4}_[^/]+\.sql$/u.test(rel)) {
  const base = run('git rev-parse --verify --quiet origin/main').ok ? 'origin/main' : 'HEAD'
  if (run(`git cat-file -e ${base}:${rel}`).ok) {
    block(
      `« ${rel} » est déjà sur ${base} (peut-être appliquée en production) : ne la modifie pas. ` +
        'Crée une nouvelle migration au numéro suivant (compétence `migration`).',
    )
  }
}
