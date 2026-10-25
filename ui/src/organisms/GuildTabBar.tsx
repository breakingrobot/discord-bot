// Barre d'onglets du bas (écrans étroits) : quatre raccourcis parmi les pages ouvertes au compte,
// et « Plus » qui ouvre le tiroir de toutes les pages du serveur.

import { useSetAtom } from 'jotai'
import { Menu } from 'lucide-react'

import type { GuildAccess } from '@/api/types'
import { visiblePages } from '@/lib/guild'
import type { GuildPage, GuildPageKey } from '@/lib/guild'
import { GuildPageLink } from '@/molecules/GuildPageLink'
import { guildNavOpenAtom } from '@/state/guild-nav'

/** Ordre de préférence des raccourcis : les pages du quotidien d'abord. */
const PRIORITY: GuildPageKey[] = [
  'accueil',
  'cotisations',
  'coffres',
  'contacts',
  'transactions',
  'membres',
  'fiche',
  'journal',
  'stats',
]

/** Les quatre raccourcis de la barre, selon les droits. */
export const tabPages = (guild: GuildAccess): GuildPage[] => {
  const pages = visiblePages(guild)
  return PRIORITY.flatMap((key) => pages.filter((page) => page.key === key)).slice(0, 4)
}

const TAB =
  'flex min-h-14 flex-1 flex-col short:min-h-11 items-center justify-center gap-0.5 px-1 text-[11px] font-medium text-ink-muted outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50'

export type GuildTabBarProps = { guild: GuildAccess; userId: string }

export const GuildTabBar = ({ guild, userId }: GuildTabBarProps) => {
  const setNavOpen = useSetAtom(guildNavOpenAtom)
  return (
    <nav
      aria-label="Raccourcis du serveur"
      className="fixed inset-x-0 bottom-0 z-40 border-t bg-card/95 pb-[env(safe-area-inset-bottom)] backdrop-blur lg:hidden"
    >
      <ul className="mx-auto flex max-w-lg">
        {tabPages(guild).map((page) => (
          <li key={page.key} className="flex flex-1">
            <GuildPageLink page={page} guildId={guild.id} userId={userId} className={TAB} activeClassName="text-brand">
              <page.icon className="size-5" aria-hidden />
              <span className="max-w-full truncate">{page.label}</span>
            </GuildPageLink>
          </li>
        ))}
        <li className="flex flex-1">
          <button type="button" className={TAB} onClick={() => setNavOpen(true)}>
            <Menu className="size-5" aria-hidden />
            Plus
          </button>
        </li>
      </ul>
    </nav>
  )
}
