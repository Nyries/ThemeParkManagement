import { describe, it, expect } from "vitest";
import { everyNthTickFor, shouldRelay } from "../relayThrottle";

describe("everyNthTickFor", () => {
  it("returns 1 (full rate) for a small population", () => {
    expect(everyNthTickFor(0)).toBe(1);
    expect(everyNthTickFor(100)).toBe(1);
  });

  it("returns 2 just above the first threshold", () => {
    expect(everyNthTickFor(101)).toBe(2);
    expect(everyNthTickFor(1000)).toBe(2);
  });

  it("returns 5 just above the second threshold", () => {
    expect(everyNthTickFor(1001)).toBe(5);
    expect(everyNthTickFor(5000)).toBe(5);
  });

  it("returns 10 for a very large population, with no upper bound", () => {
    expect(everyNthTickFor(5001)).toBe(10);
    expect(everyNthTickFor(1_000_000)).toBe(10);
  });
});

describe("shouldRelay", () => {
  it("relays every tick at full rate (small population)", () => {
    expect(shouldRelay(0, 10)).toBe(true);
    expect(shouldRelay(1, 10)).toBe(true);
    expect(shouldRelay(2, 10)).toBe(true);
  });

  it("only relays every other tick once past the first threshold", () => {
    expect(shouldRelay(0, 500)).toBe(true);
    expect(shouldRelay(1, 500)).toBe(false);
    expect(shouldRelay(2, 500)).toBe(true);
  });

  it("only relays every 10th tick for a very large population", () => {
    expect(shouldRelay(0, 1_000_000)).toBe(true);
    expect(shouldRelay(5, 1_000_000)).toBe(false);
    expect(shouldRelay(10, 1_000_000)).toBe(true);
  });
});
