import type { GuildAccess, Me } from '@/api/types'
import { visiblePages } from '@/lib/guild'
import { GuildPageLink } from '@/molecules/GuildPageLink'
import { GuildSwitcher } from '@/organisms/GuildSwitcher'

export type GuildNavProps = {
  me: Me
  guild: GuildAccess
  userId: string
  onNavigate?: () => void
}

export const GuildNav = ({ me, guild, userId, onNavigate }: GuildNavProps) => {
  const pages = visiblePages(guild)
  const sections = [
    { key: 'serveur', label: 'Serveur', pages: pages.filter((page) => page.section === 'serveur') },
    { key: 'reglages', label: 'Réglages', pages: pages.filter((page) => page.section === 'reglages') },
  ].filter((section) => section.pages.length > 0)
  return (
    <nav aria-label="Pages du serveur" className="space-y-5">
      <GuildSwitcher me={me} guild={guild} />
      {sections.map((section) => (
        <div key={section.key}>
          <p className="mb-1 px-2 text-xs font-medium tracking-wide text-ink-subtle uppercase">{section.label}</p>
          <ul className="space-y-0.5">
            {section.pages.map((page) => (
              <li key={page.key}>
                <GuildPageLink
                  page={page}
                  guildId={guild.id}
                  userId={userId}
                  {...(onNavigate ? { onNavigate } : {})}
                  className="flex items-center gap-2.5 rounded-md px-2 py-1.5 text-sm font-medium text-ink-muted transition-colors hover:bg-muted hover:text-ink pointer-coarse:min-h-11"
                  activeClassName="bg-brand-soft text-brand hover:bg-brand-soft hover:text-brand"
                >
                  <page.icon className="size-4" aria-hidden />
                  {page.label}
                </GuildPageLink>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  )
}
