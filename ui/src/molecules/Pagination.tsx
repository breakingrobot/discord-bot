import { ChevronLeft, ChevronRight } from 'lucide-react'

import { Button } from '@/atoms/Button'
import { formatNumber } from '@/lib/format'

export type PaginationProps = {
  page: number
  pageSize: number
  total: number
  onPage: (page: number) => void
  newerLabel?: string
  olderLabel?: string
}

export const Pagination = ({
  page,
  pageSize,
  total,
  onPage,
  newerLabel = 'Précédent',
  olderLabel = 'Suivant',
}: PaginationProps) => {
  const pages = Math.max(1, Math.ceil(total / Math.max(pageSize, 1)))
  if (pages <= 1) return null
  return (
    // Étroit : l'indicateur passe au-dessus, les deux boutons se partagent la largeur sans déborder.
    <nav className="mt-4 flex flex-wrap items-center justify-between gap-x-3 gap-y-2 text-sm" aria-label="Pagination">
      <span className="order-first w-full text-center text-ink-muted tabular-nums sm:order-none sm:w-auto">
        Page {formatNumber(page)} sur {formatNumber(pages)}
      </span>
      <Button
        variant="outline"
        size="sm"
        className="min-w-0 flex-1 sm:flex-none"
        disabled={page <= 1}
        onClick={() => onPage(page - 1)}
      >
        <ChevronLeft aria-hidden /> <span className="truncate">{newerLabel}</span>
      </Button>
      <Button
        variant="outline"
        size="sm"
        className="min-w-0 flex-1 sm:order-last sm:flex-none"
        disabled={page >= pages}
        onClick={() => onPage(page + 1)}
      >
        <span className="truncate">{olderLabel}</span> <ChevronRight aria-hidden />
      </Button>
    </nav>
  )
}
