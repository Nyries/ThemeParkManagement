import { describe, it, expect } from "vitest";
import type { Container } from "pixi.js";
import { createCameraController } from "../cameraController";

// Mimics the slice of PixiJS's Container/ObservablePoint API that
// cameraController touches (scale._x, position.{x,y}, both .set()) without
// pulling in real PixiJS — .set() keeps _x/x/y in sync so assertions can read
// either field regardless of which one the implementation currently uses.
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

function wheelEvent(deltaY: number, offsetX: number, offsetY: number) {
  return { deltaY, offsetX, offsetY } as WheelEvent;
}

describe("createCameraController", () => {
  describe("handleWheel", () => {
    it("zooms in when scrolling up (negative deltaY)", () => {
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        viewportWidth: 800,
        viewportHeight: 600,
      });

      handleWheel(wheelEvent(-100, 0, 0));

      expect(cameraContainer.scale.x).toBeGreaterThan(1);
    });

    it("zooms out when scrolling down (positive deltaY)", () => {
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        viewportWidth: 800,
        viewportHeight: 600,
      });

      handleWheel(wheelEvent(100, 0, 0));

      expect(cameraContainer.scale.x).toBeLessThan(1);
    });

    it("clamps zoom to MAX_ZOOM instead of scaling past it", () => {
      const cameraContainer = createFakeContainer(2.99, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        viewportWidth: 800,
        viewportHeight: 600,
      });

      handleWheel(wheelEvent(-100_000, 0, 0));

      expect(cameraContainer.scale.x).toBe(3);
    });

    it("clamps zoom to MIN_ZOOM instead of scaling below it", () => {
      const cameraContainer = createFakeContainer(0.51, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        viewportWidth: 800,
        viewportHeight: 600,
      });

      handleWheel(wheelEvent(100_000, 0, 0));

      expect(cameraContainer.scale.x).toBe(0.5);
    });

    it("keeps the world point under the cursor fixed when zooming from the origin", () => {
      const cameraContainer = createFakeContainer(1, 0, 0);
      const { handleWheel } = createCameraController({
        cameraContainer,
        viewportWidth: 800,
        viewportHeight: 600,
      });

      handleWheel(wheelEvent(-100, 100, 50));

      // factor = 1 - (-100 * 0.001) = 1.1
      expect(cameraContainer.scale.x).toBeCloseTo(1.1);
      expect(cameraContainer.position.x).toBeCloseTo(100 - 100 * 1.1);
      expect(cameraContainer.position.y).toBeCloseTo(50 - 50 * 1.1);
    });

    it("keeps the world point under the cursor fixed when the camera is already panned and zoomed", () => {
      const cameraContainer = createFakeContainer(2, 20, 10);
      const { handleWheel } = createCameraController({
        cameraContainer,
        viewportWidth: 800,
        viewportHeight: 600,
      });

      const cursor = { x: 300, y: 150 };
      const worldXBefore = (cursor.x - cameraContainer.position.x) / cameraContainer.scale.x;
      const worldYBefore = (cursor.y - cameraContainer.position.y) / cameraContainer.scale.y;

      handleWheel(wheelEvent(-50, cursor.x, cursor.y));

      const worldXAfter =
        (cursor.x - cameraContainer.position.x) / cameraContainer.scale.x;
      const worldYAfter =
        (cursor.y - cameraContainer.position.y) / cameraContainer.scale.y;
      expect(worldXAfter).toBeCloseTo(worldXBefore);
      expect(worldYAfter).toBeCloseTo(worldYBefore);
    });
  });
});
