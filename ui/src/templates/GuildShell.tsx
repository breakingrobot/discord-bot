import { Outlet } from '@tanstack/react-router'
import { useAtom, useSetAtom } from 'jotai'
import { ChevronsUpDown } from 'lucide-react'
import { useEffect } from 'react'

import type { GuildAccess, Me } from '@/api/types'
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle, SheetTrigger } from '@/atoms/Sheet'
import { useLiveUpdates } from '@/hooks/use-live-updates'
import { hasPerm } from '@/lib/guild'
import { GuildContext } from '@/lib/guild-context'
import { LiveIndicator } from '@/molecules/LiveIndicator'
import { AddCotisationDialog } from '@/organisms/AddCotisationDialog'
import { GuildNav } from '@/organisms/GuildNav'
import { GuildTabBar } from '@/organisms/GuildTabBar'
import { guildNavOpenAtom } from '@/state/guild-nav'
import { lastGuildIdAtom } from '@/state/last-guild'
import { quickCotisationOpenAtom } from '@/state/palette'

export type GuildShellProps = { me: Me; guild: GuildAccess; userId: string }

export const GuildShell = ({ me, guild, userId }: GuildShellProps) => {
  const [open, setOpen] = useAtom(guildNavOpenAtom)
  const [cotisationOpen, setCotisationOpen] = useAtom(quickCotisationOpenAtom)
  const setLastGuildId = useSetAtom(lastGuildIdAtom)
  // Mise à jour en direct des pages du serveur (flux SSE), tant que l'espace du serveur est ouvert.
  const live = useLiveUpdates(guild.id)
  const context = { me, guild, userId }
  useEffect(() => {
    setLastGuildId(guild.id)
  }, [guild.id, setLastGuildId])
  // Tiroir et dialogue refermés en quittant l'espace du serveur (les atomes survivent au démontage).
  useEffect(
    () => () => {
      setOpen(false)
      setCotisationOpen(false)
    },
    [setOpen, setCotisationOpen],
  )
  return (
    <GuildContext value={context}>
      <div className="mx-auto flex w-full max-w-7xl flex-1">
        <aside className="hidden w-60 shrink-0 border-r lg:block">
          <div className="sticky top-14 flex max-h-[calc(100dvh-3.5rem)] flex-col overflow-y-auto px-3 py-5">
            <GuildNav me={me} guild={guild} userId={userId} />
            <LiveIndicator status={live} className="mt-6 px-2" />
          </div>
        </aside>
        <div className="min-w-0 flex-1">
          <div className="sticky top-14 z-30 flex items-center gap-2 border-b bg-canvas/95 py-1 gutter backdrop-blur lg:hidden short:static">
            <Sheet open={open} onOpenChange={setOpen}>
              <SheetTrigger asChild>
                <button
                  type="button"
                  aria-label={`Pages du serveur ${guild.name}`}
                  className="-ml-2 flex min-h-11 min-w-0 flex-1 items-center gap-1.5 rounded-md px-2 text-left text-sm font-medium text-ink outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50"
                >
                  <span className="truncate">{guild.name}</span>
                  <ChevronsUpDown className="size-4 shrink-0 text-ink-subtle" aria-hidden />
                </button>
              </SheetTrigger>
              <SheetContent side="left" className="w-72 overflow-y-auto pb-[env(safe-area-inset-bottom)]">
                <SheetHeader>
                  <SheetTitle>{guild.name}</SheetTitle>
                  <SheetDescription className="sr-only">Pages du serveur</SheetDescription>
                </SheetHeader>
                <div className="px-3 pb-6">
                  <GuildNav me={me} guild={guild} userId={userId} onNavigate={() => setOpen(false)} />
                </div>
              </SheetContent>
            </Sheet>
            <LiveIndicator status={live} />
          </div>
          {/* Marge du bas : la barre d'onglets fixe ne cache pas la fin de la page. */}
          <main id="contenu" className="pt-5 gutter pb-24 sm:pt-6 lg:pb-6">
            <Outlet />
          </main>
        </div>
      </div>
      <GuildTabBar guild={guild} userId={userId} />
      {hasPerm(guild, 'cotisation.ajouter') && (
        <AddCotisationDialog
          guildId={guild.id}
          canManage={hasPerm(guild, 'cotisation.gerer')}
          open={cotisationOpen}
          onOpenChange={setCotisationOpen}
        />
      )}
    </GuildContext>
  )
}
