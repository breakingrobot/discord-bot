import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { Pagination } from '@/molecules/Pagination'

describe('Pagination', () => {
  it('ne s’affiche pas pour une seule page', () => {
    const { container } = render(
      <Pagination page={1} pageSize={25} total={10} onPage={vi.fn<(page: number) => void>()} />,
    )
    expect(container).toBeEmptyDOMElement()
  })

  it('indique la page et désactive les extrémités', () => {
    render(<Pagination page={1} pageSize={25} total={60} onPage={vi.fn<(page: number) => void>()} />)
    expect(screen.getByText('Page 1 sur 3')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Précédent' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Suivant' })).toBeEnabled()
  })

  it('change de page avec les libellés fournis', async () => {
    const onPage = vi.fn<(page: number) => void>()
    render(
      <Pagination
        page={2}
        pageSize={25}
        total={75}
        onPage={onPage}
        newerLabel="Plus récentes"
        olderLabel="Plus anciennes"
      />,
    )
    await userEvent.click(screen.getByRole('button', { name: 'Plus anciennes' }))
    await userEvent.click(screen.getByRole('button', { name: 'Plus récentes' }))
    expect(onPage).toHaveBeenNthCalledWith(1, 3)
    expect(onPage).toHaveBeenNthCalledWith(2, 1)
  })
})
