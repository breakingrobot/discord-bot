import { Coins } from 'lucide-react'

import type { CotisationsResponse } from '@/api/types'
import { Card, CardContent } from '@/atoms/Card'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/atoms/Table'
import { formatMoney, formatNumber, formatPercent } from '@/lib/format'
import { useGuild } from '@/lib/guild-context'
import { COUNTS_LABEL } from '@/lib/objective'
import { EmptyState } from '@/molecules/EmptyState'
import { MemberName } from '@/molecules/MemberName'
import { ObjectiveBar } from '@/molecules/ObjectiveBar'
import { ObjectiveStatus } from '@/molecules/ObjectiveStatus'
import { Pagination } from '@/molecules/Pagination'
import { Section } from '@/molecules/Section'
import { ShareBar } from '@/molecules/ShareBar'
import { StatCard } from '@/molecules/StatCard'
import { StatGrid } from '@/molecules/StatGrid'
import { EntriesTable } from '@/organisms/EntriesTable'
import { WeeklyCotisationsChart } from '@/organisms/WeeklyCotisationsChart'

export type CotisationsContentProps = {
  data: CotisationsResponse
  filteredMember: boolean
  onMember: (memberId: string) => void
  onPage: (page: number) => void
}

export const CotisationsContent = ({ data, filteredMember, onMember, onPage }: CotisationsContentProps) => {
  const { guild } = useGuild()
  const total = data.totals.propre + data.totals.sale
  return (
    <>
      <StatGrid className="mt-6 lg:grid-cols-5">
        <StatCard label="Total" value={formatMoney(total)} tone="staff" />
        <StatCard label="Argent propre" value={formatMoney(data.totals.propre)} />
        <StatCard label="Argent sale" value={formatMoney(data.totals.sale)} tone="internal" />
        <StatCard label="Cotisations" value={formatNumber(data.totals.count)} />
        <StatCard label="Membres" value={formatNumber(data.members.length)} hint="ayant cotisé" />
      </StatGrid>

      <Section title="Par semaine" description="Argent propre et sale, lundi → dimanche (heure de Paris).">
        <Card className="pb-3">
          <CardContent>
            <WeeklyCotisationsChart weekly={data.weekly} />
          </CardContent>
        </Card>
      </Section>

      {data.objective && (
        <Section
          title="Objectif de la semaine"
          description={`${formatMoney(data.objective.amount)} par membre (${COUNTS_LABEL[data.objective.counts]})${data.objective.reminder ? ', rappel le dimanche' : ''}. Du plus loin au plus proche de l'objectif.`}
        >
          {data.objectiveProgress.length === 0 ? (
            <EmptyState title="Aucun membre suivi" />
          ) : (
            <Card className="gap-0 py-0">
              <ul className="grid divide-y sm:grid-cols-2 sm:divide-y-0 lg:grid-cols-3">
                {data.objectiveProgress.map((progress) => (
                  <li key={progress.member.id} className="grid min-w-0 gap-2 px-5 py-3 sm:border-b">
                    <div className="flex min-w-0 items-center justify-between gap-2">
                      <MemberName member={progress.member} guildId={guild.id} />
                      <ObjectiveStatus progress={progress} />
                    </div>
                    <ObjectiveBar progress={progress} />
                  </li>
                ))}
              </ul>
            </Card>
          )}
        </Section>
      )}

      {!filteredMember && (
        <Section title="Classement" description="Cliquez sur un membre pour ne voir que ses cotisations.">
          {data.members.length === 0 ? (
            <EmptyState title="Aucune cotisation sur la période" icon={<Coins />} />
          ) : (
            <Card className="py-0">
              <Table>
                <TableHeader>
                  <TableRow className="hover:bg-transparent">
                    <TableHead className="w-10">#</TableHead>
                    <TableHead>Membre</TableHead>
                    <TableHead>Part</TableHead>
                    <TableHead className="text-right">Propre</TableHead>
                    <TableHead className="text-right">Sale</TableHead>
                    <TableHead className="text-right">Total</TableHead>
                    <TableHead className="text-right">Cotis.</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data.members.map((row, index) => {
                    const sum = row.propre + row.sale
                    const share = total > 0 ? Math.round((sum / total) * 100) : 0
                    return (
                      <TableRow key={row.member.id}>
                        <TableCell className="text-ink-subtle tabular-nums">{index + 1}</TableCell>
                        <TableCell>
                          <button
                            type="button"
                            className="tap rounded-sm text-left hover:text-brand hover:underline focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none"
                            onClick={() => onMember(row.member.id)}
                            aria-label={`Voir les cotisations de ${row.member.name}`}
                          >
                            <MemberName member={row.member} linked={false} className="hover:text-brand" />
                          </button>
                        </TableCell>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <ShareBar value={share} label={`Part de ${row.member.name}`} className="w-20" />
                            <span className="text-ink-muted tabular-nums">{formatPercent(share)}</span>
                          </div>
                        </TableCell>
                        <TableCell className="text-right tabular-nums">{formatMoney(row.propre)}</TableCell>
                        <TableCell className="text-right text-internal tabular-nums">{formatMoney(row.sale)}</TableCell>
                        <TableCell className="text-right font-semibold text-staff tabular-nums">
                          {formatMoney(sum)}
                        </TableCell>
                        <TableCell className="text-right text-ink-muted tabular-nums">
                          {formatNumber(row.count)}
                        </TableCell>
                      </TableRow>
                    )
                  })}
                </TableBody>
              </Table>
            </Card>
          )}
        </Section>
      )}

      <Section
        title="Détail"
        description={`${formatNumber(data.entries.total)} cotisation(s), des plus récentes aux plus anciennes.`}
      >
        {data.entries.items.length === 0 ? (
          <EmptyState title="Aucune cotisation" icon={<Coins />} />
        ) : (
          <Card className="py-0">
            <EntriesTable entries={data.entries.items} />
          </Card>
        )}
        <Pagination
          page={data.entries.page}
          pageSize={data.entries.pageSize}
          total={data.entries.total}
          onPage={onPage}
          newerLabel="Plus récentes"
          olderLabel="Plus anciennes"
        />
      </Section>
    </>
  )
}
