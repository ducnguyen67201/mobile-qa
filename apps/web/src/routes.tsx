// Preserve scaffold markers. All tenant screens mount only after session validation.
import { createBrowserRouter, type RouteObject } from 'react-router'
import { App } from './App'
import { Apps } from './pages/Apps'
import { AppDetail } from './pages/AppDetail'
import { SignIn } from './pages/SignIn'
import { Settings } from './pages/Settings'
import { Protected } from './components/app/session'
import { Placeholder } from './pages/Placeholder'
import { WorkspaceGate, WorkspaceIndex } from './components/app/workspace'
import { Workspaces, CreateWorkspace } from './pages/Workspaces'
import { Runs } from './pages/Runs'
import { RunDetail } from './pages/RunDetail'
import { Tests } from './pages/Tests'
import { TestLibraryDetail } from './pages/TestLibraryDetail'
import { TaskSession } from './pages/TaskSession'
// scaffold:imports
export const routes: RouteObject[] = [
  { path: '/sign-in', element: <SignIn /> },
  {
    path: '/',
    element: (
      <Protected>
        <App />
      </Protected>
    ),
    children: [
      { index: true, element: <WorkspaceIndex /> },
      { path: 'workspaces', element: <Workspaces /> },
      { path: 'workspaces/new', element: <CreateWorkspace /> },
      {
        element: <WorkspaceGate />,
        children: [
          { path: 'apps', element: <Apps /> },
          { path: 'apps/:app_id', element: <AppDetail /> },
          { path: 'apps/:app_id/try', element: <TaskSession /> },
          { path: 'tests', element: <Tests /> },
          { path: 'tests/:app_id/:entry_id', element: <TestLibraryDetail /> },
          { path: 'tests/:app_id/:entry_id/versions/:version_id', element: <TestLibraryDetail /> },
          { path: 'runs', element: <Runs /> },
          { path: 'runs/:run_id', element: <RunDetail /> },
          { path: 'settings', element: <Settings /> },
        ],
      },
      // scaffold:routes
      {
        path: '*',
        element: (
          <Placeholder
            title="Page not found"
            description="Choose a page from the navigation to continue."
          />
        ),
      },
    ],
  },
]
export const router = createBrowserRouter(routes)
