import { describe, it, expect, vi } from "vitest";
import { InfrastructureKind, Rotation } from "@app/shared-types";
import {
  applyTerrainAt,
  placeInfrastructureAt,
  removeInfrastructureAt,
  placeBuildingAt,
  removeBuildingAt,
} from "../commands";

describe("applyTerrainAt", () => {
  it("sends an applyTerrain command for the given cell", async () => {
    const sendCommand = vi.fn().mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    const response = await applyTerrainAt(sendCommand, "default", "grass", 2, 3);

    expect(sendCommand).toHaveBeenCalledWith({
      parkId: "default",
      command: {
        $case: "applyTerrain",
        applyTerrain: {
          materialId: "grass",
          coordinates: [{ x: 2, y: 3, z: 0 }],
        },
      },
    });
    expect(response.success).toBe(true);
  });
});

describe("placeInfrastructureAt", () => {
  it("sends a placeInfrastructure command with kind Path", async () => {
    const sendCommand = vi.fn().mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    await placeInfrastructureAt(sendCommand, "default", 1, 1);

    expect(sendCommand).toHaveBeenCalledWith({
      parkId: "default",
      command: {
        $case: "placeInfrastructure",
        placeInfrastructure: {
          kind: InfrastructureKind.INFRASTRUCTURE_KIND_PATH,
          toZ: 0,
          coordinates: [{ x: 1, y: 1, z: 0 }],
        },
      },
    });
  });
});

describe("removeInfrastructureAt", () => {
  it("sends a removeInfrastructure command for the given cell", async () => {
    const sendCommand = vi.fn().mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    await removeInfrastructureAt(sendCommand, "default", 4, 5);

    expect(sendCommand).toHaveBeenCalledWith({
      parkId: "default",
      command: {
        $case: "removeInfrastructure",
        removeInfrastructure: {
          coordinates: [{ x: 4, y: 5, z: 0 }],
        },
      },
    });
  });
});

describe("placeBuildingAt", () => {
  it("sends a placeBuilding command with the template id, origin and rotation", async () => {
    const sendCommand = vi.fn().mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    await placeBuildingAt(
      sendCommand,
      "default",
      "sit_down_restaurant",
      6,
      7,
      Rotation.ROTATION_DEG_90,
    );

    expect(sendCommand).toHaveBeenCalledWith({
      parkId: "default",
      command: {
        $case: "placeBuilding",
        placeBuilding: {
          templateId: "sit_down_restaurant",
          origin: { x: 6, y: 7, z: 0 },
          rotation: Rotation.ROTATION_DEG_90,
        },
      },
    });
  });
});

describe("removeBuildingAt", () => {
  it("sends a removeBuilding command for the given origin", async () => {
    const sendCommand = vi.fn().mockResolvedValue({
      success: true,
      message: "OK",
      errorCode: 0,
    });

    await removeBuildingAt(sendCommand, "default", 8, 9);

    expect(sendCommand).toHaveBeenCalledWith({
      parkId: "default",
      command: {
        $case: "removeBuilding",
        removeBuilding: {
          position: { x: 8, y: 9, z: 0 },
        },
      },
    });
  });
});
