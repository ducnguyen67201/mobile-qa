import {
  ActionIcon,
  Anchor,
  Box,
  Group,
  NavLink,
  Divider,
  Stack,
  Text,
  ThemeIcon,
  Tooltip,
} from '@mantine/core'
import { Link, useLocation } from 'react-router'
import {
  AppWindow,
  FlaskConical,
  Layers3,
  PanelLeftClose,
  PanelLeftOpen,
  Play,
  Settings2,
} from 'lucide-react'
import { useWorkspace } from '@/hooks/use-workspace'
import { WorkspaceSwitcher } from './workspace'
import { AccountMenu } from './account-menu'

const settingsLink = { to: '/settings', label: 'Settings', icon: Settings2 }

export const navigationLinks = [
  { to: '/apps', label: 'Apps', icon: AppWindow },
  { to: '/tests', label: 'Tests', icon: FlaskConical },
  { to: '/runs', label: 'Runs', icon: Play },
  settingsLink,
]

export function Navigation({
  close,
  minimized = false,
  toggle,
  signOut,
  signingOut,
}: {
  close: () => void
  minimized?: boolean
  toggle?: () => void
  signOut: () => void
  signingOut: boolean
}) {
  const { href } = useWorkspace()
  return (
    <Stack h="100%" flex={1} justify="space-between" gap="xl">
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
            {navigationLinks
              .filter((link) => link.to !== '/settings')
              .map((link) => (
                <NavigationItem key={link.to} link={link} minimized={minimized} close={close} />
              ))}
          </Stack>
        </Box>
      </Stack>
      <Stack gap="md">
        <Box component="nav" aria-label="Account navigation">
          <NavigationItem link={settingsLink} minimized={minimized} close={close} />
        </Box>
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
        <Divider color="#ffffff20" />
        <AccountMenu minimized={minimized} signOut={signOut} signingOut={signingOut} />
      </Stack>
    </Stack>
  )
}

function NavigationItem({
  link: { to, label, icon: Icon },
  minimized,
  close,
}: {
  link: (typeof navigationLinks)[number]
  minimized: boolean
  close: () => void
}) {
  const { pathname } = useLocation()
  const { href } = useWorkspace()
  const active = pathname.startsWith(to)
  if (minimized)
    return (
      <Tooltip label={label} position="right" withArrow>
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
          mx="auto"
          display="flex"
        >
          <Icon size={20} />
        </ActionIcon>
      </Tooltip>
    )
  return (
    <NavLink
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
}
