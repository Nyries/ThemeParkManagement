import type { MapResponse } from "@app/shared-types";
import { InfrastructureKind as ProtoInfrastructureKind } from "@app/shared-types";
import type { InfrastructureKind, TerrainMaterial } from "../../rendering/grid";

export interface ParkBuilding {
  templateId: string;
  cells: { x: number; y: number }[];
}

export interface ParkGrid {
  terrain: TerrainMaterial[][];
  infrastructure: (InfrastructureKind | null)[][];
  buildings: Map<string, ParkBuilding>;
  width: number;
  height: number;
}

function toClientInfrastructureKind(kind: ProtoInfrastructureKind): InfrastructureKind | null {
  switch (kind) {
    case ProtoInfrastructureKind.INFRASTRUCTURE_KIND_PATH:
      return "path";
    case ProtoInfrastructureKind.INFRASTRUCTURE_KIND_RAMP:
      return "ramp";
    case ProtoInfrastructureKind.INFRASTRUCTURE_KIND_STAIRS:
      return "stairs";
    default:
      return null;
  }
}

export function mapFromResponse(response: MapResponse): ParkGrid {
  const width = response.maxX - response.minX + 1;
  const height = response.maxY - response.minY + 1;

  const terrain: TerrainMaterial[][] = Array.from({ length: height }, () =>
    Array.from({ length: width }, () => "grass" as TerrainMaterial),
  );
  const infrastructure: (InfrastructureKind | null)[][] = Array.from(
    { length: height },
    () => Array.from({ length: width }, () => null),
  );

  for (const cell of response.terrain) {
    if (!cell.coord || cell.coord.z !== 0) continue;
    const x = cell.coord.x - response.minX;
    const y = cell.coord.y - response.minY;
    terrain[y][x] = cell.materialId as TerrainMaterial;
  }

  for (const cell of response.infrastructure) {
    if (!cell.coord || cell.coord.z !== 0) continue;
    const x = cell.coord.x - response.minX;
    const y = cell.coord.y - response.minY;
    infrastructure[y][x] = toClientInfrastructureKind(cell.kind);
  }

  const buildings = new Map<string, ParkBuilding>();
  for (const cell of response.building) {
    if (!cell.coord || cell.coord.z !== 0) continue;
    const x = cell.coord.x - response.minX;
    const y = cell.coord.y - response.minY;
    const building = buildings.get(cell.buildingId);
    if (building) {
      building.cells.push({ x, y });
    } else {
      buildings.set(cell.buildingId, {
        templateId: cell.templateId,
        cells: [{ x, y }],
      });
    }
  }

  return { terrain, infrastructure, buildings, width, height };
}
