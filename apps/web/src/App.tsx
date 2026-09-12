import { useEffect } from 'react'
import {
  AppShell,
  Anchor,
  Avatar,
  Badge,
  Box,
  Burger,
  Button,
  Container,
  Divider,
  Drawer,
  Group,
  Menu,
  NavLink,
  Paper,
  Stack,
  Text,
  ThemeIcon,
  useMantineTheme,
} from '@mantine/core'
import { useDisclosure, useMediaQuery } from '@mantine/hooks'
import { Link, Outlet, useLocation, useNavigate } from 'react-router'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import {
  AppWindow,
  ArrowUpRight,
  ChevronDown,
  FlaskConical,
  Layers3,
  LogOut,
  Play,
  Settings2,
  ShieldCheck,
} from 'lucide-react'
import { useSession } from '@/components/app/session'
import { ErrorNotice } from '@/components/app/feedback'
import { signOut } from '@/api/setup'

const links = [
  { to: '/apps', label: 'Apps', icon: AppWindow },
  { to: '/tests', label: 'Tests', icon: FlaskConical },
  { to: '/runs', label: 'Runs', icon: Play },
  { to: '/settings', label: 'Settings', icon: Settings2 },
]

function Navigation({ close }: { close: () => void }) {
  const { pathname } = useLocation()
  return (
    <Stack h="100%" justify="space-between" gap="xl">
      <Stack gap="xl">
        <Anchor component={Link} to="/apps" underline="never" c="inherit" onClick={close}>
          <Group gap="sm">
            <ThemeIcon color="forest.2" c="forest.9" size={36} radius="md">
              <Layers3 size={20} />
            </ThemeIcon>
            <Text fw={600} size="lg">
              Mobile QA
            </Text>
            <Text size="xs" c="var(--workspace-nav-muted)">
              Pilot
            </Text>
          </Group>
        </Anchor>
        <Box component="nav" aria-label="Workspace navigation">
          <Text size="xs" tt="uppercase" lts=".15em" c="var(--workspace-nav-muted)" mb="md" px="sm">
            Workspace
          </Text>
          <Stack gap="xs">
            {links.map(({ to, label, icon: Icon }) => (
              <NavLink
                key={to}
                component={Link}
                to={to}
                label={label}
                active={pathname.startsWith(to)}
                aria-current={pathname.startsWith(to) ? 'page' : undefined}
                leftSection={<Icon size={18} />}
                onClick={close}
                py="sm"
                rightSection={
                  (label === 'Tests' || label === 'Runs') && (
                    <Text size="xs" opacity={0.65}>
                      Soon
                    </Text>
                  )
                }
              />
            ))}
          </Stack>
        </Box>
      </Stack>
      <Stack gap="md">
        <Paper withBorder p="md" radius="lg" className="nav-note">
          <ShieldCheck size={22} color="var(--workspace-accent)" />
          <Text size="sm" fw={500} mt="sm">
            A reliable first step.
          </Text>
          <Text size="xs" c="var(--workspace-nav-muted)" mt="xs">
            Connect your app. Validate a build. Start with evidence.
          </Text>
          <Anchor
            component={Link}
            to="/settings"
            c="var(--workspace-accent)"
            size="xs"
            mt="md"
            onClick={close}
          >
            <Group gap={6}>
              Workspace details <ArrowUpRight size={12} />
            </Group>
          </Anchor>
        </Paper>
        <Text size="xs" c="var(--workspace-nav-muted)">
          ANDROID QUALITY WORKSPACE
        </Text>
      </Stack>
    </Stack>
  )
}

export function App() {
  const [mobileOpen, { close: closeNavigation, toggle: toggleNavigation }] = useDisclosure(false)
  const { breakpoints } = useMantineTheme()
  const desktop = useMediaQuery(`(min-width: ${breakpoints.sm})`)
  // A CSS-hidden drawer would retain its focus trap and scroll lock after resizing.
  useEffect(() => {
    if (desktop) closeNavigation()
  }, [desktop, closeNavigation])
  const session = useSession()
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
      navbar={{ width: 256, breakpoint: 'sm', collapsed: { mobile: true } }}
      padding={0}
    >
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <AppShell.Navbar component="aside" p="lg" className="workspace-nav" style={{ overflowY: 'auto' }}>
        <Navigation close={closeNavigation} />
      </AppShell.Navbar>
      <Drawer
        opened={mobileOpen && !desktop}
        onClose={closeNavigation}
        position="left"
        size={300}
        title="Workspace"
        hiddenFrom="sm"
        classNames={{ content: 'workspace-nav', header: 'workspace-nav' }}
      >
        <Navigation close={closeNavigation} />
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
            <Text size="xs" c="dimmed" visibleFrom="xs">
              Workspace
            </Text>
            <Divider orientation="vertical" />
            <Text size="xs" fw={500}>
              {links.find((link) => pathname.startsWith(link.to))?.label ?? 'Apps'}
            </Text>
          </Group>
          <Menu position="bottom-end" width={260}>
            <Menu.Target>
              <Button
                variant="subtle"
                color="forest"
                aria-label={`Open account menu for ${session.user.display_name}`}
                leftSection={
                  <Avatar size={28} radius="xl">
                    {session.user.display_name.slice(0, 1).toUpperCase()}
                  </Avatar>
                }
                rightSection={<ChevronDown size={14} />}
              >
                <Text size="xs" truncate maw={160} visibleFrom="sm">
                  {session.user.display_name}
                </Text>
              </Button>
            </Menu.Target>
            <Menu.Dropdown>
              <Menu.Label>
                <Text size="xs" className="identifier">
                  {session.user.email}
                </Text>
              </Menu.Label>
              <Menu.Divider />
              <Menu.Item
                leftSection={<LogOut size={16} />}
                onClick={() => logout.mutate()}
                disabled={logout.isPending}
              >
                {logout.isPending ? 'Signing out…' : 'Sign out'}
              </Menu.Item>
            </Menu.Dropdown>
          </Menu>
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
