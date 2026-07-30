import type { InfrastructureKind, TerrainMaterial } from "../mocks/mockMap";

export const CELL_SIZE = 16;
export const MARGIN_SIZE = 20;

export const GRID_WIDTH = 50;
export const GRID_HEIGHT = 30;

export const CANVAS_WIDTH = MARGIN_SIZE + GRID_WIDTH * CELL_SIZE;
export const CANVAS_HEIGHT = MARGIN_SIZE + GRID_HEIGHT * CELL_SIZE;

export const TERRAIN_COLORS: Record<TerrainMaterial, number> = {
  grass: 0x4caf50,
  water: 0x2196f3,
};

export const INFRASTRUCTURE_COLORS: Record<InfrastructureKind, number> = {
  path: 0xd7b98e,
  ramp: 0xd7b98e,
  stairs: 0xd7b98e,
};

export function toScreenY(y: number): number {
  return (GRID_HEIGHT - 1 - y) * CELL_SIZE;
}

export function toScreenX(x: number): number {
  return x * CELL_SIZE + MARGIN_SIZE;
}

export function getCellColor(
  x: number,
  y: number,
  terrain: TerrainMaterial[][],
  infrastructure: (InfrastructureKind | null)[][],
): number {
  const infra = infrastructure[y][x];
  if (infra !== null) {
    return INFRASTRUCTURE_COLORS[infra];
  }
  return TERRAIN_COLORS[terrain[y][x]];
}