import { TERRAIN_COLORS, type TerrainMaterial } from "../rendering/grid";

export interface MaterialOption {
  id: TerrainMaterial;
  label: string;
}

export const TERRAIN_MATERIALS: MaterialOption[] = [
  { id: "grass", label: "Herbe" },
  { id: "water", label: "Eau" },
];

export const DEFAULT_MATERIAL_ID: TerrainMaterial = "grass";

export function materialColor(id: TerrainMaterial): string {
  return `#${TERRAIN_COLORS[id].toString(16).padStart(6, "0")}`;
}

// Buildings can only stand on solid ground — water isn't a valid foundation.
const BUILDABLE_MATERIALS: ReadonlySet<TerrainMaterial> = new Set(["grass"]);

export function isMaterialBuildable(material: TerrainMaterial): boolean {
  return BUILDABLE_MATERIALS.has(material);
}
