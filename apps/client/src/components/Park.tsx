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
import type { PlaceBuilding, ToolState } from "../park/tool";
import { Toolbar } from "./Toolbar";
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
  { x: 1, y: 1 },
];

interface ParkProps {
  tool: ToolState;
  onToolChange: (tool: ToolState) => void;
  onSelectionChange?: (selection: SelectionInfo | null) => void;
}

export function Park({ tool, onToolChange, onSelectionChange }: ParkProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const { sendCommand, onWorldState, onMap, isConnected } = useParkSocket();

  // handleCellClick lives inside the map-loading effect below, which must not
  // re-run on every tool change (it would tear down and rebuild the whole
  // PixiJS app). Read the current tool through this ref instead of closing
  // over the `tool` prop directly, so clicks always see the latest tool.
  const toolRef = useRef(tool);
  useEffect(() => {
    toolRef.current = tool;
  }, [tool]);

  useEffect(() => {
    // No selectable buildings/employees exist yet (TPM-149) — this only
    // signals the initial neutral state so InspectorPanel has something to render.
    onSelectionChange?.(null);
  }, [onSelectionChange]);

  useEffect(() => {
    let cancelled = false;
    let initialized = false;
    let unsubscribeWorldState: (() => void) | null = null;
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
            response = await placeBuildingAt(
              sendCommand,
              PARK_ID,
              TEMPLATE_ID,
              x,
              y,
              Rotation.ROTATION_DEG_0,
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
              rotation: Rotation.ROTATION_DEG_0,
              footprint: TEMPLATE_FOOTPRINT,
            });
            for (const offset of TEMPLATE_FOOTPRINT) {
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

      unsubscribeWorldState = onWorldState((state) => {
        syncVisitorGraphics(app.stage, visitorGraphics, state.visitors, height);
      });
    });

    return () => {
      cancelled = true;
      unsubscribeWorldState?.();
      if (initialized) {
        app.destroy(true, { children: true });
      }
    };
  }, [sendCommand, onWorldState, onMap]);

  return (
    <div className="relative h-full w-full">
      {!isConnected && (
        <div className="absolute inset-x-0 top-0 z-10 bg-destructive/90 px-3 py-1.5 text-center text-xs text-white">
          Connexion perdue — reconnexion en cours…
        </div>
      )}
      <Toolbar
        mode={tool.mode}
        onModeChange={(mode) => onToolChange({ ...tool, mode })}
      />
      <div
        className="flex h-full w-full items-center justify-center overflow-hidden bg-neutral-900"
        ref={containerRef}
      />
    </div>
  );
}
