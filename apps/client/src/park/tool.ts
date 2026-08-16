import type { Rotation } from "@app/shared-types";

export type ToolMode = "terrain" | "infrastructure" | "building" | "remove" | null;
export interface ToolState {
    mode: ToolMode;
    selectedMaterialId?: string;
    selectedBuildingId?: string;
}

export interface PlaceBuilding {
    templateId: string;
    origin: { x: number, y: number};
    rotation: Rotation;
    footprint: { x: number, y: number}[];
}