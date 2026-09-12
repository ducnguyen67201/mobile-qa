import {
  Alert,
  Box,
  Button,
  Center,
  Divider,
  Group,
  Loader,
  Stack,
  Text,
  ThemeIcon,
  Title,
} from '@mantine/core'
import { GoogleLogin, GoogleOAuthProvider } from '@react-oauth/google'
import { Layers3, ShieldCheck, Smartphone } from 'lucide-react'
import { ErrorNotice } from '@/components/app/feedback'
import { useGoogleSignIn } from '@/hooks/use-google-sign-in'
export { safeReturnTo } from '@/lib/navigation'

export function SignIn() {
  const { challenge, login, providerError, setProviderError, acceptCredential, retry } = useGoogleSignIn()
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
            Use your Google account to manage your apps and builds.
          </Text>
          <Stack gap="lg" mt={36}>
            {challenge.isPending || challenge.isFetching ? (
              <Group role="status">
                <Loader size="sm" />
                <Text size="sm">Preparing Google sign-in…</Text>
              </Group>
            ) : challenge.isError ? (
              <ErrorNotice error={challenge.error} title="Google sign-in is unavailable" retry={retry} />
            ) : login.isPending ? (
              <Group role="status">
                <Loader size="sm" />
                <Text size="sm">Signing in…</Text>
              </Group>
            ) : login.isError ? (
              <ErrorNotice focus error={login.error} title="Unable to sign in" retry={retry} />
            ) : providerError ? (
              <Alert color="red" title="Unable to sign in">
                <Text size="sm">{providerError}</Text>
                <Button variant="outline" mt="sm" onClick={retry}>
                  Try again
                </Button>
              </Alert>
            ) : (
              challenge.data && (
                <GoogleOAuthProvider
                  key={challenge.data.challenge_id}
                  clientId={challenge.data.client_id}
                  onScriptLoadError={() =>
                    setProviderError('Google sign-in could not load. Check your connection and try again.')
                  }
                >
                  <GoogleLogin
                    nonce={challenge.data.nonce}
                    text="continue_with"
                    size="large"
                    theme="outline"
                    onSuccess={(response) => acceptCredential(response.credential)}
                    onError={() =>
                      setProviderError('Google sign-in was canceled or could not complete. Please try again.')
                    }
                  />
                </GoogleOAuthProvider>
              )
            )}
          </Stack>
          <Divider my="xl" />
          <Text size="xs" c="dimmed">
            Google sign-in is the only sign-in method. Your workspace operator manages access; use the Google
            account they invited.
          </Text>
        </Box>
      </Center>
    </main>
  )
}
