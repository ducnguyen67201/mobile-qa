import type { ModelCapability, ResolvedModel } from '@/api/generated/types.gen'

export function hasModelCapability(
  model: ResolvedModel | null | undefined,
  capability: ModelCapability,
) {
  return !!model?.capabilities.includes(capability)
}
