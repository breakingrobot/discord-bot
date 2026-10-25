import { ChevronRight } from 'lucide-react'
import { Fragment, useState } from 'react'

import type { Transaction, TransactionStatus } from '@/api/types'
import { Badge } from '@/atoms/Badge'
import type { BadgeVariant } from '@/atoms/Badge'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/atoms/Table'
import { formatDateTime, formatMoney, formatNumber } from '@/lib/format'
import { cn } from '@/lib/utils'
import { MemberName } from '@/molecules/MemberName'
import { MobileCard, MobileCards } from '@/molecules/MobileCards'
import { TransactionActions } from '@/pages/transactions/TransactionActions'

export const STATUS_VARIANT: Record<TransactionStatus, BadgeVariant> = {
  PREPARATION: 'neutral',
  EN_ATTENTE_RDV: 'warning',
  AVANCE_ATTENTE_PAIEMENT: 'warning',
  LIVRE_PAYE: 'success',
  ANNULEE: 'danger',
  CLOTUREE: 'neutral',
}

export type TransactionTableProps = {
  items: Transaction[]
  guildId: string
  /** Avec `transaction.utiliser` : statuts proposés (sans « Annulée »), actions sous les objets. */
  statuses?: { value: TransactionStatus; label: string }[]
}

export const TransactionTable = ({ items, guildId, statuses }: TransactionTableProps) => {
  const [open, setOpen] = useState<string | null>(null)
  return (
    <>
      <MobileCards label="Transactions" flush>
        {items.map((item) => (
          <MobileCard
            key={item.id}
            title={item.groupe}
            aside={
              <span
                className={cn('font-medium tabular-nums', item.status === 'ANNULEE' && 'text-ink-subtle line-through')}
              >
                {formatMoney(item.prix, true)}
              </span>
            }
            fields={[
              { label: 'Date', value: formatDateTime(item.createdAt) },
              {
                label: 'Type',
                value: (
                  <Badge variant={item.type === 'ACHAT' ? 'brand' : 'success'}>
                    {item.type === 'ACHAT' ? 'Achat' : 'Vente'}
                  </Badge>
                ),
              },
              {
                label: 'Statut',
                value: <Badge variant={STATUS_VARIANT[item.status]}>{item.statusLabel}</Badge>,
              },
              { label: 'Créée par', value: item.creator.name },
            ]}
          >
            {item.note && <p className="mt-2 text-sm whitespace-pre-line text-ink-muted">{item.note}</p>}
            {item.items.length > 0 && (
              <details className="mt-2">
                <summary className="min-h-11 cursor-pointer content-center text-ink-muted">
                  {formatNumber(item.items.length)} objet(s)
                </summary>
                <ul className="grid gap-1">
                  {item.items.map((line, index) => (
                    // oxlint-disable-next-line react/no-array-index-key -- les lignes d'objets n'ont pas d'identifiant
                    <li key={`${line.nom}-${index}`} className="flex justify-between gap-3">
                      <span>
                        {line.nom} <span className="text-ink-muted">× {formatNumber(line.quantite)}</span>
                      </span>
                      <span className="text-ink-muted tabular-nums">
                        {line.prixUnitaire === null ? '—' : `${formatMoney(line.prixUnitaire, true)} / u`}
                      </span>
                    </li>
                  ))}
                </ul>
              </details>
            )}
            {statuses && item.status !== 'CLOTUREE' && (
              <TransactionActions guildId={guildId} transaction={item} statuses={statuses} />
            )}
          </MobileCard>
        ))}
      </MobileCards>
      <div className="hidden sm:block">
        <Table>
          <TableHeader>
            <TableRow className="hover:bg-transparent">
              <TableHead className="w-8">
                <span className="sr-only">Objets</span>
              </TableHead>
              <TableHead>Date</TableHead>
              <TableHead>Groupe</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>Statut</TableHead>
              <TableHead className="text-right">Prix</TableHead>
              <TableHead>Créée par</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {items.map((item) => {
              const expanded = open === item.id
              return (
                <Fragment key={item.id}>
                  <TableRow
                    className={cn('cursor-pointer', expanded && 'bg-muted/50')}
                    onClick={() => setOpen(expanded ? null : item.id)}
                  >
                    <TableCell className="pr-0">
                      <button
                        type="button"
                        aria-expanded={expanded}
                        aria-controls={`objets-${item.id}`}
                        aria-label={`${expanded ? 'Masquer' : 'Afficher'} les objets de la transaction avec ${item.groupe}`}
                        className="tap flex size-6 items-center justify-center rounded-sm text-ink-subtle hover:text-ink focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none"
                        onClick={(event) => {
                          event.stopPropagation()
                          setOpen(expanded ? null : item.id)
                        }}
                      >
                        <ChevronRight
                          className={cn('size-4 transition-transform', expanded && 'rotate-90')}
                          aria-hidden
                        />
                      </button>
                    </TableCell>
                    <TableCell className="text-ink-muted tabular-nums">{formatDateTime(item.createdAt)}</TableCell>
                    <TableCell className="font-medium">{item.groupe}</TableCell>
                    <TableCell>
                      <Badge variant={item.type === 'ACHAT' ? 'brand' : 'success'}>
                        {item.type === 'ACHAT' ? 'Achat' : 'Vente'}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <Badge variant={STATUS_VARIANT[item.status]}>{item.statusLabel}</Badge>
                    </TableCell>
                    <TableCell
                      className={cn(
                        'text-right font-medium tabular-nums',
                        item.status === 'ANNULEE' && 'text-ink-subtle line-through',
                      )}
                    >
                      {formatMoney(item.prix, true)}
                    </TableCell>
                    <TableCell>
                      <MemberName member={item.creator} guildId={guildId} linked={false} />
                    </TableCell>
                  </TableRow>
                  {expanded && (
                    <TableRow id={`objets-${item.id}`} className="bg-muted/30 hover:bg-muted/30">
                      <TableCell />
                      <TableCell colSpan={6} className="py-3">
                        {item.note && <p className="mb-2 text-sm whitespace-pre-line text-ink-muted">{item.note}</p>}
                        {item.items.length === 0 ? (
                          <p className="text-sm text-ink-muted">Aucun objet saisi.</p>
                        ) : (
                          <ul className="grid gap-1 text-sm sm:max-w-md">
                            {item.items.map((line, index) => (
                              // oxlint-disable-next-line react/no-array-index-key -- les lignes d'objets n'ont pas d'identifiant
                              <li key={`${line.nom}-${index}`} className="flex justify-between gap-4">
                                <span>
                                  <span className="font-medium">{line.nom}</span>{' '}
                                  <span className="text-ink-muted">× {formatNumber(line.quantite)}</span>
                                </span>
                                <span className="text-ink-muted tabular-nums">
                                  {line.prixUnitaire === null
                                    ? 'prix non précisé'
                                    : `${formatMoney(line.prixUnitaire, true)} / u`}
                                </span>
                              </li>
                            ))}
                          </ul>
                        )}
                        {statuses &&
                          (item.status === 'CLOTUREE' ? (
                            <p className="mt-3 text-xs text-ink-subtle">Clôturée : plus modifiable.</p>
                          ) : (
                            <TransactionActions guildId={guildId} transaction={item} statuses={statuses} />
                          ))}
                      </TableCell>
                    </TableRow>
                  )}
                </Fragment>
              )
            })}
          </TableBody>
        </Table>
      </div>
    </>
  )
}
