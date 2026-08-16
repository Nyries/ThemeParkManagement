import { describe, it, expect } from "vitest";
import { nextRotation, rotateFootprint } from "../tool";
import { Rotation } from "@app/shared-types";

describe("nextRotation", () => {
  it("cycles clockwise through the 4 rotations", () => {
    expect(nextRotation(Rotation.ROTATION_DEG_0)).toBe(Rotation.ROTATION_DEG_90);
    expect(nextRotation(Rotation.ROTATION_DEG_90)).toBe(Rotation.ROTATION_DEG_180);
    expect(nextRotation(Rotation.ROTATION_DEG_180)).toBe(Rotation.ROTATION_DEG_270);
    expect(nextRotation(Rotation.ROTATION_DEG_270)).toBe(Rotation.ROTATION_DEG_0);
  });

  it("cycles counter-clockwise when reverse is true", () => {
    expect(nextRotation(Rotation.ROTATION_DEG_0, true)).toBe(
      Rotation.ROTATION_DEG_270,
    );
    expect(nextRotation(Rotation.ROTATION_DEG_270, true)).toBe(
      Rotation.ROTATION_DEG_180,
    );
  });
});

describe("rotateFootprint", () => {
  const lShape = [
    { x: 0, y: 0 },
    { x: 1, y: 0 },
    { x: 0, y: 1 },
  ];

  it("returns the same footprint for a 0deg rotation", () => {
    expect(rotateFootprint(lShape, Rotation.ROTATION_DEG_0)).toEqual(lShape);
  });

  it("rotates 90deg clockwise and normalizes back to non-negative offsets", () => {
    const rotated = rotateFootprint(lShape, Rotation.ROTATION_DEG_90);

    for (const point of rotated) {
      expect(point.x).toBeGreaterThanOrEqual(0);
      expect(point.y).toBeGreaterThanOrEqual(0);
    }
    expect(rotated).toHaveLength(lShape.length);
  });

  it("a square footprint is unchanged in shape (only order) after a 180deg rotation", () => {
    const square = [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 0, y: 1 },
      { x: 1, y: 1 },
    ];

    const rotated = rotateFootprint(square, Rotation.ROTATION_DEG_180);

    expect(new Set(rotated.map((p) => `${p.x},${p.y}`))).toEqual(
      new Set(square.map((p) => `${p.x},${p.y}`)),
    );
  });

  it("rotating 4 times by 90deg returns to the original shape", () => {
    let footprint = lShape;
    for (let i = 0; i < 4; i++) {
      footprint = rotateFootprint(footprint, Rotation.ROTATION_DEG_90);
    }

    expect(new Set(footprint.map((p) => `${p.x},${p.y}`))).toEqual(
      new Set(lShape.map((p) => `${p.x},${p.y}`)),
    );
  });
});
