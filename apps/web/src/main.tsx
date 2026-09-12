// Browser-only entry point; the Rust API owns server behavior and runtime secrets.
import React from 'react'
import { MantineProvider } from '@mantine/core'
import { theme, cssVariablesResolver } from './theme'
import ReactDOM from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from 'react-router'
import { router } from './routes'
import './index.css'
const root = document.getElementById('root')
if (!root) throw new Error('No root element found')
// Avoid hidden retries/focus refreshes in the foundation; the health page exposes Retry.
const client = new QueryClient({ defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false } } })
ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <MantineProvider theme={theme} cssVariablesResolver={cssVariablesResolver} forceColorScheme="light">
      <QueryClientProvider client={client}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </MantineProvider>
  </React.StrictMode>,
)
