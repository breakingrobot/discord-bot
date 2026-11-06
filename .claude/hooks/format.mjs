// PostToolUse (Edit, Write, MultiEdit) : formate le fichier modifié. Une erreur (syntaxe cassée)
// est renvoyée tout de suite à l'agent plutôt qu'à la fin.

import { existsSync } from 'node:fs'
import path from 'node:path'

import { block, readInput, relative, root, run, tail } from './lib.mjs'

const input = await readInput()
const file = input.tool_input?.file_path
if (!file || !existsSync(file)) process.exit(0)
const rel = relative(file)

if (rel.endsWith('.rs')) {
  // rustfmt lit rustfmt.toml (édition, largeur) depuis la racine du dépôt.
  const result = run(`rustfmt --edition 2024 "${rel}"`)
  if (!result.ok) block(`rustfmt a échoué sur ${rel} :\n${tail(result.output, 30)}`)
} else if (rel.startsWith('ui/') && /\.(?:ts|tsx|js|mjs|css|json)$/u.test(rel) && !rel.includes('/generated/')) {
  if (!existsSync(path.join(root, 'ui', 'node_modules'))) process.exit(0)
  const result = run(`npx --no-install oxfmt "${rel.slice('ui/'.length)}"`, { cwd: path.join(root, 'ui') })
  if (!result.ok) block(`oxfmt a échoué sur ${rel} :\n${tail(result.output, 30)}`)
}
