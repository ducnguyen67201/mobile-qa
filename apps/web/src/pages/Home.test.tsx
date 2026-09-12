import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createMemoryRouter, RouterProvider } from 'react-router'
import { afterEach, expect, it, vi } from 'vitest'
import { routes } from '../routes'
function show(path = '/') {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 } } })
  return render(<QueryClientProvider client={client}><RouterProvider router={createMemoryRouter(routes, { initialEntries: [path] })} /></QueryClientProvider>)
}
afterEach(() => vi.unstubAllGlobals())
it('shows loading then ready', async () => {
  let resolve: ((response: Response) => void) | undefined
  vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(done => { resolve = done })))
  show()
  expect(screen.getByRole('status')).toHaveTextContent('Connecting…')
  await waitFor(() => expect(fetch).toHaveBeenCalledOnce())
  resolve?.(Response.json({ status: 'ok', service: 'mobile-qa', version: '0.1.0' }))
  expect(await screen.findByText('Ready')).toBeInTheDocument()
})
it('retries an unavailable service', async () => {
  vi.stubGlobal('fetch', vi.fn().mockRejectedValueOnce(new Error('offline')).mockResolvedValueOnce(Response.json({ status: 'ok', service: 'mobile-qa', version: '0.1.0' })))
  show()
  await userEvent.click(await screen.findByRole('button', { name: 'Retry' }))
  expect(await screen.findByText('Ready')).toBeInTheDocument()
})
it('navigates placeholders without requesting health', async () => {
  const fetchMock = vi.fn(); vi.stubGlobal('fetch', fetchMock)
  show('/tests')
  expect(screen.getByRole('heading', { name: 'Tests', level: 1 })).toBeInTheDocument()
  await userEvent.click(screen.getByRole('link', { name: 'Runs' }))
  expect(screen.getByRole('heading', { name: 'Runs', level: 1 })).toBeInTheDocument()
  expect(fetchMock).not.toHaveBeenCalled()
})
