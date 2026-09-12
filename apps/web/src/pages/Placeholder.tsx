// Specs 04–06 own test execution and reports; navigation does not imply availability.
import { Button, Card, Stack, Text, Title } from '@mantine/core'
import { FlaskConical } from 'lucide-react'
import { Link } from 'react-router'
import { PageHeading } from '@/components/app/feedback'

export function Placeholder({ title, description }: { title: string; description: string }) {
  return (
    <>
      <PageHeading eyebrow="Up next" title={title} description={description} />
      <Card maw={680} p={{ base: 'lg', sm: 40 }}>
        <Stack gap="lg" align="flex-start">
          <FlaskConical size={32} strokeWidth={1.5} />
          <Title order={2} size="h3">
            One step at a time.
          </Title>
          <Text size="sm" c="dimmed">
            This part of the workspace is planned for a later phase. Start by setting up your app
            and validating an APK.
          </Text>
          <Button component={Link} to="/apps" variant="outline">
            Go to apps
          </Button>
        </Stack>
      </Card>
    </>
  )
}
