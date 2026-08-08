import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/react";
import type { MapResponse } from "@app/shared-types/grpc";

const { mockAppInit, mockAppDestroy, mockStageAddChild } = vi.hoisted(() => ({
  mockAppInit: vi.fn(),
  mockAppDestroy: vi.fn(),
  mockStageAddChild: vi.fn(),
}));

vi.mock("pixi.js", () => {
  class MockGraphics {
    rect = vi.fn().mockReturnThis();
    fill = vi.fn().mockReturnThis();
    stroke = vi.fn().mockReturnThis();
    circle = vi.fn().mockReturnThis();
    clear = vi.fn().mockReturnThis();
    destroy = vi.fn();
    on = vi.fn();
  }

  class MockText {}

  class MockApplication {
    init = mockAppInit;
    destroy = mockAppDestroy;
    canvas = document.createElement("canvas");
    stage = { addChild: mockStageAddChild, removeChild: vi.fn() };
  }

  return {
    Application: MockApplication,
    Graphics: MockGraphics,
    Text: MockText,
  };
});


const { mockOnMap, mockOnWorldState, mockSendCommand } = vi.hoisted(() => ({
  mockOnMap: vi.fn(),
  mockOnWorldState: vi.fn(() => () => {}),
  mockSendCommand: vi.fn(),
}));

vi.mock("../../hooks/useParkSocket", () => ({
  useParkSocket: () => ({
    sendCommand: mockSendCommand,
    onWorldState: mockOnWorldState,
    onMap: mockOnMap,
  }),
}));

import { Park } from "../Park";

function flushMicrotasks() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

const emptyMap: MapResponse = {
  minX: 0,
  maxX: 4,
  minY: 0,
  maxY: 3,
  terrain: [],
  infrastructure: [],
  entrance: undefined,
};

describe("Park", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAppInit.mockResolvedValue(undefined);
  });

  afterEach(() => {
    cleanup();
  });

  it("fetches the map once on mount", () => {
    mockOnMap.mockReturnValue(new Promise(() => {})); // never resolves: only checking the call happens

    render(<Park />);

    expect(mockOnMap).toHaveBeenCalledTimes(1);
  });

  it("initializes the PixiJS application with dimensions derived from the received map", async () => {
    mockOnMap.mockResolvedValue(emptyMap);

    render(<Park />);
    await flushMicrotasks();
    await flushMicrotasks();

    expect(mockAppInit).toHaveBeenCalledWith(
      expect.objectContaining({ width: expect.any(Number), height: expect.any(Number) }),
    );
  });

  it("subscribes to world state updates once the map has loaded", async () => {
    mockOnMap.mockResolvedValue(emptyMap);

    render(<Park />);
    await flushMicrotasks();
    await flushMicrotasks();

    expect(mockOnWorldState).toHaveBeenCalledTimes(1);
  });
});
