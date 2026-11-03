import { useEffect, useRef } from "react";
import * as PIXI from "pixi.js";

const TILE_SIZE = 32;
const grid_width = 30;
const grid_height = 30;

export function TestEditor() {
  const canvasRef = useRef<HTMLDivElement | null>(null);
  const appRef = useRef<PIXI.Application | null>(null);
  const worldRef = useRef<PIXI.Container | null>(null);

  useEffect(() => {
    let app: PIXI.Application | null = null;
    let isMounted = true; // Sécurité si le composant se démonte pendant le chargement

    const initGame = async () => {
      app = new PIXI.Application();

      await app.init({
        resizeTo: window,
        backgroundColor: 0xf7f4ee,
        antialias: true,
      });

      if (!isMounted) {
        app.destroy(true);
        return;
      }

      if (canvasRef.current) {
        canvasRef.current.appendChild(app.canvas);
      }
      appRef.current = app;

      const world = new PIXI.Container();
      app.stage.addChild(world);
      worldRef.current = world;

      try {
        const grassTexture = await PIXI.Assets.load(
          "/assets/textures/Grass-1.png",
        );

        for (let x = 0; x < grid_width; x++) {
          for (let y = 0; y < grid_height; y++) {
            const tile = new PIXI.Sprite(grassTexture);
            tile.width = TILE_SIZE;
            tile.height = TILE_SIZE;
            tile.position.set(x * TILE_SIZE, y * TILE_SIZE);
            world.addChild(tile);

            //TODO: optional border for the grid tile
            const border = new PIXI.Graphics();
            border.rect(0, 0, TILE_SIZE, TILE_SIZE);
            border.stroke({
              width: 1,
              color: 0x000000,
              alpha: 0.12,
            });
            border.position.set(x * TILE_SIZE, y * TILE_SIZE);

            world.addChild(border);
          }
        }
      } catch (error) {
        console.error(
          "Erreur lors du chargement de la texture d'herbe :",
          error,
        );
      }

      if (canvasRef.current) {
        const canvas = canvasRef.current;
        //Zoom
        canvas.addEventListener("wheel", (e) => {
          e.preventDefault();
          const zoomFactor = e.deltaY < 0 ? 1.1 : 0.9;

          const newScale = Math.max(
            0.5,
            Math.min(3.0, world.scale.x * zoomFactor),
          );
          world.scale.set(newScale);
        });

        //Panning
        let isDragging = false;
        let startX = 0;
        let startY = 0;
        let startWorldX = 0;
        let startWorldY = 0;

        canvas.addEventListener("mousedown", (e) => {
          isDragging = true;
          startX = e.clientX;
          startY = e.clientY;
          startWorldX = world.x;
          startWorldY = world.y;
        });

        canvas.addEventListener("mousemove", (e) => {
          if (!isDragging) return;

          const dx = e.clientX - startX;
          const dy = e.clientY - startY;

          world.x = startWorldX + dx;
          world.y = startWorldY + dy;
        });

        canvas.addEventListener("mouseup", () => {
          isDragging = false;
        });
        canvas.addEventListener("mouseleave", () => {
          isDragging = false;
        });
      }
    };

    initGame();

    return () => {
      isMounted = false;
      if (app) {
        try {
          if (app.canvas && app.canvas.parentNode) {
            app.canvas.parentNode.removeChild(app.canvas);
          }
          app.destroy(true);
        } catch (e) {
          console.warn("Error when destroying Pixi", e);
        }
      }
    };
  }, []);

  return (
    <div
      ref={canvasRef}
      className="overflow-hidden cursor-grab active:cursor-grabbing"
    />
  );
}
