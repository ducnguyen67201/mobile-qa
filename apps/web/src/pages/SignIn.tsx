import { useForm } from '@mantine/form'
import { Box, Button, Center, Divider, Group, Stack, Text, TextInput, ThemeIcon, Title } from '@mantine/core'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useLocation, useNavigate } from 'react-router'
import { ArrowRight, Layers3, ShieldCheck, Smartphone } from 'lucide-react'
import { signIn, sessionQuery } from '@/api/setup'
import { ErrorNotice } from '@/components/app/feedback'

export function safeReturnTo(value: unknown) {
  return typeof value === 'string' &&
    /^\/(apps(?:\/|\?|$)|settings(?:\?|$)|tests(?:\?|$)|runs(?:\?|$))/.test(value) &&
    !value.includes('\\')
    ? value
    : '/apps'
}
export function SignIn() {
  const form = useForm({
    initialValues: { email: '', password: '' },
    transformValues: (values) => ({ email: values.email.trim(), password: values.password }),
  })
  const client = useQueryClient()
  const navigate = useNavigate()
  const location = useLocation()
  const login = useMutation({
    mutationFn: signIn,
    onSuccess: (session) => {
      client.clear()
      client.setQueryData(sessionQuery.queryKey, session)
      form.setFieldValue('password', '')
      void navigate(safeReturnTo(location.state?.returnTo), { replace: true })
    },
  })

  return (
    <main className="sign-in">
      <Stack component="section" visibleFrom="md" p={56} justify="space-between" className="sign-in-story">
        <Group>
          <Layers3 size={32} />
          <Text size="xl" fw={600}>
            Mobile QA
          </Text>
        </Group>
        <Stack gap="xl" py={64} maw={540}>
          <Text size="xs" tt="uppercase" lts=".2em" c="var(--workspace-accent)">
            Android quality, grounded.
          </Text>
          <Text className="sign-in-tagline" fw={500}>
            Confidence starts
            <br />
            with your build.
          </Text>
          <Text c="var(--workspace-nav-muted)" maw={380}>
            One considered workspace for your app, its builds, and the evidence behind them.
          </Text>
          <Divider color="var(--workspace-nav-muted)" opacity={0.3} />
          <Group wrap="nowrap">
            <Smartphone size={40} />
            <Box>
              <Text size="sm">Your next release starts here.</Text>
              <Text size="xs" c="var(--workspace-nav-muted)" mt={4}>
                Private uploads. Real validation. Clear next steps.
              </Text>
            </Box>
          </Group>
        </Stack>
        <Text size="xs" c="var(--workspace-nav-muted)">
          Built for teams that care about the details.
        </Text>
      </Stack>
      <Center component="section" px="xl" py={64}>
        <Box w="100%" maw={380}>
          <Group hiddenFrom="md" mb={48}>
            <Layers3 />
            <Text fw={600}>Mobile QA</Text>
          </Group>
          <ThemeIcon size={48} variant="light" radius="lg" mb="lg">
            <ShieldCheck size={24} />
          </ThemeIcon>
          <Text size="xs" fw={600} tt="uppercase" lts=".15em" c="dimmed" mb="sm">
            Your workspace awaits
          </Text>
          <Title order={1} size="h2">
            Welcome back.
          </Title>
          <Text size="sm" c="dimmed" mt="sm">
            Sign in to manage your apps and builds.
          </Text>
          <form onSubmit={form.onSubmit((values) => login.mutate(values))}>
            <Stack gap="lg" mt={36}>
              <TextInput
                label="Email address"
                type="email"
                autoComplete="username"
                required
                withAsterisk={false}
                {...form.getInputProps('email')}
                placeholder="you@company.com"
                size="md"
              />
              <TextInput
                label="Password"
                type="password"
                autoComplete="current-password"
                required
                withAsterisk={false}
                {...form.getInputProps('password')}
                size="md"
              />
              {login.isError && <ErrorNotice focus error={login.error} title="Unable to sign in" />}
              <Button
                type="submit"
                size="md"
                fullWidth
                disabled={login.isPending}
                rightSection={<ArrowRight size={18} />}
              >
                {login.isPending ? 'Signing in…' : 'Sign in'}
              </Button>
            </Stack>
          </form>
          <Divider my="xl" />
          <Text size="xs" c="dimmed">
            Pilot access is provisioned by your workspace operator. Contact them if you need an account or
            password assistance.
          </Text>
        </Box>
      </Center>
    </main>
  )
}
