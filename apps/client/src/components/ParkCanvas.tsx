import { useEffect, useRef } from "react";
import {
  generateMockInfrastructure,
  generateMockTerrain,
  type InfrastructureKind,
  type TerrainMaterial,
} from "../mocks/mockMap";
import { Application, Graphics, Text } from "pixi.js";

const CELL_SIZE = 16;
const MARGIN_SIZE = 20;

const GRID_WIDTH = 50;
const GRID_HEIGHT = 30;

const CANVAS_WIDTH = MARGIN_SIZE + GRID_WIDTH * CELL_SIZE;
const CANVAS_HEIGHT = MARGIN_SIZE + GRID_HEIGHT * CELL_SIZE;

const TERRAIN_COLORS: Record<TerrainMaterial, number> = {
  grass: 0x4caf50,
  water: 0x2196f3,
};

const INFRASTRUCTURE_COLORS: Record<InfrastructureKind, number> = {
  path: 0xd7b98e,
  ramp: 0xd7b98e,
  stairs: 0xd7b98e,
};

function toScreenY(y: number): number {
  return (GRID_HEIGHT - 1 - y) * CELL_SIZE;
}

function getCellColor(
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

export function ParkCanvas() {
  const terrain = generateMockTerrain();
  const infrastructure = generateMockInfrastructure();
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const app = new Application();
    let cancelled = false;
    let initialized = false;

    app
      .init({
        width: CANVAS_WIDTH,
        height: CANVAS_HEIGHT,
        background: 0x000000,
      })
      .then(() => {
        if (cancelled) return;
        initialized = true;

        containerRef.current?.appendChild(app.canvas);

        for (let y = 0; y < terrain.length; y++) {
          for (let x = 0; x < terrain[y].length; x++) {
            const color = getCellColor(x, y, terrain, infrastructure);
            const cell = new Graphics()
              .rect(
                x * CELL_SIZE + MARGIN_SIZE,
                toScreenY(y),
                CELL_SIZE,
                CELL_SIZE,
              )
              .fill(color)
              .stroke({ width: 1, color: 0x000000, alpha: 0.3 });
            app.stage.addChild(cell);
          }
        }
        for (let x = 0; x < terrain[0].length; x++) {
          const label = new Text({
            text: String(x),
            style: { fontSize: 10, fill: 0xffffff },
          });
          label.x = MARGIN_SIZE + x * CELL_SIZE;
          label.y = GRID_HEIGHT * CELL_SIZE;
          app.stage.addChild(label);
        }
        for (let y = 0; y < terrain.length; y++) {
          const label = new Text({
            text: String(y),
            style: { fontSize: 10, fill: 0xffffff },
          });
          label.x = 0;
          label.y = toScreenY(y);
          app.stage.addChild(label);
        }
      });
    return () => {
      cancelled = true;
      if (initialized) {
        app.destroy(true, { children: true });
      }
    };
  }, []);

  return (
    <div
      className="flex h-screen w-screen items-center justify-center overflow-hidden bg-neutral-900"
      ref={containerRef}
    />
  );
}
