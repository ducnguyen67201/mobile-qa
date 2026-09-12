import { NavLink, Outlet } from 'react-router'
export function App() {
  return <div className="min-h-screen">
    <a className="skip-link" href="#main">Skip to content</a>
    <header className="border-b border-border bg-white">
      <div className="mx-auto flex max-w-6xl flex-wrap items-center gap-x-12 gap-y-4 px-6 py-5 sm:px-10">
        <NavLink to="/" className="flex items-center gap-3 text-lg font-semibold tracking-tight" aria-label="Mobile QA home">
          <span aria-hidden="true" className="flex size-8 items-center justify-center rounded-lg bg-foreground text-sm text-white">M</span>Mobile QA
        </NavLink>
        <nav aria-label="Main navigation" className="flex gap-1">
          {([['/', 'App'], ['/tests', 'Tests'], ['/runs', 'Runs'], ['/settings', 'Settings']] as const).map(([to, label]) => <NavLink key={to} to={to} end={to === '/'} className={({ isActive }) => `rounded-md px-3 py-2 text-sm font-medium ${isActive ? 'bg-muted text-foreground' : 'text-muted-foreground hover:bg-muted'}`}>{label}</NavLink>)}
        </nav>
      </div>
    </header>
    <main id="main" tabIndex={-1} className="mx-auto max-w-6xl px-6 py-12 sm:px-10"><Outlet /></main>
    <footer className="mx-auto max-w-6xl px-6 pb-8 text-xs text-muted-foreground sm:px-10">Mobile QA · Local workspace</footer>
  </div>
}
