import { describe, it, expect, vi } from "vitest";
import { InfrastructureKind } from "@app/shared-types";
import { buildPlaceInfrastructureCommand, placeInfrastructureAt } from "../placeInfrastructure";

describe("buildPlaceInfrastructureCommand", () => {
  it("builds a placeInfrastructure command with the given park, coordinates and kind Path", () => {
    const request = buildPlaceInfrastructureCommand("park-1", 3, 5);

    expect(request).toEqual({
      parkId: "park-1",
      command: {
        $case: "placeInfrastructure",
        placeInfrastructure: {
          kind: InfrastructureKind.INFRASTRUCTURE_KIND_PATH,
          toZ: 0,
          coordinates: [{ x: 3, y: 5, z: 0 }],
        },
      },
    });
  });
});

describe("placeInfrastructureAt", () => {
  it("sends the built command through sendCommand and forwards the response", async () => {
    const mockResponse = { success: true, message: "OK", errorCode: 0 };
    const sendCommand = vi.fn().mockResolvedValue(mockResponse);

    const result = await placeInfrastructureAt(sendCommand, "park-1", 3, 5);

    expect(sendCommand).toHaveBeenCalledWith(
      buildPlaceInfrastructureCommand("park-1", 3, 5),
    );
    expect(result).toEqual(mockResponse);
  });
});
