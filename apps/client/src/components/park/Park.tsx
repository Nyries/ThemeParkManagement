import { useEffect, useRef, useState } from "react";
import { Application, Container, Graphics, Text } from "pixi.js";
import {
  CELL_SIZE,
  getCellColor,
  toScreenX,
  toScreenY,
} from "../../rendering/grid";
import { useParkSocket } from "../../hooks/park/useParkSocket";
import { useParkKeyboardShortcuts } from "../../hooks/park/useParkKeyboardShortcuts";
import { createGridController } from "../../lib/park/gridController";
import { mapFromResponse } from "../../lib/park/mapFromResponse";
import { syncVisitorGraphics } from "../../lib/park/syncVisitors";
import type { SelectionInfo } from "../../types/park/selection";
import { nextRotation, type ToolState } from "../../types/park/tool";
import { Toolbar } from "./Toolbar";
import {
  SecondaryToolbar,
  type SecondaryToolbarHandle,
} from "./SecondaryToolbar";
import { ParkTopBar } from "../shell/ParkTopBar";
import { InspectorPanel } from "../shell/InspectorPanel";
import { Rotation } from "@app/shared-types";

const PARK_ID = "default";

interface ParkProps {
  tool: ToolState;
  onToolChange: (tool: ToolState) => void;
}

