import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, cleanup, fireEvent, act } from "@testing-library/react";
import type { MapResponse } from "@app/shared-types/grpc";
import type { ToolState } from "@/types/park/tool";
import { Rotation } from "@app/shared-types";
import { toast } from "sonner";

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

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

  // Park.tsx adds all visual content (cells/labels/ghost/visitors) to a
  // camera container, itself the stage's only child — mockStageAddChild
  // tracks the container's addChild calls so every existing index-based
  // assertion in this file (mockStageAddChild.mock.calls[0][0] = first
  // cell, etc.) keeps working unchanged. The stage's own addChild (which
  // only ever receives that one container) stays untracked.
  class MockContainer {
    addChild = mockStageAddChild;
    removeChild = vi.fn();
  }

  class MockApplication {
    init = mockAppInit;
    destroy = mockAppDestroy;
    canvas = document.createElement("canvas");
    stage = { addChild: vi.fn(), removeChild: vi.fn() };
  }

  return {
    Application: MockApplication,
    Container: MockContainer,
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

vi.mock("../../../hooks/park/useParkSocket", () => ({
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
  building: [],
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

  it("renders the inspector panel alongside the canvas", () => {
    mockOnMap.mockReturnValue(new Promise(() => {})); // never resolves: only checking the initial render

    const { getByText } = render(<Park tool={NO_TOOL} onToolChange={vi.fn()} />);

    expect(getByText("Aucune sélection")).toBeInTheDocument();
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

    // One subscription feeds the top bar (balance/visitor count) from mount,
    // the other syncs visitor sprites once the map has loaded.
    expect(mockOnWorldState).toHaveBeenCalledTimes(2);
  });

  it("shows a default cursor on cells when no tool is active", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 0 }); // 2x1 map

    render(<Park tool={NO_TOOL} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    for (const [cell] of mockStageAddChild.mock.calls.slice(0, 2)) {
      expect(cell.cursor).toBe("default");
    }
  });

  it("shows a pointer cursor on cells while a tool is active, and updates it when the tool changes", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 0 }); // 2x1 map

    const { rerender } = render(
      <Park tool={{ mode: "terrain" }} onToolChange={vi.fn()} />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    for (const [cell] of mockStageAddChild.mock.calls.slice(0, 2)) {
      expect(cell.cursor).toBe("pointer");
    }

    rerender(<Park tool={NO_TOOL} onToolChange={vi.fn()} />);

    for (const [cell] of mockStageAddChild.mock.calls.slice(0, 2)) {
      expect(cell.cursor).toBe("default");
    }
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

    // Infrastructure placement is driven by pointerdown (drag-tracing), not
    // pointertap: index 0 is pointertap, 1 is pointerover, 2 is pointerdown.
    const cell = mockStageAddChild.mock.calls[0][0];
    const handlePointerDown = cell.on.mock.calls[2][1];

    await handlePointerDown();

    expect(mockSendCommand).toHaveBeenCalledWith(
      expect.objectContaining({ parkId: "default" }),
    );
    expect(cell.clear).toHaveBeenCalled();
  });

  it("traces a drag across multiple cells, placing infrastructure on each new one entered", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 0 }); // 2x1 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    render(<Park tool={{ mode: "infrastructure" }} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    const [firstCell] = mockStageAddChild.mock.calls[0];
    const [secondCell] = mockStageAddChild.mock.calls[1];
    const handlePointerDown = firstCell.on.mock.calls[2][1];
    const handleSecondPointerOver = secondCell.on.mock.calls[1][1];

    await handlePointerDown(); // start the stroke on the first cell
    await handleSecondPointerOver(); // drag onto the second cell

    expect(mockSendCommand).toHaveBeenCalledTimes(2);
    expect(firstCell.clear).toHaveBeenCalled();
    expect(secondCell.clear).toHaveBeenCalled();
  });

  it("does not re-place infrastructure when re-entering an already-dragged cell", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 0, maxY: 0 }); // 1x1 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    render(<Park tool={{ mode: "infrastructure" }} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    const cell = mockStageAddChild.mock.calls[0][0];
    const handlePointerDown = cell.on.mock.calls[2][1];
    const handlePointerOver = cell.on.mock.calls[1][1];

    await handlePointerDown();
    await handlePointerOver(); // re-entering the same cell mid-stroke

    expect(mockSendCommand).toHaveBeenCalledTimes(1);
  });

  it("stops extending the stroke once the pointer is released", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 0 }); // 2x1 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    render(<Park tool={{ mode: "infrastructure" }} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    const [firstCell] = mockStageAddChild.mock.calls[0];
    const [secondCell] = mockStageAddChild.mock.calls[1];
    const handlePointerDown = firstCell.on.mock.calls[2][1];
    const handleSecondPointerOver = secondCell.on.mock.calls[1][1];

    await handlePointerDown();
    window.dispatchEvent(new Event("pointerup"));
    await handleSecondPointerOver(); // hovering after release should not place

    expect(mockSendCommand).toHaveBeenCalledTimes(1);
  });

  it("applies terrain to a cell when the terrain tool is active", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 0, maxY: 0 }); // 1x1 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    render(<Park tool={{ mode: "terrain" }} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    // Terrain application is driven by pointerdown (drag-tracing), not
    // pointertap: index 0 is pointertap, 1 is pointerover, 2 is pointerdown.
    const cell = mockStageAddChild.mock.calls[0][0];
    const handlePointerDown = cell.on.mock.calls[2][1];

    await handlePointerDown();

    expect(mockSendCommand).toHaveBeenCalledWith(
      expect.objectContaining({
        parkId: "default",
        command: expect.objectContaining({
          $case: "applyTerrain",
          applyTerrain: expect.objectContaining({ materialId: "grass" }),
        }),
      }),
    );
    expect(cell.clear).toHaveBeenCalled();
  });

  it("applies the selected material instead of the default when one is chosen", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 0, maxY: 0 }); // 1x1 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    render(
      <Park
        tool={{ mode: "terrain", selectedMaterialId: "water" }}
        onToolChange={vi.fn()}
      />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    const cell = mockStageAddChild.mock.calls[0][0];
    const handlePointerDown = cell.on.mock.calls[2][1];
    await handlePointerDown();

    expect(mockSendCommand).toHaveBeenCalledWith(
      expect.objectContaining({
        command: expect.objectContaining({
          $case: "applyTerrain",
          applyTerrain: expect.objectContaining({ materialId: "water" }),
        }),
      }),
    );
  });

  it("places a building covering its whole footprint when the building tool is active", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 1 }); // 2x2 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    render(<Park tool={{ mode: "building" }} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    // Cells are added in (y, x) order: (0,0) is the first one added.
    const originCell = mockStageAddChild.mock.calls[0][0];
    const handleCellClick = originCell.on.mock.calls[0][1];

    await handleCellClick();

    expect(mockSendCommand).toHaveBeenCalledWith(
      expect.objectContaining({
        command: expect.objectContaining({ $case: "placeBuilding" }),
      }),
    );
    // sit_down_restaurant's catalogue footprint (the default building) is a
    // 2x2 square covering the whole map. The first 4 addChild calls are the
    // grid cells (Graphics), the rest are the axis labels (Text, no `.clear`).
    for (const [cell] of mockStageAddChild.mock.calls.slice(0, 4)) {
      expect(cell.clear).toHaveBeenCalled();
    }
  });

  it("refuses to place a building whose footprint overlaps existing infrastructure", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 1 }); // 2x2 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    const { rerender } = render(
      <Park tool={{ mode: "infrastructure" }} onToolChange={vi.fn()} />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    // Cells are added in (y, x) order: index 1 is (1,0), part of the
    // building's 2x2 footprint placed at origin (0,0) below.
    // Infrastructure placement is driven by pointerdown (index 2).
    const pathCell = mockStageAddChild.mock.calls[1][0];
    const handlePathCellPointerDown = pathCell.on.mock.calls[2][1];
    await handlePathCellPointerDown(); // lay a path on a footprint cell

    rerender(<Park tool={{ mode: "building" }} onToolChange={vi.fn()} />);
    mockSendCommand.mockClear();

    const originCell = mockStageAddChild.mock.calls[0][0];
    const handleOriginCellClick = originCell.on.mock.calls[0][1];
    await handleOriginCellClick();

    expect(toast.error).toHaveBeenCalledWith(
      "Failed to place the building",
      expect.objectContaining({
        description: expect.stringContaining("infrastructure"),
      }),
    );
    expect(mockSendCommand).not.toHaveBeenCalledWith(
      expect.objectContaining({
        command: expect.objectContaining({ $case: "placeBuilding" }),
      }),
    );
  });

  it("refuses to place a building whose footprint stands on non-buildable terrain (water)", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 1 }); // 2x2 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    const { rerender } = render(
      <Park
        tool={{ mode: "terrain", selectedMaterialId: "water" }}
        onToolChange={vi.fn()}
      />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    // (1,0) is part of the building's 2x2 footprint placed at origin
    // (0,0) below. Terrain application is driven by pointerdown (index 2).
    const waterCell = mockStageAddChild.mock.calls[1][0];
    const handleWaterCellPointerDown = waterCell.on.mock.calls[2][1];
    await handleWaterCellPointerDown(); // turn a footprint cell into water

    rerender(<Park tool={{ mode: "building" }} onToolChange={vi.fn()} />);
    mockSendCommand.mockClear();

    const originCell = mockStageAddChild.mock.calls[0][0];
    const handleOriginCellClick = originCell.on.mock.calls[0][1];
    await handleOriginCellClick();

    expect(toast.error).toHaveBeenCalledWith(
      "Failed to place the building",
      expect.objectContaining({
        description: expect.stringContaining("terrain"),
      }),
    );
    expect(mockSendCommand).not.toHaveBeenCalledWith(
      expect.objectContaining({
        command: expect.objectContaining({ $case: "placeBuilding" }),
      }),
    );
  });

  it("traces a drag across multiple cells, applying terrain on each new one entered", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 0 }); // 2x1 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    render(<Park tool={{ mode: "terrain" }} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    const [firstCell] = mockStageAddChild.mock.calls[0];
    const [secondCell] = mockStageAddChild.mock.calls[1];
    const handlePointerDown = firstCell.on.mock.calls[2][1];
    const handleSecondPointerOver = secondCell.on.mock.calls[1][1];

    await handlePointerDown(); // start the stroke on the first cell
    await handleSecondPointerOver(); // drag onto the second cell

    expect(mockSendCommand).toHaveBeenCalledTimes(2);
    expect(firstCell.clear).toHaveBeenCalled();
    expect(secondCell.clear).toHaveBeenCalled();
  });

  it("asks for confirmation and removes a building when the remove tool is used on it", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 1 }); // 2x2 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);

    const { rerender } = render(
      <Park tool={{ mode: "building" }} onToolChange={vi.fn()} />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    const originCell = mockStageAddChild.mock.calls[0][0];
    const handleCellClick = originCell.on.mock.calls[0][1];
    await handleCellClick(); // place the building first

    rerender(<Park tool={{ mode: "remove" }} onToolChange={vi.fn()} />);
    mockSendCommand.mockClear();

    // Removal is driven by pointerdown (drag-tracing), not pointertap.
    const handlePointerDown = originCell.on.mock.calls[2][1];
    await handlePointerDown(); // click the same (now built-on) cell again

    expect(confirmSpy).toHaveBeenCalled();
    expect(mockSendCommand).toHaveBeenCalledWith(
      expect.objectContaining({
        command: expect.objectContaining({ $case: "removeBuilding" }),
      }),
    );

    confirmSpy.mockRestore();
  });

  it("restores a building received from the map on load, fillable and removable without re-placing it", async () => {
    mockOnMap.mockResolvedValue({
      ...emptyMap,
      maxX: 1,
      maxY: 1, // 2x2 map
      building: [
        { coord: { x: 0, y: 0, z: 0 }, buildingId: "b1", templateId: "shop" },
        { coord: { x: 1, y: 0, z: 0 }, buildingId: "b1", templateId: "shop" },
      ],
    });
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);

    render(<Park tool={{ mode: "remove" }} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    // Cells are added in (y, x) order: index 0 is (0,0), index 1 is (1,0),
    // both restored from the map response without ever being placed in-session.
    const originCell = mockStageAddChild.mock.calls[0][0];
    const handlePointerDown = originCell.on.mock.calls[2][1];
    await handlePointerDown();

    expect(confirmSpy).toHaveBeenCalledTimes(1);
    expect(mockSendCommand).toHaveBeenCalledWith(
      expect.objectContaining({
        command: expect.objectContaining({ $case: "removeBuilding" }),
      }),
    );

    confirmSpy.mockRestore();
  });

  it("does not remove a building if the confirmation is declined", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 1 }); // 2x2 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);

    const { rerender } = render(
      <Park tool={{ mode: "building" }} onToolChange={vi.fn()} />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    const originCell = mockStageAddChild.mock.calls[0][0];
    const handleCellClick = originCell.on.mock.calls[0][1];
    await handleCellClick(); // place the building first

    rerender(<Park tool={{ mode: "remove" }} onToolChange={vi.fn()} />);
    mockSendCommand.mockClear();

    // Removal is driven by pointerdown (drag-tracing), not pointertap.
    const handlePointerDown = originCell.on.mock.calls[2][1];
    await handlePointerDown();

    expect(confirmSpy).toHaveBeenCalled();
    expect(mockSendCommand).not.toHaveBeenCalled();

    confirmSpy.mockRestore();
  });

  it("asks for confirmation only once when dragging the remove tool across the same building", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 1 }); // 2x2 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);

    const { rerender } = render(
      <Park tool={{ mode: "building" }} onToolChange={vi.fn()} />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    // Origin (0,0) and (1,0) both belong to the building's 2x2 footprint.
    const originCell = mockStageAddChild.mock.calls[0][0];
    const secondCell = mockStageAddChild.mock.calls[1][0];
    const handleCellClick = originCell.on.mock.calls[0][1];
    await handleCellClick(); // place the building first

    rerender(<Park tool={{ mode: "remove" }} onToolChange={vi.fn()} />);
    mockSendCommand.mockClear();

    const handlePointerDown = originCell.on.mock.calls[2][1];
    const handleSecondPointerOver = secondCell.on.mock.calls[1][1];
    await handlePointerDown(); // start the drag on the building's origin
    await handleSecondPointerOver(); // drag onto another cell of the same building

    expect(confirmSpy).toHaveBeenCalledTimes(1);
    expect(mockSendCommand).toHaveBeenCalledTimes(1);

    confirmSpy.mockRestore();
  });

  it("removes infrastructure without confirmation when the remove tool is used on it", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 0, maxY: 0 }); // 1x1 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });
    const confirmSpy = vi.spyOn(window, "confirm");

    const { rerender } = render(
      <Park tool={{ mode: "infrastructure" }} onToolChange={vi.fn()} />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    const cell = mockStageAddChild.mock.calls[0][0];
    const handlePointerDown = cell.on.mock.calls[2][1];
    await handlePointerDown(); // place a path first

    rerender(<Park tool={{ mode: "remove" }} onToolChange={vi.fn()} />);
    mockSendCommand.mockClear();

    // Removal is driven by pointerdown (drag-tracing), not pointertap.
    await handlePointerDown(); // click the same (now built-on) cell again

    expect(confirmSpy).not.toHaveBeenCalled();
    expect(mockSendCommand).toHaveBeenCalledWith(
      expect.objectContaining({
        command: expect.objectContaining({ $case: "removeInfrastructure" }),
      }),
    );

    confirmSpy.mockRestore();
  });

  it("shows a valid (green) ghost when hovering an unobstructed footprint", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 1 }); // 2x2 map

    render(<Park tool={{ mode: "building" }} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    const originCell = mockStageAddChild.mock.calls[0][0];
    const handlePointerOver = originCell.on.mock.calls[1][1];
    const ghost =
      mockStageAddChild.mock.calls[mockStageAddChild.mock.calls.length - 1][0];

    handlePointerOver();

    expect(ghost.clear).toHaveBeenCalled();
    expect(ghost.fill).toHaveBeenCalledWith(
      expect.objectContaining({ color: 0x2e6e62 }),
    );
  });

  it("shows an invalid (red) ghost when the footprint would go out of bounds", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 0, maxY: 0 }); // 1x1 map

    render(<Park tool={{ mode: "building" }} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    const cell = mockStageAddChild.mock.calls[0][0];
    const handlePointerOver = cell.on.mock.calls[1][1];
    const ghost =
      mockStageAddChild.mock.calls[mockStageAddChild.mock.calls.length - 1][0];

    handlePointerOver();

    expect(ghost.fill).toHaveBeenCalledWith(
      expect.objectContaining({ color: 0xdc2626 }),
    );
  });

  it("hides the ghost again once the cursor leaves the canvas container", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 1 }); // 2x2 map

    const { container } = render(
      <Park tool={{ mode: "building" }} onToolChange={vi.fn()} />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    const originCell = mockStageAddChild.mock.calls[0][0];
    const handlePointerOver = originCell.on.mock.calls[1][1];
    handlePointerOver();

    const ghost =
      mockStageAddChild.mock.calls[mockStageAddChild.mock.calls.length - 1][0];
    ghost.clear.mockClear();

    const target = container.querySelector('[tabindex="0"]')!;
    fireEvent.pointerLeave(target);

    expect(ghost.clear).toHaveBeenCalled();
  });

  it("focuses the canvas container on mount so keyboard shortcuts work immediately", () => {
    mockOnMap.mockReturnValue(new Promise(() => {})); // never resolves

    const { container } = render(<Park tool={NO_TOOL} onToolChange={vi.fn()} />);

    const target = container.querySelector('[tabindex="0"]')!;
    expect(target).toHaveFocus();
  });

  it("selects a tool via its number key shortcut", () => {
    mockOnMap.mockReturnValue(new Promise(() => {}));
    const onToolChange = vi.fn();

    const { container } = render(
      <Park tool={{ mode: null }} onToolChange={onToolChange} />,
    );

    const target = container.querySelector('[tabindex="0"]')!;
    fireEvent.keyDown(target, { code: "Digit3" });

    expect(onToolChange).toHaveBeenCalledWith({ mode: "building" });
  });

  it("selects a tool by physical key position even on a layout where digits need Shift (AZERTY)", () => {
    mockOnMap.mockReturnValue(new Promise(() => {}));
    const onToolChange = vi.fn();

    const { container } = render(
      <Park tool={{ mode: null }} onToolChange={onToolChange} />,
    );

    const target = container.querySelector('[tabindex="0"]')!;
    // On AZERTY, the "3" key position reports key: "\"" without Shift —
    // event.code stays "Digit3" regardless of layout, which is what we match on.
    fireEvent.keyDown(target, { key: '"', code: "Digit3" });

    expect(onToolChange).toHaveBeenCalledWith({ mode: "building" });
  });

  it("toggles the active tool off when its shortcut is pressed again", () => {
    mockOnMap.mockReturnValue(new Promise(() => {}));
    const onToolChange = vi.fn();

    const { container } = render(
      <Park tool={{ mode: "remove" }} onToolChange={onToolChange} />,
    );

    const target = container.querySelector('[tabindex="0"]')!;
    fireEvent.keyDown(target, { code: "Digit4" });

    expect(onToolChange).toHaveBeenCalledWith({ mode: null });
  });

  it("rotates the building ghost with R, and in reverse with Shift+R", () => {
    mockOnMap.mockReturnValue(new Promise(() => {}));
    const onToolChange = vi.fn();

    const { container } = render(
      <Park tool={{ mode: "building" }} onToolChange={onToolChange} />,
    );

    const target = container.querySelector('[tabindex="0"]')!;
    fireEvent.keyDown(target, { code: "KeyR" });
    expect(onToolChange).toHaveBeenCalledWith({
      mode: "building",
      rotation: Rotation.ROTATION_DEG_90,
    });

    fireEvent.keyDown(target, { code: "KeyR", shiftKey: true });
    expect(onToolChange).toHaveBeenCalledWith({
      mode: "building",
      rotation: Rotation.ROTATION_DEG_270,
    });
  });

  it("visually presses the matching rotate button when triggered from the keyboard", () => {
    vi.useFakeTimers();
    try {
      mockOnMap.mockReturnValue(new Promise(() => {}));

      const { container } = render(
        <Park tool={{ mode: "building" }} onToolChange={vi.fn()} />,
      );

      const target = container.querySelector('[tabindex="0"]')!;
      const cwButton = container.querySelector('button[aria-label="Rotation (R)"]')!;
      const ccwButton = container.querySelector(
        'button[aria-label="Rotation inversée (Maj+R)"]',
      )!;

      expect(cwButton).not.toHaveClass("translate-y-px");

      act(() => {
        fireEvent.keyDown(target, { code: "KeyR" });
      });
      expect(cwButton).toHaveClass("translate-y-px");
      expect(ccwButton).not.toHaveClass("translate-y-px");

      act(() => {
        vi.advanceTimersByTime(150);
      });
      expect(cwButton).not.toHaveClass("translate-y-px");

      act(() => {
        fireEvent.keyDown(target, { code: "KeyR", shiftKey: true });
      });
      expect(ccwButton).toHaveClass("translate-y-px");
      expect(cwButton).not.toHaveClass("translate-y-px");

      act(() => {
        vi.advanceTimersByTime(150);
      });
      expect(ccwButton).not.toHaveClass("translate-y-px");
    } finally {
      vi.useRealTimers();
    }
  });

  it("does not rotate when the building tool is not active", () => {
    mockOnMap.mockReturnValue(new Promise(() => {}));
    const onToolChange = vi.fn();

    const { container } = render(
      <Park tool={{ mode: "terrain" }} onToolChange={onToolChange} />,
    );

    const target = container.querySelector('[tabindex="0"]')!;
    fireEvent.keyDown(target, { code: "KeyR" });

    expect(onToolChange).not.toHaveBeenCalled();
  });

  it("cancels the active tool on Escape", () => {
    mockOnMap.mockReturnValue(new Promise(() => {}));
    const onToolChange = vi.fn();

    const { container } = render(
      <Park tool={{ mode: "building" }} onToolChange={onToolChange} />,
    );

    const target = container.querySelector('[tabindex="0"]')!;
    fireEvent.keyDown(target, { code: "Escape" });

    expect(onToolChange).toHaveBeenCalledWith({ mode: null });
  });

  it("prevents the browser default action only for its shortcut keys", () => {
    mockOnMap.mockReturnValue(new Promise(() => {}));

    const { container } = render(
      <Park tool={{ mode: null }} onToolChange={vi.fn()} />,
    );

    const target = container.querySelector('[tabindex="0"]')!;
    const notPrevented = fireEvent.keyDown(target, { code: "KeyA" });
    const prevented = fireEvent.keyDown(target, { code: "Digit1" });

    expect(notPrevented).toBe(true);
    expect(prevented).toBe(false);
  });

  it("reclaims keyboard focus after a click on a Toolbar button", async () => {
    mockOnMap.mockReturnValue(new Promise(() => {}));

    const { container } = render(
      <Park tool={{ mode: null }} onToolChange={vi.fn()} />,
    );

    const target = container.querySelector<HTMLElement>('[tabindex="0"]')!;
    target.blur();
    expect(target).not.toHaveFocus();

    const buildingButton = container.querySelector<HTMLElement>(
      'button[aria-label="Bâtiment"]',
    )!;
    buildingButton.focus();
    fireEvent.click(buildingButton);

    expect(target).toHaveFocus();
  });

  it("sends the currently selected rotation when placing a building", async () => {
    mockOnMap.mockResolvedValue({ ...emptyMap, maxX: 1, maxY: 1 }); // 2x2 map
    mockSendCommand.mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    render(
      <Park
        tool={{ mode: "building", rotation: Rotation.ROTATION_DEG_90 }}
        onToolChange={vi.fn()}
      />,
    );
    await flushMicrotasks();
    await flushMicrotasks();

    const originCell = mockStageAddChild.mock.calls[0][0];
    const handleCellClick = originCell.on.mock.calls[0][1];
    await handleCellClick();

    expect(mockSendCommand).toHaveBeenCalledWith(
      expect.objectContaining({
        command: expect.objectContaining({
          $case: "placeBuilding",
          placeBuilding: expect.objectContaining({
            rotation: Rotation.ROTATION_DEG_90,
          }),
        }),
      }),
    );
  });

  it("renders a visitor sprite when a world state update is received", async () => {
    mockOnMap.mockResolvedValue(emptyMap);

    render(<Park tool={NO_TOOL} onToolChange={vi.fn()} />);
    await flushMicrotasks();
    await flushMicrotasks();

    // The visitor-sync subscription is registered after the map loads, so it
    // is the last of the two onWorldState subscriptions (the other feeds the
    // top bar from mount).
    const lastCallIndex = mockOnWorldState.mock.calls.length - 1;
    const onWorldStateCallback = mockOnWorldState.mock.calls[lastCallIndex]![0]!;
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

  it("updates the top bar balance and visitor count from world state updates", async () => {
    mockOnMap.mockReturnValue(new Promise(() => {})); // never resolves: not relevant to this test

    const { getByText } = render(
      <Park tool={NO_TOOL} onToolChange={vi.fn()} />,
    );

    const onWorldStateCallback = mockOnWorldState.mock.calls[0]![0]!;
    act(() => {
      onWorldStateCallback({
        tickCount: 1,
        dirtyChunksJson: "{}",
        visitors: [
          { id: "v1", x: 0, y: 0, z: 0 },
          { id: "v2", x: 1, y: 0, z: 0 },
        ],
        balance: 2500,
      });
    });

    expect(getByText("2 500 €")).toBeInTheDocument();
    expect(getByText("2 visiteurs")).toBeInTheDocument();
  });
});
