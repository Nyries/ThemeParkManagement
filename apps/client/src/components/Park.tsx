import { useEffect, useRef } from "react";
import { Application, Graphics, Text } from "pixi.js";
import {
  canvasHeight,
  canvasWidth,
  CELL_SIZE,
  getCellColor,
  toScreenX,
  toScreenY,
} from "../rendering/grid";
import { useParkSocket } from "../hooks/useParkSocket";
import { mapFromResponse } from "../park/mapFromResponse";
import { syncVisitorGraphics } from "../park/syncVisitors";
import type { SelectionInfo } from "../park/selection";
import {
  nextRotation,
  rotateFootprint,
  type PlaceBuilding,
  type ToolState,
} from "../park/tool";
import { Toolbar } from "./Toolbar";
import { SecondaryToolbar, type SecondaryToolbarHandle } from "./SecondaryToolbar";
import {
  applyTerrainAt,
  placeBuildingAt,
  placeInfrastructureAt,
  removeBuildingAt,
  removeInfrastructureAt,
} from "@/park/commands";
import { toast } from "sonner";
import { Rotation } from "@app/shared-types";

const PARK_ID = "default";
const MATERIAL_ID = "grass";
const TEMPLATE_ID = "sit_down_restaurant";
// Stub footprint until the real catalogue (TPM-163) is wired in — matches
// "sit_down_restaurant" in apps/engine/assets/catalog/buildings.json.
const TEMPLATE_FOOTPRINT = [
  { x: 0, y: 0 },
  { x: 1, y: 0 },
  { x: 0, y: 1 },
];

const GHOST_VALID_COLOR = 0x2e6e62;
const GHOST_INVALID_COLOR = 0xdc2626;
const GHOST_ALPHA = 0.45;

// Keyboard shortcuts: 1-4 select a tool, R/Shift+R rotate the building
// ghost, Escape cancels the active tool. All of them must preempt the
// browser's own default behavior for that key.
//
// Matched on event.code (physical key position) rather than event.key: on
// an AZERTY layout the digit row only produces "1".."4" while holding
// Shift, so event.key for an unshifted press is "&"/"é"/'"'/"'" instead —
// event.code stays "Digit1".."Digit4" regardless of layout or Shift state.
const SHORTCUT_CODES = new Set([
  "Digit1",
  "Digit2",
  "Digit3",
  "Digit4",
  "KeyR",
  "Escape",
]);

interface ParkProps {
  tool: ToolState;
  onToolChange: (tool: ToolState) => void;
  onSelectionChange?: (selection: SelectionInfo | null) => void;
}

