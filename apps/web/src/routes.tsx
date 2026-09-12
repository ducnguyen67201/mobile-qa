// Preserve scaffold markers. All tenant screens mount only after session validation.
import { createBrowserRouter, Navigate, type RouteObject } from 'react-router'
import { App } from './App'
import { Apps } from './pages/Apps'
import { AppDetail } from './pages/AppDetail'
import { SignIn } from './pages/SignIn'
import { Settings } from './pages/Settings'
import { Protected } from './components/app/session'
import { Placeholder } from './pages/Placeholder'
// scaffold:imports
export const routes: RouteObject[] = [
  { path: '/sign-in', element: <SignIn /> },
  { path: '/', element: <Protected><App /></Protected>, children: [
    { index: true, element: <Navigate to="/apps" replace /> },
    { path: 'apps', element: <Apps /> },
    { path: 'apps/:app_id', element: <AppDetail /> },
    { path: 'tests', element: <Placeholder title="Tests" description="Create and organize your test cases here." /> },
    { path: 'runs', element: <Placeholder title="Runs" description="Review test outcomes and evidence here." /> },
    { path: 'settings', element: <Settings /> },
    // scaffold:routes
    { path: '*', element: <Placeholder title="Page not found" description="Choose a page from the navigation to continue." /> },
  ] },
]
export const router = createBrowserRouter(routes)
