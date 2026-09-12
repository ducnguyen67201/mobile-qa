// DOM-only test cleanup; these tests do not replace manual browser acceptance.
import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach } from 'vitest'
afterEach(cleanup)
