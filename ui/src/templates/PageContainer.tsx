import type { ReactNode } from 'react'

export type PageContainerProps = { children: ReactNode; wide?: boolean }

export const PageContainer = ({ children, wide = false }: PageContainerProps) => (
  <main
    id="contenu"
    className={wide ? 'mx-auto w-full max-w-7xl py-6 gutter' : 'mx-auto w-full max-w-5xl py-6 gutter sm:py-8'}
  >
    {children}
  </main>
)
