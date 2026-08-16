import "@testing-library/jest-dom/vitest";

// jsdom doesn't implement ResizeObserver, but Radix UI's Popper positioning
// (used by Tooltip, Select, etc.) relies on it — stub it so tests that mount
// those components don't crash.
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver ??= ResizeObserverStub;
