// apps/client/src/park/__tests__/mapFromResponse.spec.ts
import { describe, it, expect } from "vitest";
import { InfrastructureKind as ProtoInfrastructureKind } from "@app/shared-types/grpc";
import { mapFromResponse } from "../mapFromResponse";

function emptyResponse(overrides = {}) {
  return {
    minX: 0,
    maxX: 2,
    minY: 0,
    maxY: 1,
    terrain: [],
    infrastructure: [],
    entrance: undefined,
    building: [],
    ...overrides,
  };
}

describe("mapFromResponse", () => {
  it("computes width and height from the bounds", () => {
    const grid = mapFromResponse(emptyResponse({ minX: 0, maxX: 9, minY: 0, maxY: 7 }));

    expect(grid.width).toBe(10);
    expect(grid.height).toBe(8);
  });

  it("defaults every cell to grass terrain and no infrastructure", () => {
    const grid = mapFromResponse(emptyResponse());

    expect(grid.terrain).toEqual([
      ["grass", "grass", "grass"],
      ["grass", "grass", "grass"],
    ]);
    expect(grid.infrastructure).toEqual([
      [null, null, null],
      [null, null, null],
    ]);
  });

  it("places a terrain cell at its offset position", () => {
    const grid = mapFromResponse(emptyResponse({
      minX: 5,
      maxX: 7,
      minY: 10,
      maxY: 11,
      terrain: [{ coord: { x: 6, y: 11, z: 0 }, materialId: "water" }],
    }));

    expect(grid.terrain[1][1]).toBe("water");
  });

  it("maps each proto infrastructure kind to its client string equivalent", () => {
    const grid = mapFromResponse(emptyResponse({
      maxX: 2,
      infrastructure: [
        { coord: { x: 0, y: 0, z: 0 }, kind: ProtoInfrastructureKind.INFRASTRUCTURE_KIND_PATH, toZ: 0 },
        { coord: { x: 1, y: 0, z: 0 }, kind: ProtoInfrastructureKind.INFRASTRUCTURE_KIND_RAMP, toZ: 1 },
        { coord: { x: 2, y: 0, z: 0 }, kind: ProtoInfrastructureKind.INFRASTRUCTURE_KIND_STAIRS, toZ: 1 },
      ],
    }));

    expect(grid.infrastructure[0]).toEqual(["path", "ramp", "stairs"]);
  });

  it("leaves a cell without infrastructure as null for an unspecified kind", () => {
    const grid = mapFromResponse(emptyResponse({
      infrastructure: [
        { coord: { x: 0, y: 0, z: 0 }, kind: ProtoInfrastructureKind.INFRASTRUCTURE_KIND_UNSPECIFIED, toZ: 0 },
      ],
    }));

    expect(grid.infrastructure[0][0]).toBeNull();
  });

  it("ignores cells that are not on level z=0", () => {
    const grid = mapFromResponse(emptyResponse({
      terrain: [{ coord: { x: 0, y: 0, z: 1 }, materialId: "water" }],
      infrastructure: [{ coord: { x: 0, y: 0, z: -1 }, kind: ProtoInfrastructureKind.INFRASTRUCTURE_KIND_PATH, toZ: 0 }],
    }));

    expect(grid.terrain[0][0]).toBe("grass");
    expect(grid.infrastructure[0][0]).toBeNull();
  });

  it("ignores cells without a coord", () => {
    const grid = mapFromResponse(emptyResponse({
      terrain: [{ coord: undefined, materialId: "water" }],
    }));

    expect(grid.terrain.flat().every((material) => material === "grass")).toBe(true);
  });

  it("groups building cells by building id at their offset position", () => {
    const grid = mapFromResponse(emptyResponse({
      building: [
        { coord: { x: 0, y: 0, z: 0 }, buildingId: "b1", templateId: "shop" },
        { coord: { x: 1, y: 0, z: 0 }, buildingId: "b1", templateId: "shop" },
        { coord: { x: 2, y: 1, z: 0 }, buildingId: "b2", templateId: "restaurant" },
      ],
    }));

    expect(grid.buildings.size).toBe(2);
    expect(grid.buildings.get("b1")).toEqual({
      templateId: "shop",
      cells: [{ x: 0, y: 0 }, { x: 1, y: 0 }],
    });
    expect(grid.buildings.get("b2")).toEqual({
      templateId: "restaurant",
      cells: [{ x: 2, y: 1 }],
    });
  });

  it("ignores building cells that are not on level z=0", () => {
    const grid = mapFromResponse(emptyResponse({
      building: [
        { coord: { x: 0, y: 0, z: 1 }, buildingId: "b1", templateId: "shop" },
      ],
    }));

    expect(grid.buildings.size).toBe(0);
  });
});
