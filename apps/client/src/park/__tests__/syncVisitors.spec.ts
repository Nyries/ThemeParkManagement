import { describe, it, expect } from "vitest";
import { Container, Graphics } from "pixi.js";
import { syncVisitorGraphics } from "../syncVisitors";

describe("syncVisitorGraphics", () => {
  it("creates a graphic and adds it to the stage for a new visitor", () => {
    const stage = new Container();
    const visitorGraphics = new Map<string, Graphics>();

    syncVisitorGraphics(stage, visitorGraphics, [{ id: "v1", x: 3, y: 2, z: 0 }]);

    expect(visitorGraphics.has("v1")).toBe(true);
    expect(stage.children).toContain(visitorGraphics.get("v1"));
  });

  it("moves an existing visitor's graphic instead of creating a new one", () => {
    const stage = new Container();
    const visitorGraphics = new Map<string, Graphics>();

    syncVisitorGraphics(stage, visitorGraphics, [{ id: "v1", x: 0, y: 0, z: 0 }]);
    const sprite = visitorGraphics.get("v1");

    syncVisitorGraphics(stage, visitorGraphics, [{ id: "v1", x: 5, y: 5, z: 0 }]);

    expect(visitorGraphics.get("v1")).toBe(sprite);
    expect(stage.children).toHaveLength(1);
  });

  it("removes a visitor's graphic when it is absent from the next message", () => {
    const stage = new Container();
    const visitorGraphics = new Map<string, Graphics>();

    syncVisitorGraphics(stage, visitorGraphics, [{ id: "v1", x: 0, y: 0, z: 0 }]);
    syncVisitorGraphics(stage, visitorGraphics, []);

    expect(visitorGraphics.has("v1")).toBe(false);
    expect(stage.children).toHaveLength(0);
  });

  it("keeps visitors still present and removes only the ones that vanished", () => {
    const stage = new Container();
    const visitorGraphics = new Map<string, Graphics>();

    syncVisitorGraphics(stage, visitorGraphics, [
      { id: "v1", x: 0, y: 0, z: 0 },
      { id: "v2", x: 1, y: 1, z: 0 },
    ]);
    syncVisitorGraphics(stage, visitorGraphics, [{ id: "v2", x: 1, y: 1, z: 0 }]);

    expect(visitorGraphics.has("v1")).toBe(false);
    expect(visitorGraphics.has("v2")).toBe(true);
  });
});
