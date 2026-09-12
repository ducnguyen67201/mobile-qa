import { useEffect } from 'react'
import {
  AppShell,
  Badge,
  Box,
  Burger,
  Container,
  Drawer,
  Group,
  Text,
  useMantineTheme,
} from '@mantine/core'
import { useDisclosure, useMediaQuery } from '@mantine/hooks'
import { Outlet, useLocation, useNavigate } from 'react-router'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ErrorNotice } from '@/components/app/feedback'
import { Navigation, navigationLinks } from '@/components/app/navigation'
import { signOut } from '@/api/setup'

export function App() {
  const [minimized, { toggle: toggleSidebar }] = useDisclosure(false)
  const [mobileOpen, { close: closeNavigation, toggle: toggleNavigation }] = useDisclosure(false)
  const { breakpoints } = useMantineTheme()
  const desktop = useMediaQuery(`(min-width: ${breakpoints.sm})`)
  // A CSS-hidden drawer would retain its focus trap and scroll lock after resizing.
  useEffect(() => {
    if (desktop) closeNavigation()
  }, [desktop, closeNavigation])
  const client = useQueryClient()
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const logout = useMutation({
    mutationFn: signOut,
    onSuccess: () => {
      client.clear()
      void navigate('/sign-in', { replace: true })
    },
  })

  return (
    <AppShell
      layout="alt"
      header={{ height: 72 }}
      navbar={{ width: minimized ? 80 : 256, breakpoint: 'sm', collapsed: { mobile: true } }}
      padding={0}
    >
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <AppShell.Navbar
        component="aside"
        p={minimized ? 'sm' : 'lg'}
        className="workspace-nav"
        style={{ overflowY: 'auto' }}
      >
        <Navigation
          close={closeNavigation}
          minimized={minimized}
          toggle={toggleSidebar}
          signOut={() => logout.mutate()}
          signingOut={logout.isPending}
        />
      </AppShell.Navbar>
      <Drawer
        opened={mobileOpen && !desktop}
        onClose={closeNavigation}
        position="left"
        size={300}
        title="Workspace"
        hiddenFrom="sm"
        styles={{ body: { display: 'flex', minHeight: 'calc(100dvh - 80px)' } }}
        classNames={{ content: 'workspace-nav', header: 'workspace-nav' }}
      >
        <Navigation
          close={closeNavigation}
          signOut={() => logout.mutate()}
          signingOut={logout.isPending}
        />
      </Drawer>
      <AppShell.Header px={{ base: 'md', sm: 'xl' }} bg="var(--mantine-color-body)">
        <Group justify="space-between" h="100%" wrap="nowrap">
          <Group gap="md" wrap="nowrap">
            <Burger
              opened={mobileOpen}
              onClick={toggleNavigation}
              hiddenFrom="sm"
              size="sm"
              aria-label="Toggle navigation"
            />
            <Text size="xs" fw={500}>
              {pathname.startsWith('/workspaces')
                ? 'Workspaces'
                : (navigationLinks.find((link) => pathname.startsWith(link.to))?.label ?? 'Apps')}
            </Text>
          </Group>
        </Group>
      </AppShell.Header>
      <AppShell.Main id="main" tabIndex={-1}>
        <Container size="xl" px={{ base: 'lg', sm: 36, lg: 48 }} py={{ base: 32, sm: 48 }}>
          {logout.isError && (
            <Box mb="xl">
              <ErrorNotice
                error={logout.error}
                title="Sign-out was not confirmed"
                retry={() => logout.mutate()}
              />
            </Box>
          )}
          <Outlet />
          <Group component="footer" justify="space-between" mt={64} gap="xs">
            <Text size="xs" c="dimmed">
              Mobile QA · Android workspace
            </Text>
            <Badge variant="transparent" color="gray">
              Evidence before execution.
            </Badge>
          </Group>
        </Container>
      </AppShell.Main>
    </AppShell>
  )
}
