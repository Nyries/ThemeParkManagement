import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/react";
import type { MapResponse } from "@app/shared-types/grpc";
import type { ToolState } from "@/park/tool";

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
  mockOnWorldState: vi.fn((callback: (state: unknown) => void) => {
    void callback;
    return () => {};
  }),
  mockSendCommand: vi.fn(),
}));

vi.mock("../../hooks/useParkSocket", () => ({
  useParkSocket: () => ({
    sendCommand: mockSendCommand,
    onWorldState: mockOnWorldState,
    onMap: mockOnMap,
    isConnected: true,
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

const NO_TOOL: ToolState = { mode: null };

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

    render(<Park tool={NO_TOOL} onToolChange={vi.fn()} />);

    expect(mockOnMap).toHaveBeenCalledTimes(1);
  });

  it("initializes the PixiJS application with dimensions derived from the received map", async () => {
    mockOnMap.mockResolvedValue(emptyMap);

    render(<Park tool={NO_TOOL} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    expect(mockAppInit).toHaveBeenCalledWith(
      expect.objectContaining({
        width: expect.any(Number),
        height: expect.any(Number),
      }),
    );
  });

  it("subscribes to world state updates once the map has loaded", async () => {
    mockOnMap.mockResolvedValue(emptyMap);

    render(<Park tool={NO_TOOL} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    expect(mockOnWorldState).toHaveBeenCalledTimes(1);
  });

  it("updates the cell's rendering after successfully placing infrastructure via a click", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 0, maxY: 0 }); // 1x1 map, une seule case vide
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    render(
      <Park tool={{ mode: "infrastructure" }} onToolChange={vi.fn()} />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    const cell = mockStageAddChild.mock.calls[0][0];
    const handleCellClick = cell.on.mock.calls[0][1];

    await handleCellClick();

    expect(mockSendCommand).toHaveBeenCalledWith(
      expect.objectContaining({ parkId: "default" }),
    );
    expect(cell.clear).toHaveBeenCalled();
  });

  it("renders a visitor sprite when a world state update is received", async () => {
    mockOnMap.mockResolvedValue(emptyMap);

    render(<Park tool={NO_TOOL} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    const onWorldStateCallback = mockOnWorldState.mock.calls[0]![0]!;
    const addChildCallsBefore = mockStageAddChild.mock.calls.length;

    onWorldStateCallback({
      tickCount: 1,
      dirtyChunksJson: "{}",
      visitors: [{ id: "v1", x: 0, y: 0, z: 0 }],
    });

    expect(mockStageAddChild.mock.calls.length).toBeGreaterThan(
      addChildCallsBefore,
    );
  });
});
