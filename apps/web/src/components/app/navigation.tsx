import {
  ActionIcon,
  Anchor,
  Box,
  Group,
  NavLink,
  Paper,
  Stack,
  Text,
  ThemeIcon,
  Tooltip,
} from '@mantine/core'
import { Link, useLocation } from 'react-router'
import {
  AppWindow,
  ArrowUpRight,
  FlaskConical,
  Layers3,
  PanelLeftClose,
  PanelLeftOpen,
  Play,
  Settings2,
  ShieldCheck,
} from 'lucide-react'
import { useWorkspace } from '@/hooks/use-workspace'
import { WorkspaceSwitcher } from './workspace'

export const navigationLinks = [
  { to: '/apps', label: 'Apps', icon: AppWindow },
  { to: '/tests', label: 'Tests', icon: FlaskConical },
  { to: '/runs', label: 'Runs', icon: Play },
  { to: '/settings', label: 'Settings', icon: Settings2 },
]

export function Navigation({
  close,
  minimized = false,
  toggle,
}: {
  close: () => void
  minimized?: boolean
  toggle?: () => void
}) {
  const { pathname } = useLocation()
  const { href } = useWorkspace()
  return (
    <Stack h="100%" justify="space-between" gap="xl">
      <Stack gap="xl">
        <Anchor
          component={Link}
          to={href('/apps')}
          underline="never"
          c="inherit"
          onClick={close}
          aria-label="Mobile QA"
        >
          <Group gap="sm" justify={minimized ? 'center' : undefined} wrap="nowrap">
            <ThemeIcon color="forest.2" c="forest.9" size={36} radius="md">
              <Layers3 size={20} />
            </ThemeIcon>
            {!minimized && (
              <>
                <Text fw={600} size="lg">
                  Mobile QA
                </Text>
                <Text size="xs" c="var(--workspace-nav-muted)">
                  Pilot
                </Text>
              </>
            )}
          </Group>
        </Anchor>
        <WorkspaceSwitcher minimized={minimized} close={close} />
        <Box component="nav" aria-label="Workspace navigation">
          {!minimized && (
            <Text
              size="xs"
              tt="uppercase"
              lts=".15em"
              c="var(--workspace-nav-muted)"
              mb="md"
              px="sm"
            >
              Workspace
            </Text>
          )}
          <Stack gap="xs" align={minimized ? 'center' : undefined}>
            {navigationLinks.map(({ to, label, icon: Icon }) => {
              const active = pathname.startsWith(to)
              return minimized ? (
                <Tooltip key={to} label={label} position="right" withArrow>
                  <ActionIcon
                    component={Link}
                    to={href(to)}
                    aria-label={label}
                    aria-current={active ? 'page' : undefined}
                    data-active={active || undefined}
                    className="nav-icon"
                    variant="subtle"
                    size={44}
                    onClick={close}
                  >
                    <Icon size={20} />
                  </ActionIcon>
                </Tooltip>
              ) : (
                <NavLink
                  key={to}
                  component={Link}
                  to={href(to)}
                  label={label}
                  active={active}
                  aria-current={active ? 'page' : undefined}
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
              )
            })}
          </Stack>
        </Box>
      </Stack>
      <Stack gap="md">
        {!minimized && (
          <>
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
                to={href('/settings')}
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
          </>
        )}
        {toggle && (
          <Tooltip
            label={minimized ? 'Expand sidebar' : 'Collapse sidebar'}
            position="right"
            withArrow
          >
            <ActionIcon
              onClick={toggle}
              aria-label={minimized ? 'Expand sidebar' : 'Collapse sidebar'}
              aria-expanded={!minimized}
              variant="subtle"
              className="nav-icon"
              size={44}
              mx={minimized ? 'auto' : undefined}
            >
              {minimized ? <PanelLeftOpen size={20} /> : <PanelLeftClose size={20} />}
            </ActionIcon>
          </Tooltip>
        )}
      </Stack>
    </Stack>
  )
}
