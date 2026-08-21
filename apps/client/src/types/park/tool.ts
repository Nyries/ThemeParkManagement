import { Rotation } from "@app/shared-types";
import type { TerrainMaterial } from "../../rendering/grid";

export type ToolMode = "selection" | "terrain" | "infrastructure" | "building" | "remove";
export interface ToolState {
    mode: ToolMode;
    rotation?: Rotation;
    selectedMaterialId?: TerrainMaterial;
    selectedBuildingId?: string;
}

export interface PlaceBuilding {
    templateId: string;
    origin: { x: number, y: number};
    rotation: Rotation;
    footprint: { x: number, y: number}[];
}

const ROTATION_ORDER = [
  Rotation.ROTATION_DEG_0,
  Rotation.ROTATION_DEG_90,
  Rotation.ROTATION_DEG_180,
  Rotation.ROTATION_DEG_270,
];

export function nextRotation(rotation: Rotation, reverse = false): Rotation {
  const index = ROTATION_ORDER.indexOf(rotation);
  const delta = reverse ? -1 : 1;
  return ROTATION_ORDER[
    (index + delta + ROTATION_ORDER.length) % ROTATION_ORDER.length
  ];
}

function rotatePoint(
  point: { x: number; y: number },
  steps: number,
): { x: number; y: number } {
  let { x, y } = point;
  for (let i = 0; i < steps; i++) {
    const rotatedX = -y;
    const rotatedY = x;
    x = rotatedX;
    y = rotatedY;
  }
  return { x, y };
}

export function rotateFootprint(
  footprint: { x: number; y: number }[],
  rotation: Rotation,
): { x: number; y: number }[] {
  const steps = ROTATION_ORDER.indexOf(rotation);
  const rotated = footprint.map((point) => rotatePoint(point, steps));
  const minX = Math.min(...rotated.map((point) => point.x));
  const minY = Math.min(...rotated.map((point) => point.y));
  return rotated.map((point) => ({ x: point.x - minX, y: point.y - minY }));
}