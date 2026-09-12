import {
  Badge,
  Button,
  Card,
  createTheme,
  Drawer,
  Text,
  type CSSVariablesResolver,
} from '@mantine/core'

// Product colors and component defaults live here; Mantine owns primitive behavior.
export const theme = createTheme({
  primaryColor: 'forest',
  primaryShade: 7,
  colors: {
    forest: [
      '#f0f7ee',
      '#e0eddc',
      '#c3dbba',
      '#a4c899',
      '#80b076',
      '#639658',
      '#437c46',
      '#285d48',
      '#204b3a',
      '#173b30',
    ],
  },
  fontFamily: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  headings: { fontWeight: '600' },
  defaultRadius: 'md',
  respectReducedMotion: true,
  components: {
    Button: Button.extend({ defaultProps: { size: 'sm' } }),
    Badge: Badge.extend({ defaultProps: { variant: 'light', tt: 'none' } }),
    Card: Card.extend({ defaultProps: { withBorder: true, padding: 'lg', radius: 'lg' } }),
    Drawer: Drawer.extend({
      defaultProps: {
        position: 'right',
        size: 'lg',
        padding: 'xl',
        closeButtonProps: { 'aria-label': 'Close panel' },
      },
    }),
    Text: Text.extend({ styles: { root: { overflowWrap: 'anywhere' } } }),
  },
})

export const cssVariablesResolver: CSSVariablesResolver = () => ({
  variables: {
    '--workspace-nav': '#173b30',
    '--workspace-nav-text': '#e0e9dc',
    '--workspace-nav-muted': '#adc2b5',
    '--workspace-accent': '#d4e8b9',
  },
  light: {
    '--mantine-color-body': '#f7f8f3',
    '--mantine-color-text': '#203c32',
    '--mantine-color-dimmed': '#68756a',
    '--mantine-color-default-border': '#dce3d8',
  },
  dark: {},
})
