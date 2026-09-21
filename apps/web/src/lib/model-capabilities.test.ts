import { describe, expect, it } from 'vitest'
import { hasModelCapability } from './model-capabilities'

const model = {
  reference: { key: 'synthetic.openai', revision: 1 },
  display_name: 'Synthetic',
  provider: 'open_ai' as const,
  provider_model: 'synthetic-model',
  capabilities: ['minitap_navigation' as const],
}

describe('hasModelCapability', () => {
  it('distinguishes navigation, authoring, and model-free contexts', () => {
    expect(hasModelCapability(model, 'minitap_navigation')).toBe(true)
    expect(hasModelCapability(model, 'structured_authoring')).toBe(false)
    expect(hasModelCapability(undefined, 'minitap_navigation')).toBe(false)
  })
})
