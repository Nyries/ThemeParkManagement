import { useEffect, useRef } from "react";
import {
  generateMockInfrastructure,
  generateMockTerrain,
  HEIGHT as GRID_HEIGHT
} from "../mocks/mockMap";
import { Application, Graphics, Text } from "pixi.js";
import { CANVAS_HEIGHT, CANVAS_WIDTH, CELL_SIZE, getCellColor, toScreenX, toScreenY } from "../rendering/grid";

export function Park() {
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
                toScreenX(x),
                toScreenY(y),
                CELL_SIZE,
                CELL_SIZE,
              )
              .fill(color)
              .stroke({ width: 1, color: 0x000000, alpha: 0.15 });
            app.stage.addChild(cell);
          }
        }
        for (let x = 0; x < terrain[0].length; x++) {
          const label = new Text({
            text: String(x),
            style: { fontSize: 10, fill: 0xffffff },
          });
          label.x = toScreenX(x);
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
