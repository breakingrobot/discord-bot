import { ToggleGroup as ToggleGroupPrimitive } from 'radix-ui'
import type { ComponentProps } from 'react'

import { cn } from '@/lib/utils'

/** Groupe de boutons exclusifs (périodes, vues). */
export const ToggleGroup = ({ className, ...props }: ComponentProps<typeof ToggleGroupPrimitive.Root>) => (
  <ToggleGroupPrimitive.Root
    data-slot="toggle-group"
    className={cn(
      'grid w-full grid-cols-[repeat(auto-fit,minmax(5.5rem,1fr))] items-center gap-1.5 sm:inline-flex sm:w-auto sm:flex-wrap',
      className,
    )}
    {...props}
  />
)

export const ToggleGroupItem = ({ className, ...props }: ComponentProps<typeof ToggleGroupPrimitive.Item>) => (
  <ToggleGroupPrimitive.Item
    data-slot="toggle-group-item"
    className={cn(
      'inline-flex h-8 items-center justify-center gap-1.5 rounded-md border border-border bg-card px-3 text-sm font-medium whitespace-nowrap text-ink-muted shadow-xs transition-colors outline-none hover:border-line-strong hover:text-ink focus-visible:ring-[3px] focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 data-[state=on]:border-brand data-[state=on]:bg-brand-soft data-[state=on]:text-brand [&_svg]:size-4',
      className,
    )}
    {...props}
  />
)