export function Park({ tool, onToolChange, onSelectionChange }: ParkProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const rotateButtonRef = useRef<SecondaryToolbarHandle>(null);
  const { sendCommand, onWorldState, onMap, isConnected } = useParkSocket();

  // handleCellClick lives inside the map-loading effect below, which must not
  // re-run on every tool change (it would tear down and rebuild the whole
  // PixiJS app). Read the current tool through this ref instead of closing
  // over the `tool` prop directly, so clicks always see the latest tool.
  const toolRef = useRef(tool);
  useEffect(() => {
    toolRef.current = tool;
  }, [tool]);

  // The ghost preview lives inside the map-loading effect too (it needs the
  // PixiJS Graphics instance and the loaded grid data). Expose a way to
  // trigger a redraw from outside that effect whenever the tool changes.
  const updateGhostRef = useRef<() => void>(() => {});
  useEffect(() => {
    updateGhostRef.current();
  }, [tool]);

  useEffect(() => {
    containerRef.current?.focus();
  }, []);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (!SHORTCUT_CODES.has(event.code)) {
        return;
      }
      event.preventDefault();

      const currentTool = toolRef.current;
      switch (event.code) {
        case "Digit1":
          onToolChange({
            ...currentTool,
            mode: currentTool.mode === "terrain" ? null : "terrain",
          });
          break;
        case "Digit2":
          onToolChange({
            ...currentTool,
            mode:
              currentTool.mode === "infrastructure" ? null : "infrastructure",
          });
          break;
        case "Digit3":
          onToolChange({
            ...currentTool,
            mode: currentTool.mode === "building" ? null : "building",
          });
          break;
        case "Digit4":
          onToolChange({
            ...currentTool,
            mode: currentTool.mode === "remove" ? null : "remove",
          });
          break;
        case "KeyR":
          if (currentTool.mode === "building") {
            // Goes through the rotate button's own handler (and its visual
            // "pressed" feedback) rather than updating the tool state
            // directly, so the shortcut behaves like clicking the button.
            rotateButtonRef.current?.triggerRotate(event.shiftKey);
          }
          break;
        case "Escape":
          onToolChange({ ...currentTool, mode: null });
          break;
        default:
          break;
      }
    }

    container.addEventListener("keydown", handleKeyDown);
    return () => container.removeEventListener("keydown", handleKeyDown);
  }, [onToolChange]);

  useEffect(() => {
    // No selectable buildings/employees exist yet (TPM-149) — this only
    // signals the initial neutral state so InspectorPanel has something to render.
    onSelectionChange?.(null);
  }, [onSelectionChange]);

  useEffect(() => {
    let cancelled = false;
    let initialized = false;
    let unsubscribeWorldState: (() => void) | null = null;
    let removePointerLeaveListener: (() => void) | null = null;
    const app = new Application();

    onMap().then(async (mapResponse) => {
      if (cancelled) return;

      const { terrain, infrastructure, width, height } =
        mapFromResponse(mapResponse);
      const buildingGrid: (string | null)[][] = Array.from(
        { length: terrain.length },
        () => Array(terrain[0].length).fill(null),
      );
      const buildings = new Map<string, PlaceBuilding>();
      const cellGraphics: Graphics[][] = Array.from(
        { length: terrain.length },
        () => [],
      );
      const visitorGraphics = new Map<string, Graphics>();
      const ghost = new Graphics();
      const hoveredCellRef: { current: { x: number; y: number } | null } = {
        current: null,
      };

      // A footprint is placeable if every cell it covers is within the grid
      // and free of both an existing building and infrastructure (a
      // building cannot be dropped on top of a path).
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
            !infrastructure[cy][cx]
          );
        });
      }

      function updateGhost() {
        ghost.clear();
        const currentTool = toolRef.current;
        const hovered = hoveredCellRef.current;
        if (currentTool.mode !== "building" || !hovered) {
          return;
        }

        const rotation = currentTool.rotation ?? Rotation.ROTATION_DEG_0;
        const footprint = rotateFootprint(TEMPLATE_FOOTPRINT, rotation);
        const valid = isFootprintValid(hovered.x, hovered.y, footprint);

        const color = valid ? GHOST_VALID_COLOR : GHOST_INVALID_COLOR;
        for (const offset of footprint) {
          const cx = hovered.x + offset.x;
          const cy = hovered.y + offset.y;
          if (cx < 0 || cx >= width || cy < 0 || cy >= height) {
            continue;
          }
          ghost
            .rect(toScreenX(cx), toScreenY(cy, height), CELL_SIZE, CELL_SIZE)
            .fill({ color, alpha: GHOST_ALPHA });
        }
      }
      updateGhostRef.current = updateGhost;

      async function handleCellClick(x: number, y: number) {
        let response;
        const cell = cellGraphics[y][x];

        switch (toolRef.current.mode) {
          case "terrain":
            response = await applyTerrainAt(
              sendCommand,
              PARK_ID,
              MATERIAL_ID,
              x,
              y,
            );
            if (!response.success) {
              toast.error("Failed to apply the terrain", {
                description: response.message,
              });
              console.error("Failed to apply the terrain: ", response.message);
              break;
            }

            terrain[y][x] = MATERIAL_ID;
            cell.clear();
            cell
              .rect(toScreenX(x), toScreenY(y, height), CELL_SIZE, CELL_SIZE)
              .fill(getCellColor(x, y, terrain, infrastructure))
              .stroke({ width: 1, color: 0x000000, alpha: 0.15 });
            cell.cursor = "default";
            break;
          case "infrastructure":
            response = await placeInfrastructureAt(sendCommand, PARK_ID, x, y);
            if (!response.success) {
              console.error(
                "Failed to place infrastructure: ",
                response.message,
              );
              break;
            }

            infrastructure[y][x] = "path";
            cell.clear();
            cell
              .rect(toScreenX(x), toScreenY(y, height), CELL_SIZE, CELL_SIZE)
              .fill(getCellColor(x, y, terrain, infrastructure))
              .stroke({ width: 1, color: 0x000000, alpha: 0.15 });
            cell.cursor = "default";
            break;
          case "building": {
            const rotation = toolRef.current.rotation ?? Rotation.ROTATION_DEG_0;
            const footprint = rotateFootprint(TEMPLATE_FOOTPRINT, rotation);

            if (!isFootprintValid(x, y, footprint)) {
              toast.error("Failed to place the building", {
                description:
                  "Emplacement invalide : chevauche un bâtiment ou une infrastructure existante.",
              });
              break;
            }

            response = await placeBuildingAt(
              sendCommand,
              PARK_ID,
              TEMPLATE_ID,
              x,
              y,
              rotation,
            );
            if (!response.success) {
              toast.error("Failed to place the building", {
                description: response.message,
              });
              console.error("Failed to place the building: ", response.message);
              break;
            }

            const key = `${x},${y}`;
            buildings.set(key, {
              templateId: TEMPLATE_ID,
              origin: { x, y },
              rotation,
              footprint,
            });
            for (const offset of footprint) {
              const cx = x + offset.x;
              const cy = y + offset.y;
              buildingGrid[cy][cx] = key;
              const occupiedCell = cellGraphics[cy][cx];
              occupiedCell.clear();
              occupiedCell
                .rect(toScreenX(cx), toScreenY(cy, height), CELL_SIZE, CELL_SIZE)
                .fill(getCellColor(cx, cy, terrain, infrastructure, true))
                .stroke({ width: 1, color: 0x000000, alpha: 0.15 });
              occupiedCell.cursor = "default";
            }
            updateGhost();
            break;
          }
          case "remove": {
            const buildingKey = buildingGrid[y][x];
            if (buildingKey) {
              const building = buildings.get(buildingKey)!;
              if (!window.confirm("Démolir ce bâtiment ?")) {
                break;
              }
              response = await removeBuildingAt(
                sendCommand,
                PARK_ID,
                building.origin.x,
                building.origin.y,
              );
              if (!response.success) {
                toast.error("Failed to remove the building", {
                  description: response.message,
                });
                break;
              }

              buildings.delete(buildingKey);
              for (const offset of building.footprint) {
                const cx = building.origin.x + offset.x;
                const cy = building.origin.y + offset.y;
                buildingGrid[cy][cx] = null;
                const clearedCell = cellGraphics[cy][cx];
                clearedCell.clear();
                clearedCell
                  .rect(toScreenX(cx), toScreenY(cy, height), CELL_SIZE, CELL_SIZE)
                  .fill(getCellColor(cx, cy, terrain, infrastructure))
                  .stroke({ width: 1, color: 0x000000, alpha: 0.15 });
                clearedCell.eventMode = "static";
                clearedCell.cursor = "pointer";
                clearedCell.on("pointertap", () => handleCellClick(cx, cy));
              }
            } else if (infrastructure[y][x]) {
              response = await removeInfrastructureAt(sendCommand, PARK_ID, x, y);
              if (!response.success) {
                toast.error("Failed to remove the infrastructure", {
                  description: response.message,
                });
                break;
              }

              infrastructure[y][x] = null;
              cell.clear();
              cell
                .rect(toScreenX(x), toScreenY(y, height), CELL_SIZE, CELL_SIZE)
                .fill(getCellColor(x, y, terrain, infrastructure))
                .stroke({ width: 1, color: 0x000000, alpha: 0.15 });
              cell.eventMode = "static";
              cell.cursor = "pointer";
              cell.on("pointertap", () => handleCellClick(x, y));
            }
            break;
          }
          default:
            break;
        }
      }

      await app.init({
        width: canvasWidth(width),
        height: canvasHeight(height),
        background: 0x000000,
      });

      if (cancelled) return;
      initialized = true;

      containerRef.current?.appendChild(app.canvas);

      for (let y = 0; y < terrain.length; y++) {
        for (let x = 0; x < terrain[y].length; x++) {
          const color = getCellColor(x, y, terrain, infrastructure);
          const cell = new Graphics()
            .rect(toScreenX(x), toScreenY(y, height), CELL_SIZE, CELL_SIZE)
            .fill(color)
            .stroke({ width: 1, color: 0x000000, alpha: 0.15 });

          // Every cell stays interactive regardless of what's already on it —
          // handleCellClick dispatches by the *current* tool (via toolRef),
          // so an occupied cell must remain clickable to be removable later.
          cell.eventMode = "static";
          cell.cursor = "pointer";
          cell.on("pointertap", () => handleCellClick(x, y));
          cell.on("pointerover", () => {
            hoveredCellRef.current = { x, y };
            updateGhost();
          });

          cellGraphics[y][x] = cell;
          app.stage.addChild(cell);
        }
      }
      for (let x = 0; x < terrain[0].length; x++) {
        const label = new Text({
          text: String(x),
          style: { fontSize: 10, fill: 0xffffff },
        });
        label.x = toScreenX(x);
        label.y = height * CELL_SIZE;
        app.stage.addChild(label);
      }
      for (let y = 0; y < terrain.length; y++) {
        const label = new Text({
          text: String(y),
          style: { fontSize: 10, fill: 0xffffff },
        });
        label.x = 0;
        label.y = toScreenY(y, height);
        app.stage.addChild(label);
      }

      app.stage.addChild(ghost);

      const containerEl = containerRef.current;
      function handlePointerLeave() {
        hoveredCellRef.current = null;
        updateGhost();
      }
      containerEl?.addEventListener("pointerleave", handlePointerLeave);
      removePointerLeaveListener = () =>
        containerEl?.removeEventListener("pointerleave", handlePointerLeave);

      unsubscribeWorldState = onWorldState((state) => {
        syncVisitorGraphics(app.stage, visitorGraphics, state.visitors, height);
      });
    });

    return () => {
      cancelled = true;
      unsubscribeWorldState?.();
      removePointerLeaveListener?.();
      if (initialized) {
        app.destroy(true, { children: true });
      }
    };
  }, [sendCommand, onWorldState, onMap]);

  return (
    // Toolbar buttons are real <button> elements: clicking one shifts DOM
    // focus there (browser default on mousedown), which would silently kill
    // the keyboard shortcuts since they only listen on the canvas container.
    // Reclaim focus after any click inside the Park so shortcuts keep working.
    <div
      className="relative h-full w-full"
      onClick={() => containerRef.current?.focus()}
    >
      {!isConnected && (
        <div className="absolute inset-x-0 top-0 z-10 bg-destructive/90 px-3 py-1.5 text-center text-xs text-white">
          Connexion perdue — reconnexion en cours…
        </div>
      )}
      <div className="absolute right-4 top-4 z-10 flex flex-col items-center gap-1">
        <Toolbar
          mode={tool.mode}
          onModeChange={(mode) => onToolChange({ ...tool, mode })}
        />
        <SecondaryToolbar
          ref={rotateButtonRef}
          mode={tool.mode}
          onRotate={(reverse) =>
            onToolChange({
              ...tool,
              rotation: nextRotation(
                tool.rotation ?? Rotation.ROTATION_DEG_0,
                reverse,
              ),
            })
          }
        />
      </div>
      <div
        className="flex h-full w-full items-center justify-center overflow-hidden bg-neutral-900 outline-none"
        ref={containerRef}
        tabIndex={0}
      />
    </div>
  );
}
