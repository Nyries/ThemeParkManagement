import { useEffect, useRef } from "react";
import {
  generateMockInfrastructure,
  generateMockTerrain,
  type InfrastructureKind,
  type TerrainMaterial,
} from "../mocks/mockMap";
import { Application, Graphics } from "pixi.js";

const CELL_SIZE = 16;

const TERRAIN_COLORS: Record<TerrainMaterial, number> = {
  grass: 0x4caf50,
  water: 0x2196f3,
};

const INFRASTRUCTURE_COLORS: Record<InfrastructureKind, number> = {
  path: 0xd7b98e,
  ramp: 0xd7b98e,
  stairs: 0xd7b98e,
};

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

    app.init({ width: 800, height: 480, background: 0x000000 }).then(() => {
      if (cancelled) return;

      containerRef.current?.appendChild(app.canvas);

      for (let y = 0; y < terrain.length; y++) {
        for (let x = 0; x < terrain[y].length; x++) {
          const color = getCellColor(x, y, terrain, infrastructure);
          const cell = new Graphics()
            .rect(x * CELL_SIZE, y * CELL_SIZE, CELL_SIZE, CELL_SIZE)
            .fill(color);
          app.stage.addChild(cell);
        }
      }
    });
  }, []);

  return <div ref={containerRef} />;
}
