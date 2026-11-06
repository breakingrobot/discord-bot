// Stop : avant de rendre la main, vérifie ce qui a changé (Clippy + tests Rust, types + lint + tests de
// l'interface). Un échec renvoie l'agent au travail avec la sortie utile.
//
// - Rien à refaire si le diff n'a pas bougé depuis la dernière vérification réussie (.claude/.verify-state.json).
// - Après 3 échecs de suite sur le même diff, laisse rendre la main (l'agent doit alors signaler le blocage).
// - CLAUDE_SKIP_VERIFY=1 désactive le contrôle (réservé à l'humain).

import { createHash } from 'node:crypto'
import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'

import { block, readInput, root, run, tail } from './lib.mjs'

const MAX_ATTEMPTS = 3
const statePath = path.join(root, '.claude', '.verify-state.json')

if (process.env.CLAUDE_SKIP_VERIFY === '1') process.exit(0)
await readInput()

const status = run('git status --porcelain --untracked-files=all')
if (!status.ok) process.exit(0)
const changed = status.output
  .split(/\r?\n/u)
  .filter(Boolean)
  .map((line) => line.slice(3).replace(/^.* -> /u, '').replaceAll('"', ''))

const isRust = (file) => /^(?:src\/|migrations\/|Cargo\.(?:toml|lock)$|clippy\.toml$|deny\.toml$)/u.test(file)
const isUi = (file) => file.startsWith('ui/') && !file.startsWith('ui/dist')
const domains = { rust: changed.filter(isRust), ui: changed.filter(isUi) }
if (domains.rust.length === 0 && domains.ui.length === 0) process.exit(0)

/** Empreinte du travail en cours d'un domaine : diff suivi + contenu des fichiers non suivis. */
const fingerprint = (files) => {
  const hash = createHash('sha256')
  hash.update(run(`git diff HEAD -- ${files.map((f) => `"${f}"`).join(' ')}`).output)
  for (const file of files) {
    const full = path.join(root, file)
    if (existsSync(full)) hash.update(readFileSync(full))
  }
  return hash.digest('hex')
}

const loadState = () => {
  try {
    return JSON.parse(readFileSync(statePath, 'utf8'))
  } catch {
    return {}
  }
}
const state = loadState()
const save = () => writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`)

const checks = {
  rust: [
    ['Clippy', 'cargo clippy --all-targets --locked -q -- -D warnings'],
    ['Tests Rust', 'cargo test --locked -q'],
  ],
  ui: [
    ['Types (tsc)', 'npm run -s typecheck'],
    ['Lint (Oxlint)', 'npm run -s lint'],
    ['Tests (Vitest)', 'npm test -s'],
  ],
}
const hints = {
  'Tests Rust':
    'Si openapi_spec_is_up_to_date échoue : `mise run openapi`. Si command_definitions_are_valid échoue : une limite Discord est dépassée.',
  'Types (tsc)': 'Si le contrat d’API a changé : `mise run openapi` puis mets à jour ui/src/api/types*.ts.',
}

const failures = []
for (const [domain, files] of Object.entries(domains)) {
  if (files.length === 0) continue
  if (domain === 'ui' && !existsSync(path.join(root, 'ui', 'node_modules'))) continue
  const print = fingerprint(files)
  if (state[domain]?.ok === print) continue
  const cwd = domain === 'ui' ? path.join(root, 'ui') : root
  const failed = checks[domain].find(([, command]) => {
    const result = run(command, { cwd })
    if (!result.ok) failures.push({ domain, print, label: '', output: result.output })
    return !result.ok
  })
  if (failed) {
    failures.at(-1).label = failed[0]
  } else {
    state[domain] = { ok: print }
  }
}

if (failures.length === 0) {
  state.attempts = 0
  save()
  process.exit(0)
}

const key = failures.map((f) => f.print).join(':')
state.attempts = state.failedOn === key ? (state.attempts ?? 0) + 1 : 1
state.failedOn = key
save()
if (state.attempts > MAX_ATTEMPTS) {
  process.stdout.write('Vérification toujours en échec après plusieurs essais : signale précisément le blocage.\n')
  process.exit(0)
}

const report = failures
  .map(({ label, output }) => `### ${label} en échec\n${tail(output)}${hints[label] ? `\n→ ${hints[label]}` : ''}`)
  .join('\n\n')
block(`Vérification avant de rendre la main (essai ${state.attempts}/${MAX_ATTEMPTS}) :\n\n${report}\n\nCorrige la cause puis termine.`)
