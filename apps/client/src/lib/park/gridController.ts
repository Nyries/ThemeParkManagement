import type { Graphics } from "pixi.js";
import type { CommandRequest, CommandResponse } from "@app/shared-types";
import { Rotation } from "@app/shared-types";
import { toast } from "sonner";
import {
  CELL_SIZE,
  getCellColor,
  toScreenX,
  toScreenY,
  type InfrastructureKind,
  type TerrainMaterial,
} from "../../rendering/grid";
import { DEFAULT_BUILDING_ID, findBuildingTemplate } from "./buildingCatalog";
import {
  applyTerrainAt,
  placeBuildingAt,
  placeInfrastructureAt,
  removeBuildingAt,
  removeInfrastructureAt,
} from "./commands";
import { DEFAULT_MATERIAL_ID, isMaterialBuildable } from "./materials";
import type { ParkBuilding } from "./mapFromResponse";
import {
  rotateFootprint,
  type PlaceBuilding,
  type ToolState,
} from "../../types/park/tool";

const GHOST_VALID_COLOR = 0x2e6e62;
const GHOST_INVALID_COLOR = 0xdc2626;
const GHOST_ALPHA = 0.45;

interface GridControllerOptions {
  parkId: string;
  sendCommand: (request: CommandRequest) => Promise<CommandResponse>;
  toolRef: { current: ToolState };
  cellGraphics: Graphics[][];
  terrain: TerrainMaterial[][];
  infrastructure: (InfrastructureKind | null)[][];
  buildings: Map<string, ParkBuilding>;
  ghost: Graphics;
  width: number;
  height: number;
}

