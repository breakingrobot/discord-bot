// Outils communs aux hooks Claude Code du dépôt (Node, sans dépendance : Windows, macOS, Linux).

import { spawnSync } from 'node:child_process'
import path from 'node:path'

/** Racine du dépôt : fournie par Claude Code, sinon le dossier courant. */
export const root = process.env.CLAUDE_PROJECT_DIR ?? process.cwd()

/** Entrée JSON du hook (stdin) ; objet vide si absente ou illisible. */
export const readInput = async () => {
  let raw = ''
  for await (const chunk of process.stdin) raw += chunk
  try {
    return JSON.parse(raw)
  } catch {
    return {}
  }
}

/** Chemin relatif à la racine, séparateurs `/` (pour comparer sous Windows). */
export const relative = (file) => path.relative(root, path.resolve(root, file)).split(path.sep).join('/')

/** Lance une commande (shell : `npm`, `npx`, `cargo` sont des .cmd/.exe sous Windows). */
export const run = (command, { cwd = root, timeout = 15 * 60_000 } = {}) => {
  const result = spawnSync(command, { cwd, shell: true, encoding: 'utf8', timeout, maxBuffer: 64 * 1024 * 1024 })
  return { ok: result.status === 0, output: `${result.stdout ?? ''}${result.stderr ?? ''}`.trim() }
}

/** Dernières lignes d'une sortie, pour un retour lisible à l'agent. */
export const tail = (text, lines = 60) => text.split(/\r?\n/u).slice(-lines).join('\n')

/** Bloque l'action (code 2) : le message est renvoyé à l'agent. */
export const block = (message) => {
  process.stderr.write(`${message}\n`)
  process.exit(2)
}
