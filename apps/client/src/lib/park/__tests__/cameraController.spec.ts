import { describe, it, expect } from "vitest";
import type { Container } from "pixi.js";
import type { ToolState } from "@/types/park/tool";
import { createCameraController } from "../cameraController";

// Mimics the slice of PixiJS's Container/ObservablePoint API that
// cameraController touches (scale._x/x/y, position.{x,y}, both .set())
// without pulling in real PixiJS — .set() keeps _x/x/y in sync so assertions
// can read either field regardless of which one the implementation uses.
function createFakeContainer(
  scaleX: number,
  positionX: number,
  positionY: number,
): Container {
  const scale = {
    _x: scaleX,
    x: scaleX,
    y: scaleX,
    set: (v: number) => {
      scale._x = v;
      scale.x = v;
      scale.y = v;
    },
  };
  const position = {
    x: positionX,
    y: positionY,
    set: (x: number, y: number) => {
      position.x = x;
      position.y = y;
    },
  };
  return { scale, position } as unknown as Container;
}

function toolRefWithMode(mode: ToolState["mode"]) {
  return { current: { mode } };
}

function wheelEvent(deltaY: number, offsetX: number, offsetY: number) {
  return { deltaY, offsetX, offsetY } as WheelEvent;
}

function pointerEvent(clientX: number, clientY: number) {
  return { clientX, clientY } as PointerEvent;
}

// A world much larger than the viewport by default, so panning/zooming in
// the tests below actually has room to move before hitting a clamp bound
// (unless a test is specifically about clamping).
const LARGE_WORLD = { worldWidth: 5000, worldHeight: 5000 };
const VIEWPORT = { viewportWidth: 800, viewportHeight: 600 };