export function Park({ tool, onToolChange }: ParkProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const rotateButtonRef = useRef<SecondaryToolbarHandle>(null);
  const { sendCommand, onWorldState, onMap, isConnected } = useParkSocket();
  const [balance, setBalance] = useState(0);
  const [visitorCount, setVisitorCount] = useState(0);
  const [selection] = useState<SelectionInfo | null>(null);

  // handleCellClick lives inside the map-loading effect below, which must not
  // re-run on every tool change (it would tear down and rebuild the whole
  // PixiJS app). Read the current tool through this ref instead of closing
  // over the `tool` prop directly, so clicks always see the latest tool.
  const toolRef = useRef(tool);
  useEffect(() => {
    toolRef.current = tool;
  }, [tool]);

  useParkKeyboardShortcuts({
    containerRef,
    toolRef,
    onToolChange,
    rotateButtonRef,
  });

  // The grid controller (ghost preview + cursor style) lives inside the
  // map-loading effect too (it needs the PixiJS Graphics instances and the
  // loaded grid data). These expose a way to trigger a redraw from outside
  // that effect whenever the tool changes.
  const updateGhostRef = useRef<() => void>(() => {});
  const updateCursorsRef = useRef<() => void>(() => {});
  useEffect(() => {
    updateGhostRef.current();
    updateCursorsRef.current();
  }, [tool]);

  useEffect(() => {
    containerRef.current?.focus();
  }, []);

  useEffect(() => {
    return onWorldState((state) => {
      setBalance(state.balance);
      setVisitorCount(state.visitors.length);
    });
  }, [onWorldState]);

  useEffect(() => {
    let cancelled = false;
    let initialized = false;
    let unsubscribeWorldState: (() => void) | null = null;
    let removePointerLeaveListener: (() => void) | null = null;
    let removePointerUpListener: (() => void) | null = null;
    const app = new Application();

    onMap().then(async (mapResponse) => {
      if (cancelled) return;

      const { terrain, infrastructure, buildings, width, height } =
        mapFromResponse(mapResponse);
      const cellGraphics: Graphics[][] = Array.from(
        { length: terrain.length },
        () => [],
      );
      const cameraContainer = new Container();
      app.stage.addChild(cameraContainer);
      const visitorGraphics = new Map<string, Graphics>();
      const ghost = new Graphics();

      const controller = createGridController({
        parkId: PARK_ID,
        sendCommand,
        toolRef,
        cellGraphics,
        terrain,
        infrastructure,
        buildings,
        ghost,
        width,
        height,
      });
      updateGhostRef.current = controller.updateGhost;
      updateCursorsRef.current = controller.updateCursors;

      const viewportWidth = containerRef.current?.clientWidth ? containerRef.current?.clientWidth: 600 
      const viewportHeight = containerRef.current?.clientHeight ? containerRef.current?.clientHeight: 800;
      await app.init({
        width: viewportWidth,
        height: viewportHeight,
        background: 0x000000,
      });

      if (cancelled) return;
      initialized = true;

      containerRef.current?.appendChild(app.canvas);

      for (let y = 0; y < terrain.length; y++) {
        for (let x = 0; x < terrain[y].length; x++) {
          const color = getCellColor(
            x,
            y,
            terrain,
            infrastructure,
            Boolean(controller.buildingGrid[y][x]),
          );
          const cell = new Graphics()
            .rect(toScreenX(x), toScreenY(y, height), CELL_SIZE, CELL_SIZE)
            .fill(color)
            .stroke({ width: 1, color: 0x000000, alpha: 0.15 });

          // Every cell stays interactive regardless of what's already on it —
          // handleCellClick dispatches by the *current* tool (via toolRef),
          // so an occupied cell must remain clickable to be removable later.
          // The cursor itself is set below, once all cells exist, by
          // controller.updateCursors() — it depends on the active tool, not
          // on this per-cell setup.
          cell.eventMode = "static";
          cell.on("pointertap", () => controller.handleCellClick(x, y));
          cell.on("pointerover", () => controller.handlePointerOver(x, y));
          cell.on("pointerdown", () => controller.handlePointerDown(x, y));

          cellGraphics[y][x] = cell;
          cameraContainer.addChild(cell);
        }
      }
      controller.updateCursors();
      for (let x = 0; x < terrain[0].length; x++) {
        const label = new Text({
          text: String(x),
          style: { fontSize: 10, fill: 0xffffff },
        });
        label.x = toScreenX(x);
        label.y = height * CELL_SIZE;
        cameraContainer.addChild(label);
      }
      for (let y = 0; y < terrain.length; y++) {
        const label = new Text({
          text: String(y),
          style: { fontSize: 10, fill: 0xffffff },
        });
        label.x = 0;
        label.y = toScreenY(y, height);
        cameraContainer.addChild(label);
      }

      cameraContainer.addChild(ghost);

      const containerEl = containerRef.current;
      containerEl?.addEventListener(
        "pointerleave",
        controller.handlePointerLeave,
      );
      removePointerLeaveListener = () =>
        containerEl?.removeEventListener(
          "pointerleave",
          controller.handlePointerLeave,
        );

      // The pointer can be released outside any cell (or outside the canvas
      // entirely) once a drag has started, so this listens globally rather
      // than on individual cells.
      window.addEventListener("pointerup", controller.handlePointerUp);
      removePointerUpListener = () =>
        window.removeEventListener("pointerup", controller.handlePointerUp);

      unsubscribeWorldState = onWorldState((state) => {
        syncVisitorGraphics(cameraContainer, visitorGraphics, state.visitors, height);
      });
    });

    return () => {
      cancelled = true;
      unsubscribeWorldState?.();
      removePointerLeaveListener?.();
      removePointerUpListener?.();
      if (initialized) {
        app.destroy(true, { children: true });
      }
    };
  }, [sendCommand, onWorldState, onMap]);

  return (
    <div className="flex h-full w-full">
      <div className="flex flex-1 flex-col min-w-0">
        <ParkTopBar balance={balance} visitorCount={visitorCount} />
        {/* Toolbar buttons are real <button> elements: clicking one shifts DOM
        focus there (browser default on mousedown), which would silently kill
        the keyboard shortcuts since they only listen on the canvas container.
        Reclaim focus after any click inside the Park so shortcuts keep working. */}
        <div
          className="relative min-h-0 flex-1"
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
      </div>
      <InspectorPanel
        selection={selection}
        tool={tool}
        onToolChange={onToolChange}
      />
    </div>
  );
}
