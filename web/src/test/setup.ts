import '@testing-library/jest-dom/vitest'

// Radix primitives (Switch, Popper) measure their nodes with a ResizeObserver,
// which jsdom does not implement. Nothing under test reads the sizes back, so a
// no-op stand-in is enough to let them mount.
if (!globalThis.ResizeObserver) {
  globalThis.ResizeObserver = class ResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
}