describe("createCameraController", () => {
  describe("handleWheel", () => {
    it("zooms in when scrolling up (negative deltaY)", () => {
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        ...VIEWPORT,
        ...LARGE_WORLD,
      });

      handleWheel(wheelEvent(-100, 0, 0));

      expect(cameraContainer.scale.x).toBeGreaterThan(1);
    });

    it("zooms out when scrolling down (positive deltaY)", () => {
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        ...VIEWPORT,
        ...LARGE_WORLD,
      });

      handleWheel(wheelEvent(100, 0, 0));

      expect(cameraContainer.scale.x).toBeLessThan(1);
    });

    it("clamps zoom to MAX_ZOOM instead of scaling past it", () => {
      const cameraContainer = createFakeContainer(2.99, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        ...VIEWPORT,
        ...LARGE_WORLD,
      });

      handleWheel(wheelEvent(-100_000, 0, 0));

      expect(cameraContainer.scale.x).toBe(3);
    });

    it("clamps zoom to MIN_ZOOM instead of scaling below it", () => {
      const cameraContainer = createFakeContainer(0.91, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        ...VIEWPORT,
        ...LARGE_WORLD,
      });

      handleWheel(wheelEvent(100_000, 0, 0));

      expect(cameraContainer.scale.x).toBe(0.9);
    });

    it("keeps the world point under the cursor fixed when zooming from the origin", () => {
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        ...VIEWPORT,
        ...LARGE_WORLD,
      });

      handleWheel(wheelEvent(-100, 100, 50));

      // factor = 1 - (-100 * 0.001) = 1.1
      expect(cameraContainer.scale.x).toBeCloseTo(1.1);
      expect(cameraContainer.position.x).toBeCloseTo(100 - 100 * 1.1);
      expect(cameraContainer.position.y).toBeCloseTo(50 - 50 * 1.1);
    });

    it("keeps the world point under the cursor fixed when the camera is already panned and zoomed", () => {
      const cameraContainer = createFakeContainer(2, -100, -50);
      const { handleWheel } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        ...VIEWPORT,
        ...LARGE_WORLD,
      });

      const cursor = { x: 300, y: 150 };
      const worldXBefore =
        (cursor.x - cameraContainer.position.x) / cameraContainer.scale.x;
      const worldYBefore =
        (cursor.y - cameraContainer.position.y) / cameraContainer.scale.y;

      handleWheel(wheelEvent(-50, cursor.x, cursor.y));

      const worldXAfter =
        (cursor.x - cameraContainer.position.x) / cameraContainer.scale.x;
      const worldYAfter =
        (cursor.y - cameraContainer.position.y) / cameraContainer.scale.y;
      expect(worldXAfter).toBeCloseTo(worldXBefore);
      expect(worldYAfter).toBeCloseTo(worldYBefore);
    });

    it("clamps the resulting position so zooming out never reveals space beyond the world's edge", () => {
      // World barely bigger than viewport at scale 1: zooming out from the
      // corner would otherwise push position.x/y past 0 (max bound).
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        viewportWidth: 800,
        viewportHeight: 600,
        worldWidth: 810,
        worldHeight: 610,
      });

      handleWheel(wheelEvent(100, 0, 0)); // zoom out, cursor at the top-left corner

      expect(cameraContainer.position.x).toBeLessThanOrEqual(0);
      expect(cameraContainer.position.y).toBeLessThanOrEqual(0);
    });
  });

  describe("pan (handlePointerDown/handlePointerMove/handlePointerUp)", () => {
    it("pans the camera while dragging with no tool active", () => {
      // -2000,-2000 sits comfortably inside the valid range for a 5000x5000
      // world in an 800x600 viewport ([-4200, 0] on each axis), so this drag
      // doesn't hit a clamp bound — see the dedicated clamp tests below for that.
      const cameraContainer = createFakeContainer(1, -2000, -2000);
      const { handlePointerDown, handlePointerMove } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        ...VIEWPORT,
        ...LARGE_WORLD,
      });

      handlePointerDown(pointerEvent(100, 100));
      handlePointerMove(pointerEvent(140, 130));

      expect(cameraContainer.position.x).toBeCloseTo(-1960);
      expect(cameraContainer.position.y).toBeCloseTo(-1970);
    });

    it("does not pan when a building/terrain/infrastructure/remove tool is active", () => {
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handlePointerDown, handlePointerMove } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("terrain"),
        ...VIEWPORT,
        ...LARGE_WORLD,
      });

      handlePointerDown(pointerEvent(100, 100));
      handlePointerMove(pointerEvent(140, 130));

      expect(cameraContainer.position.x).toBe(0);
      expect(cameraContainer.position.y).toBe(0);
    });

    it("accumulates the pan across successive pointer moves", () => {
      const cameraContainer = createFakeContainer(1, -2000, -2000);
      const { handlePointerDown, handlePointerMove } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        ...VIEWPORT,
        ...LARGE_WORLD,
      });

      handlePointerDown(pointerEvent(100, 100));
      handlePointerMove(pointerEvent(120, 100));
      handlePointerMove(pointerEvent(150, 90));

      expect(cameraContainer.position.x).toBeCloseTo(-1950);
      expect(cameraContainer.position.y).toBeCloseTo(-2010);
    });

    it("stops panning once the pointer is released", () => {
      const cameraContainer = createFakeContainer(1, -2000, -2000);
      const { handlePointerDown, handlePointerMove, handlePointerUp } =
        createCameraController({
          cameraContainer,
          toolRef: toolRefWithMode("selection"),
          ...VIEWPORT,
          ...LARGE_WORLD,
        });

      handlePointerDown(pointerEvent(100, 100));
      handlePointerMove(pointerEvent(140, 130));
      handlePointerUp();
      handlePointerMove(pointerEvent(500, 500)); // moving after release should not pan

      expect(cameraContainer.position.x).toBeCloseTo(-1960);
      expect(cameraContainer.position.y).toBeCloseTo(-1970);
    });

    it("does not pan before any pointerdown has started a drag", () => {
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handlePointerMove } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        ...VIEWPORT,
        ...LARGE_WORLD,
      });

      handlePointerMove(pointerEvent(140, 130));

      expect(cameraContainer.position.x).toBe(0);
      expect(cameraContainer.position.y).toBe(0);
    });

    it("clamps panning so the world can never be dragged out of the viewport", () => {
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handlePointerDown, handlePointerMove } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        ...VIEWPORT,
        worldWidth: 5000,
        worldHeight: 5000,
      });

      handlePointerDown(pointerEvent(0, 0));
      // A huge drag toward the bottom-right, far past what a real gesture
      // would cover — must still be clamped to the "world's left/top edge at
      // the viewport's left/top edge" bound (0), not go positive.
      handlePointerMove(pointerEvent(10_000, 10_000));

      expect(cameraContainer.position.x).toBe(0);
      expect(cameraContainer.position.y).toBe(0);
    });

    it("clamps panning so a world smaller than the viewport stays fully visible", () => {
      // World smaller than the viewport (e.g. zoomed far out): the valid
      // range is [0, viewport - world] on each axis, not [-Infinity, 0].
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handlePointerDown, handlePointerMove } = createCameraController({
        cameraContainer,
        toolRef: toolRefWithMode("selection"),
        viewportWidth: 800,
        viewportHeight: 600,
        worldWidth: 200,
        worldHeight: 150,
      });

      handlePointerDown(pointerEvent(0, 0));
      handlePointerMove(pointerEvent(-10_000, -10_000)); // drag toward negative, past the 0 lower bound

      expect(cameraContainer.position.x).toBe(0);
      expect(cameraContainer.position.y).toBe(0);
    });
  });
});
