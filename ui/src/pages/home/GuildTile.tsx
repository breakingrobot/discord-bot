import type { GuildAccess } from '@/api/types'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/atoms/Card'
import { visiblePages } from '@/lib/guild'
import { GuildIcon } from '@/molecules/GuildIcon'
import { GuildPageLink } from '@/molecules/GuildPageLink'

export type GuildTileProps = { guild: GuildAccess; userId: string }

export const GuildTile = ({ guild, userId }: GuildTileProps) => {
  const pages = visiblePages(guild)
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-3">
          <GuildIcon guild={guild} />
          <div className="min-w-0">
            <CardTitle className="truncate">{guild.name}</CardTitle>
            <CardDescription>{guild.isAdmin ? 'Administration du serveur' : 'Membre'}</CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        <ul className="grid gap-1 sm:grid-cols-2">
          {pages.map((page) => (
            <li key={page.key}>
              <GuildPageLink
                page={page}
                guildId={guild.id}
                userId={userId}
                className="group flex items-center gap-2.5 rounded-md px-2 py-1.5 text-sm hover:bg-muted pointer-coarse:min-h-11"
              >
                <page.icon className="size-4 text-ink-subtle group-hover:text-brand" aria-hidden />
                <span className="font-medium text-ink">{page.label}</span>
              </GuildPageLink>
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  )
}
