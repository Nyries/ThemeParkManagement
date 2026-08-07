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
import { placeInfrastructureAt } from "../park/placeInfrastructure";
import { mapFromResponse } from "../park/mapFromResponse";
import { syncVisitorGraphics } from "../park/syncVisitors";

const PARK_ID = "default";

export function Park() {
  const containerRef = useRef<HTMLDivElement>(null);
  const { sendCommand, onWorldState, onMap } = useParkSocket();

  useEffect(() => {
    let cancelled = false;
    let initialized = false;
    let unsubscribeWorldState: (() => void) | null = null;
    const app = new Application();

    onMap().then(async (mapResponse) => {
      if (cancelled) return;

      const { terrain, infrastructure, width, height } = mapFromResponse(mapResponse);
      const cellGraphics: Graphics[][] = Array.from(
        { length: terrain.length },
        () => [],
      );
      const visitorGraphics = new Map<string, Graphics>();

      async function handleCellClick(x: number, y: number) {
        const response = await placeInfrastructureAt(sendCommand, PARK_ID, x, y);
        if (!response.success) {
          console.error("Failed to place infrastructure: ", response.message);
          return;
        }

        infrastructure[y][x] = "path";
        const cell = cellGraphics[y][x];
        cell.clear();
        cell
          .rect(toScreenX(x), toScreenY(y, height), CELL_SIZE, CELL_SIZE)
          .fill(getCellColor(x, y, terrain, infrastructure))
          .stroke({ width: 1, color: 0x000000, alpha: 0.15 });
        cell.eventMode = "none";
        cell.cursor = "default";
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

          if (infrastructure[y][x] === null) {
            cell.eventMode = "static";
            cell.cursor = "pointer";
            cell.on("pointertap", () => handleCellClick(x, y));
          }

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
    <div
      className="flex h-screen w-screen items-center justify-center overflow-hidden bg-neutral-900"
      ref={containerRef}
    />
  );
}
