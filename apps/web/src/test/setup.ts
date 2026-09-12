// DOM-only test cleanup; these tests do not replace manual browser acceptance.
import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach } from 'vitest'
afterEach(cleanup)
// JSDOM has no layout engine; this deterministic desktop media query is a DOM fixture.
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener() {},
    removeEventListener() {},
    addListener() {},
    removeListener() {},
    dispatchEvent: () => false,
  }),
})
// Vitest uses Node's Request/FormData. Match their Blob/File brands so multipart
// tests exercise actual fetch serialization rather than JSDOM's incompatible File.
// This affects synthetic test objects only; production uses browser-native globals.
import { Blob as FetchBlob, File as FetchFile } from 'node:buffer'
Object.defineProperty(globalThis, 'Blob', { configurable: true, writable: true, value: FetchBlob })
Object.defineProperty(globalThis, 'File', { configurable: true, writable: true, value: FetchFile })
const nativeFormData = await new Response('', {
  headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
}).formData()
Object.defineProperty(globalThis, 'FormData', {
  configurable: true,
  writable: true,
  value: nativeFormData.constructor,
})

// Mantine measures scroll containers with ResizeObserver; JSDOM has no layout.
class TestResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
Object.defineProperty(globalThis, 'ResizeObserver', {
  configurable: true,
  writable: true,
  value: TestResizeObserver,
})

// Font and scroll measurements are also layout-only in Mantine's DOM test guide.
Object.defineProperty(document, 'fonts', {
  configurable: true,
  value: { addEventListener() {}, removeEventListener() {} },
})
window.HTMLElement.prototype.scrollIntoView = () => {}
