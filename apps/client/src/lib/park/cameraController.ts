import type { ToolState } from "@/types/park/tool";
import type { Container } from "pixi.js";

type PointerCoord = { x: number; y: number };

interface CameraControllerOptions {
  cameraContainer: Container;
  toolRef: { current: ToolState };
  viewportWidth: number;
  viewportHeight: number;
  worldWidth: number;
  worldHeight: number;
}

export function createCameraController({
  cameraContainer,
  toolRef,
  viewportWidth,
  viewportHeight,
  worldWidth,
  worldHeight,
}: CameraControllerOptions) {
  let isPanning = false;
  let lastPointer: PointerCoord | null = null;

  function clampPosition(x: number, y: number, scale: number): [number, number] {
    const scaledWorldWidth = worldWidth * scale;
    const scaledWorldHeight = worldHeight * scale;

    let minX: number, maxX: number;
    let minY: number, maxY: number;

    if (scaledWorldWidth >= viewportWidth) {
      minX = viewportWidth - scaledWorldWidth;
      maxX = 0;
    } else {
      minX = 0;
      maxX = viewportWidth - scaledWorldWidth;
    }

    if (scaledWorldHeight >= viewportHeight) {
      minY = viewportHeight - scaledWorldHeight;
      maxY = 0;
    } else {
      minY = 0;
      maxY = viewportHeight - scaledWorldHeight;
    }

    const clampedX = Math.min(maxX, Math.max(minX, x));
    const clampedY = Math.min(maxY, Math.max(minY, y));

    return [clampedX, clampedY];
  }

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

    const [clampedX, clampedY] = clampPosition(
      position.x - worldX * newScale,
      position.y - worldY * newScale,
      newScale,
    );

    cameraContainer.scale.set(newScale);
    cameraContainer.position.set(clampedX, clampedY);
  }

  function handlePointerDown(event: PointerEvent) {
    if (toolRef.current.mode !== null) return;
    isPanning = true;
    lastPointer = { x: event.clientX, y: event.clientY };
  }

  function handlePointerMove(event: PointerEvent) {
    if (!isPanning || !lastPointer) return;
    const dx = event.clientX - lastPointer.x;
    const dy = event.clientY - lastPointer.y;

    const [clampedX, clampedY] = clampPosition(
      cameraContainer.position.x + dx,
      cameraContainer.position.y + dy,
      cameraContainer.scale.x,
    );
    cameraContainer.position.set(clampedX, clampedY);
    lastPointer = { x: event.clientX, y: event.clientY };
  }

  function handlePointerUp() {
    isPanning = false;
    lastPointer = null;
  }

  return { handleWheel, handlePointerDown, handlePointerUp, handlePointerMove };
}

export type CameraController = ReturnType<typeof createCameraController>;
