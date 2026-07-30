import { describe, expect, it } from "vitest";
import {
  CANVAS_HEIGHT,
  CANVAS_WIDTH,
  CELL_SIZE,
  MARGIN_SIZE,
  getCellColor,
  INFRASTRUCTURE_COLORS,
  TERRAIN_COLORS,
  toScreenX,
  toScreenY,
} from "../grid";
import { WIDTH as GRID_WIDTH, HEIGHT as GRID_HEIGHT } from "../../mocks/mockMap";
import type { InfrastructureKind, TerrainMaterial } from "../../mocks/mockMap";

describe("toScreenX", () => {
  it("places x=0 right after the left margin", () => {
    expect(toScreenX(0)).toBe(MARGIN_SIZE);
  });

  it("advances by CELL_SIZE per column", () => {
    expect(toScreenX(5)).toBe(MARGIN_SIZE + 5 * CELL_SIZE);
  });
});

describe("toScreenY", () => {
  it("places the last row (GRID_HEIGHT - 1) at the top of the canvas", () => {
    expect(toScreenY(GRID_HEIGHT - 1)).toBe(0);
  });

  it("places row 0 at the bottom of the grid", () => {
    expect(toScreenY(0)).toBe((GRID_HEIGHT - 1) * CELL_SIZE);
  });
});

describe("getCellColor", () => {
  function buildMatrices() {
    const terrain: TerrainMaterial[][] = Array.from({ length: 2 }, () =>
      Array.from({ length: 2 }, () => "grass" as TerrainMaterial),
    );
    const infrastructure: (InfrastructureKind | null)[][] = Array.from(
      { length: 2 },
      () => Array.from({ length: 2 }, () => null),
    );
    return { terrain, infrastructure };
  }

  it("returns the terrain color when there is no infrastructure on the cell", () => {
    const { terrain, infrastructure } = buildMatrices();
    terrain[0][0] = "water";

    expect(getCellColor(0, 0, terrain, infrastructure)).toBe(TERRAIN_COLORS.water);
  });

  it("returns the infrastructure color when the cell has one, ignoring terrain", () => {
    const { terrain, infrastructure } = buildMatrices();
    terrain[1][1] = "water";
    infrastructure[1][1] = "path";

    expect(getCellColor(1, 1, terrain, infrastructure)).toBe(INFRASTRUCTURE_COLORS.path);
  });

  it("uses [y][x] indexing, not [x][y]", () => {
    const { terrain, infrastructure } = buildMatrices();
    terrain[0][1] = "water"; // ligne 0, colonne 1

    expect(getCellColor(1, 0, terrain, infrastructure)).toBe(TERRAIN_COLORS.water);
    expect(getCellColor(0, 1, terrain, infrastructure)).toBe(TERRAIN_COLORS.grass);
  });
});

describe("canvas dimensions", () => {
  it("derive from grid dimensions, cell size and margin", () => {
    expect(CANVAS_WIDTH).toBe(MARGIN_SIZE + GRID_WIDTH * CELL_SIZE);
    expect(CANVAS_HEIGHT).toBe(MARGIN_SIZE + GRID_HEIGHT * CELL_SIZE);
  });
});
