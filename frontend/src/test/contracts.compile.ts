// Compiled with the application: generated probe preserves string precision and optionality.
import type { ContractProbe } from '../bindings/ContractProbe'
export const probe: ContractProbe = { run_id: '018f1f12-7ba0-7000-8000-000000000001', observed_at: '2026-09-12T00:00:00Z', scenario: { kind: 'pass' }, nullable_note: null, counter: '9007199254740993' }
// @ts-expect-error Precision-sensitive counters must never become a JS number.
const invalidCounter: ContractProbe['counter'] = 42
void invalidCounter
