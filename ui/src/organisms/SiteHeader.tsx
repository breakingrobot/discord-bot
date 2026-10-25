import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { useSetAtom } from 'jotai'
import { LogOut, Search, Ticket } from 'lucide-react'
import type { ReactNode } from 'react'

import { meQuery } from '@/api/queries'
import { Button } from '@/atoms/Button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/atoms/DropdownMenu'
import { LoginLink } from '@/atoms/LoginLink'
import { MemberAvatar } from '@/molecules/MemberAvatar'
import { ThemeToggle } from '@/molecules/ThemeToggle'
import { paletteOpenAtom } from '@/state/palette'

export type SiteHeaderProps = { menu?: ReactNode }

export const SiteHeader = ({ menu }: SiteHeaderProps) => {
  const me = useQuery(meQuery)
  const user = me.data?.user ?? null
  const openPalette = useSetAtom(paletteOpenAtom)
  return (
    <header className="sticky top-0 z-40 border-b bg-card/90 backdrop-blur supports-[backdrop-filter]:bg-card/75 short:static">
      <div className="mx-auto flex h-14 max-w-7xl items-center gap-2 gutter">
        {menu}
        <Link
          to="/"
          className="flex min-w-0 items-center gap-2 rounded-md font-semibold tracking-tight text-ink pointer-coarse:min-h-11"
        >
          <img src="/favicon.svg" alt="" className="size-6 shrink-0" />
          <span className="truncate">Bot Discord</span>
        </Link>
        <nav className="ml-2 hidden items-center gap-1 sm:flex" aria-label="Navigation principale">
          {me.data?.tickets && (
            <Link
              to="/tickets"
              className="inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium text-ink-muted hover:text-ink pointer-coarse:min-h-11"
              activeProps={{
                className: 'bg-brand-soft text-brand hover:text-brand',
                'aria-current': 'page',
              }}
            >
              <Ticket className="size-4" aria-hidden /> Tickets
            </Link>
          )}
        </nav>
        <div className="ml-auto flex items-center gap-1">
          {user && (
            <Button
              variant="ghost"
              className="gap-2 px-2.5 text-ink-muted"
              aria-label="Rechercher (Ctrl+K)"
              aria-keyshortcuts="Control+K Meta+K"
              onClick={() => openPalette(true)}
            >
              <Search aria-hidden />
              <span className="hidden text-sm md:inline">Rechercher</span>
              <kbd className="hidden rounded border bg-muted px-1.5 font-sans text-[11px] text-ink-subtle md:inline">
                Ctrl K
              </kbd>
            </Button>
          )}
          <ThemeToggle />
          {user ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" className="gap-2 px-2" aria-label={`Compte : ${user.name}`}>
                  <MemberAvatar member={user} className="size-7" />
                  <span className="hidden max-w-40 truncate text-ink sm:inline">{user.name}</span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuLabel>{user.name}</DropdownMenuLabel>
                {me.data?.tickets && (
                  <DropdownMenuItem asChild className="sm:hidden">
                    <Link to="/tickets">
                      <Ticket aria-hidden /> Tickets
                    </Link>
                  </DropdownMenuItem>
                )}
                <DropdownMenuSeparator />
                <DropdownMenuItem asChild>
                  {/* Déconnexion servie par le bot : navigation complète. */}
                  <a href="/logout">
                    <LogOut aria-hidden /> Déconnexion
                  </a>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          ) : (
            me.data?.loginEnabled && (
              <Button asChild size="sm">
                <LoginLink loginUrl={me.data.loginUrl}>Se connecter</LoginLink>
              </Button>
            )
          )}
        </div>
      </div>
    </header>
  )
}
