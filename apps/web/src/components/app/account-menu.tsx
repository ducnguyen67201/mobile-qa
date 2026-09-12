import { Avatar, Box, Group, Menu, Text, Tooltip, UnstyledButton } from '@mantine/core'
import { ChevronUp, LogOut } from 'lucide-react'
import { useSession } from './session'

/** One account menu serves the expanded sidebar, icon rail and mobile drawer. */
export function AccountMenu({
  minimized,
  signOut,
  signingOut,
}: {
  minimized: boolean
  signOut: () => void
  signingOut: boolean
}) {
  const { user } = useSession()
  return (
    <Menu position={minimized ? 'right-end' : 'top-start'} width={260}>
      <Menu.Target>
        <Tooltip label={user.display_name} disabled={!minimized} position="right" withArrow>
          <UnstyledButton
            aria-label={`Open account menu for ${user.display_name}`}
            className="nav-account"
            w="100%"
            p={minimized ? 6 : 8}
          >
            <Group gap="sm" wrap="nowrap" justify={minimized ? 'center' : undefined}>
              <Avatar size={32} radius="xl" color="forest.2" c="forest.9">
                {user.display_name.slice(0, 1).toUpperCase()}
              </Avatar>
              {!minimized && (
                <>
                  <Box flex={1} miw={0}>
                    <Text size="sm" fw={500} truncate>
                      {user.display_name}
                    </Text>
                    <Text size="xs" c="var(--workspace-nav-muted)" truncate>
                      {user.email}
                    </Text>
                  </Box>
                  <ChevronUp size={14} />
                </>
              )}
            </Group>
          </UnstyledButton>
        </Tooltip>
      </Menu.Target>
      <Menu.Dropdown>
        <Menu.Label>
          <Text size="xs" className="identifier">
            {user.email}
          </Text>
        </Menu.Label>
        <Menu.Divider />
        <Menu.Item leftSection={<LogOut size={16} />} onClick={signOut} disabled={signingOut}>
          {signingOut ? 'Signing out…' : 'Sign out'}
        </Menu.Item>
      </Menu.Dropdown>
    </Menu>
  )
}