// Owns every cell mutation for the Park canvas — placing/removing terrain,
// infrastructure and buildings, the building ghost preview, drag-tracing
// across a stroke, and the cursor style — so Park.tsx only has to wire it up
// to PixiJS events and React state. All of `terrain`/`infrastructure`/
// `cellGraphics` are mutated in place; the caller owns their lifetime.
export function createGridController({
  parkId,
  sendCommand,
  toolRef,
  cellGraphics,
  terrain,
  infrastructure,
  buildings: initialBuildings,
  ghost,
  width,
  height,
}: GridControllerOptions) {
  const buildingGrid: (string | null)[][] = Array.from(
    { length: terrain.length },
    () => Array(terrain[0].length).fill(null),
  );
  const buildings = new Map<string, PlaceBuilding>();

  // Buildings restored from the server on load: no origin/rotation is
  // persisted engine-side (TPM-175 — out of scope, current colour fill
  // doesn't depend on orientation), so an arbitrary cell of the footprint is
  // used as origin and the footprint offsets are derived from it directly.
  for (const building of initialBuildings.values()) {
    const origin = building.cells[0];
    const footprint = building.cells.map((cell) => ({
      x: cell.x - origin.x,
      y: cell.y - origin.y,
    }));
    const key = `${origin.x},${origin.y}`;
    buildings.set(key, {
      templateId: building.templateId,
      origin,
      rotation: Rotation.ROTATION_DEG_0,
      footprint,
    });
    for (const cell of building.cells) {
      buildingGrid[cell.y][cell.x] = key;
    }
  }

  let hoveredCell: { x: number; y: number } | null = null;
  // Drag-tracing state, shared by ApplyTerrain, PlaceInfrastructure and
  // Remove (only one tool is ever active at a time): a pointerdown on a
  // cell starts a "stroke", pointerover on subsequent cells while the
  // pointer is still down extends it, and a global pointerup ends it.
  // Visited cells are tracked per-stroke so re-entering a cell never
  // sends a duplicate command (or, for a building, asks for confirmation
  // more than once).
  let isDragging = false;
  let draggedCells = new Set<string>();

  function redrawCell(x: number, y: number, hasBuilding = false) {
    const cell = cellGraphics[y][x];
    cell.clear();
    cell
      .rect(toScreenX(x), toScreenY(y, height), CELL_SIZE, CELL_SIZE)
      .fill(getCellColor(x, y, terrain, infrastructure, hasBuilding))
      .stroke({ width: 1, color: 0x000000, alpha: 0.15 });
  }

  // A footprint is placeable if every cell it covers is within the grid,
  // free of both an existing building and infrastructure (a building
  // cannot be dropped on top of a path), and stands on buildable terrain
  // (e.g. not water).
  function isFootprintValid(
    originX: number,
    originY: number,
    footprint: { x: number; y: number }[],
  ): boolean {
    return footprint.every((offset) => {
      const cx = originX + offset.x;
      const cy = originY + offset.y;
      return (
        cx >= 0 &&
        cx < width &&
        cy >= 0 &&
        cy < height &&
        !buildingGrid[cy][cx] &&
        !infrastructure[cy][cx] &&
        isMaterialBuildable(terrain[cy][cx])
      );
    });
  }

  function updateGhost() {
    ghost.clear();
    const currentTool = toolRef.current;
    if (currentTool.mode !== "building" || !hoveredCell) {
      return;
    }

    const rotation = currentTool.rotation ?? Rotation.ROTATION_DEG_0;
    const template = findBuildingTemplate(
      currentTool.selectedBuildingId ?? DEFAULT_BUILDING_ID,
    );
    const footprint = rotateFootprint(template.footprint, rotation);
    const valid = isFootprintValid(hoveredCell.x, hoveredCell.y, footprint);

    const color = valid ? GHOST_VALID_COLOR : GHOST_INVALID_COLOR;
    for (const offset of footprint) {
      const cx = hoveredCell.x + offset.x;
      const cy = hoveredCell.y + offset.y;
      if (cx < 0 || cx >= width || cy < 0 || cy >= height) {
        continue;
      }
      ghost
        .rect(toScreenX(cx), toScreenY(cy, height), CELL_SIZE, CELL_SIZE)
        .fill({ color, alpha: GHOST_ALPHA });
    }
  }

  function updateCursors() {
    const cursor = toolRef.current.mode ? "pointer" : "default";
    for (const row of cellGraphics) {
      for (const cell of row) {
        cell.cursor = cursor;
      }
    }
  }

  // Drives both a plain click (a 1-cell stroke) and drag-tracing
  // (pointerdown + pointerover while dragging) for ApplyTerrain.
  async function applyTerrainAtCell(x: number, y: number) {
    const key = `${x},${y}`;
    if (draggedCells.has(key)) {
      return;
    }
    draggedCells.add(key);

    const materialId = toolRef.current.selectedMaterialId ?? DEFAULT_MATERIAL_ID;

    const response = await applyTerrainAt(sendCommand, parkId, materialId, x, y);
    if (!response.success) {
      toast.error("Failed to apply the terrain", {
        description: response.message,
      });
      console.error("Failed to apply the terrain: ", response.message);
      return;
    }

    terrain[y][x] = materialId;
    redrawCell(x, y);
  }

  // Drives both a plain click (a 1-cell stroke) and drag-tracing
  // (pointerdown + pointerover while dragging) for PlaceInfrastructure.
  async function placeInfrastructureAtCell(x: number, y: number) {
    const key = `${x},${y}`;
    if (draggedCells.has(key)) {
      return;
    }
    draggedCells.add(key);

    if (infrastructure[y][x] !== null) {
      return;
    }

    const response = await placeInfrastructureAt(sendCommand, parkId, x, y);
    if (!response.success) {
      console.error("Failed to place infrastructure: ", response.message);
      return;
    }

    infrastructure[y][x] = "path";
    redrawCell(x, y);
  }

  // Drives both a plain click and drag-tracing for Remove. A building
  // requires confirmation (once per building, even if the drag passes
  // through several of its cells); an infrastructure segment does not.
  async function removeAtCell(x: number, y: number) {
    const key = `${x},${y}`;
    if (draggedCells.has(key)) {
      return;
    }

    const buildingKey = buildingGrid[y][x];
    if (buildingKey) {
      const building = buildings.get(buildingKey)!;
      // Mark every footprint cell visited up front so dragging across the
      // same building never asks for confirmation twice in one stroke.
      for (const offset of building.footprint) {
        draggedCells.add(
          `${building.origin.x + offset.x},${building.origin.y + offset.y}`,
        );
      }

      if (!window.confirm("Démolir ce bâtiment ?")) {
        return;
      }
      const response = await removeBuildingAt(
        sendCommand,
        parkId,
        building.origin.x,
        building.origin.y,
      );
      if (!response.success) {
        toast.error("Failed to remove the building", {
          description: response.message,
        });
        return;
      }

      buildings.delete(buildingKey);
      for (const offset of building.footprint) {
        const cx = building.origin.x + offset.x;
        const cy = building.origin.y + offset.y;
        buildingGrid[cy][cx] = null;
        redrawCell(cx, cy);
      }
      return;
    }

    draggedCells.add(key);
    if (!infrastructure[y][x]) {
      return;
    }

    const response = await removeInfrastructureAt(sendCommand, parkId, x, y);
    if (!response.success) {
      toast.error("Failed to remove the infrastructure", {
        description: response.message,
      });
      return;
    }

    infrastructure[y][x] = null;
    redrawCell(x, y);
  }

  async function placeBuildingAtCell(x: number, y: number) {
    const rotation = toolRef.current.rotation ?? Rotation.ROTATION_DEG_0;
    const template = findBuildingTemplate(
      toolRef.current.selectedBuildingId ?? DEFAULT_BUILDING_ID,
    );
    const footprint = rotateFootprint(template.footprint, rotation);

    if (!isFootprintValid(x, y, footprint)) {
      toast.error("Failed to place the building", {
        description:
          "Emplacement invalide : chevauche un bâtiment, une infrastructure existante, ou le terrain ne permet pas de construire ici.",
      });
      return;
    }

    const response = await placeBuildingAt(
      sendCommand,
      parkId,
      template.templateId,
      x,
      y,
      rotation,
    );
    if (!response.success) {
      toast.error("Failed to place the building", {
        description: response.message,
      });
      console.error("Failed to place the building: ", response.message);
      return;
    }

    const key = `${x},${y}`;
    buildings.set(key, {
      templateId: template.templateId,
      origin: { x, y },
      rotation,
      footprint,
    });
    for (const offset of footprint) {
      const cx = x + offset.x;
      const cy = y + offset.y;
      buildingGrid[cy][cx] = key;
      redrawCell(cx, cy, true);
    }
    updateGhost();
  }

  // Picks which drag-tracing action (if any) applies to the currently
  // active tool, shared by the pointerdown (stroke start) and pointerover
  // (stroke continuation) handlers in Park.tsx.
  function dragActionAt(x: number, y: number): (() => Promise<void>) | null {
    switch (toolRef.current.mode) {
      case "terrain":
        return () => applyTerrainAtCell(x, y);
      case "infrastructure":
        return () => placeInfrastructureAtCell(x, y);
      case "remove":
        return () => removeAtCell(x, y);
      default:
        return null;
    }
  }

  async function handleCellClick(x: number, y: number) {
    // "terrain"/"infrastructure"/"remove" are handled by dragActionAt,
    // driven from pointerdown/pointerover for drag-tracing — not from this
    // click dispatcher.
    if (toolRef.current.mode === "building") {
      await placeBuildingAtCell(x, y);
    }
  }

  function handlePointerOver(x: number, y: number) {
    hoveredCell = { x, y };
    updateGhost();
    if (isDragging) {
      void dragActionAt(x, y)?.();
    }
  }

  function handlePointerDown(x: number, y: number) {
    const action = dragActionAt(x, y);
    if (!action) {
      return;
    }
    isDragging = true;
    draggedCells = new Set();
    void action();
  }

  function handlePointerUp() {
    isDragging = false;
  }

  function handlePointerLeave() {
    hoveredCell = null;
    updateGhost();
  }

  return {
    buildingGrid,
    handleCellClick,
    handlePointerOver,
    handlePointerDown,
    handlePointerUp,
    handlePointerLeave,
    updateGhost,
    updateCursors,
  };
}

export type GridController = ReturnType<typeof createGridController>;
