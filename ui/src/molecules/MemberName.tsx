import { Link } from '@tanstack/react-router'

import type { MemberRef } from '@/api/types'
import { cn } from '@/lib/utils'
import { MemberAvatar } from '@/molecules/MemberAvatar'

export type MemberNameProps = {
  member: MemberRef
  guildId?: string | undefined
  linked?: boolean
  avatar?: boolean
  className?: string
}

/** Nom d'un membre avec avatar ; lien vers sa fiche quand `guildId` est donné et la fiche visible. */
export const MemberName = ({ member, guildId, linked = true, avatar = true, className }: MemberNameProps) => {
  const content = (
    <>
      {avatar && <MemberAvatar member={member} className="size-6" />}
      <span className="truncate">{member.name}</span>
    </>
  )
  const base = cn('inline-flex min-w-0 items-center gap-2 font-medium text-ink', className)
  if (guildId && linked) {
    return (
      <Link
        to="/serveurs/$guildId/membres/$userId"
        params={{ guildId, userId: member.id }}
        className={cn(base, 'tap rounded-sm hover:text-brand hover:underline')}
      >
        {content}
      </Link>
    )
  }
  return <span className={base}>{content}</span>
}
