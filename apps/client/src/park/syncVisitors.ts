import { Graphics, Container } from "pixi.js";
import type { VisitorState } from "@app/shared-types/grpc";
import { toScreenX, toScreenY, CELL_SIZE } from "../rendering/grid";

const VISITOR_RADIUS = CELL_SIZE / 4;
const VISITOR_COLOR = 0xff4444;

export function syncVisitorGraphics(
  stage: Container,
  visitorGraphics: Map<string, Graphics>,
  visitors: readonly VisitorState[],
): void {
  const presentIds = new Set(visitors.map((v) => v.id));

  for (const visitor of visitors) {
    const screenX = toScreenX(visitor.x) + CELL_SIZE / 2;
    const screenY = toScreenY(visitor.y) + CELL_SIZE / 2;

    let sprite = visitorGraphics.get(visitor.id);
    if (!sprite) {
      sprite = new Graphics().circle(0, 0, VISITOR_RADIUS).fill(VISITOR_COLOR);
      visitorGraphics.set(visitor.id, sprite);
      stage.addChild(sprite);
    }

    sprite.x = screenX;
    sprite.y = screenY;
  }

  for (const [id, sprite] of visitorGraphics) {
    if (!presentIds.has(id)) {
      stage.removeChild(sprite);
      sprite.destroy();
      visitorGraphics.delete(id);
    }
  }
}
