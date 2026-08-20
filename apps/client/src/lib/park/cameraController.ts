import type { Container } from "pixi.js";

interface CameraControllerOptions {
  cameraContainer: Container;
  viewportWidth: number;
  viewportHeight: number;
}

export function createCameraController({
  cameraContainer,
  viewportWidth,
  viewportHeight,
}: CameraControllerOptions) {
  function handleWheel(event: WheelEvent) {
    const ZOOM_SPEED = 0.001;
    const MIN_ZOOM = 0.5;
    const MAX_ZOOM = 3;

    const amount = event.deltaY;
    const currentScale = cameraContainer.scale._x;
    const newScale = Math.min(
      MAX_ZOOM,
      Math.max(MIN_ZOOM, currentScale * (1 - amount * ZOOM_SPEED)),
    );

    const position = { x: event.offsetX, y: event.offsetY };
    const worldX = (position.x - cameraContainer.position.x) / currentScale;
    const worldY = (position.y - cameraContainer.position.y) / currentScale;

    cameraContainer.scale.set(newScale);
    cameraContainer.position.set(
        position.x - worldX * newScale,
        position.y - worldY * newScale,
    );
  }

  return { handleWheel };
}

export type CameraController = ReturnType<typeof createCameraController>;
