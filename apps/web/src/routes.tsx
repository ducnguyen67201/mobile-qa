// Keep scaffold markers for future route insertion. Only Home currently calls the API;
// Tests, Runs and Settings are honest placeholders, not mocked product implementations.
import { createBrowserRouter, type RouteObject } from 'react-router'
import { App } from './App'
import { Home } from './pages/Home'
import { Placeholder } from './pages/Placeholder'
// scaffold:imports
export const routes: RouteObject[] = [{
  path: '/', element: <App />, children: [
    { index: true, element: <Home /> },
    { path: 'tests', element: <Placeholder title="Tests" description="Create and organize your test cases here." /> },
    { path: 'runs', element: <Placeholder title="Runs" description="Review test outcomes and evidence here." /> },
    { path: 'settings', element: <Placeholder title="Settings" description="Manage your workspace and access here." /> },
    // scaffold:routes
    { path: '*', element: <Placeholder title="Page not found" description="Choose a page from the navigation to continue." /> },
  ],
}]
export const router = createBrowserRouter(routes)
