import { describe, it, expect, vi, beforeEach } from "vitest";
import * as grpc from "@grpc/grpc-js";

const { mockSendCommand, mockStreamState, mockGetMap } = vi.hoisted(() => ({
  mockSendCommand: vi.fn(),
  mockStreamState: vi.fn(),
  mockGetMap: vi.fn(),
}));

vi.mock("@app/shared-types/grpc", async () => {
  const actual = await vi.importActual<typeof import("@app/shared-types/grpc")>(
    "@app/shared-types/grpc",
  );
  return {
    ...actual,
    SimulationServiceClient: class {
      sendCommand = mockSendCommand;
      streamState = mockStreamState;
      getMap = mockGetMap;
    },
  };
});

import {
  sendApplyTerrain,
  sendPlaceInfrastructure,
  sendPlaceBuilding,
  sendRemoveBuilding,
  subscribeToEngineStream,
  sendRemoveInfrastructure,
  getMap,
} from "../engineClient";

describe("EngineClient Service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("sendApplyTerrain", () => {
    it("resolves with the engine response on success", async () => {
      const mockResponse = { success: true, message: "OK", errorCode: 0 };
      mockSendCommand.mockImplementation((_request, callback) =>
        callback(null, mockResponse),
      );

      const applyTerrain = {
        materialId: "grass",
        coordinates: [{ x: 0, y: 0, z: 0 }],
      };
      const result = await sendApplyTerrain("park-1", applyTerrain);

      expect(mockSendCommand).toHaveBeenCalledWith(
        { parkId: "park-1", command: { $case: "applyTerrain", applyTerrain } },
        expect.any(Function),
      );
      expect(result).toEqual(mockResponse);
    });

    it("rejects when the engine returns a gRPC error", async () => {
      const mockError: grpc.ServiceError = {
        name: "GrpcError",
        message: "Unavailable",
        code: grpc.status.UNAVAILABLE,
        details: "Service down",
        metadata: new grpc.Metadata(),
      };
      mockSendCommand.mockImplementation((_request, callback) =>
        callback(mockError, null),
      );

      await expect(
        sendApplyTerrain("park-1", { materialId: "grass", coordinates: [] }),
      ).rejects.toEqual(mockError);
    });
  });

  describe("sendPlaceInfrastructure", () => {
    it("wraps the payload in the placeInfrastructure oneof case", async () => {
      mockSendCommand.mockImplementation((_request, callback) =>
        callback(null, { success: true, message: "OK", errorCode: 0 }),
      );

      const placeInfrastructure = {
        kind: 1,
        toZ: 0,
        coordinates: [{ x: 0, y: 0, z: 0 }],
      };
      await sendPlaceInfrastructure("park-1", placeInfrastructure);

      expect(mockSendCommand).toHaveBeenCalledWith(
        {
          parkId: "park-1",
          command: { $case: "placeInfrastructure", placeInfrastructure },
        },
        expect.any(Function),
      );
    });
  });

  describe("sendRemoveInfrastructure", () => {
    it("wraps the payload in the removeInfrastructure oneof case", async () => {
      mockSendCommand.mockImplementation((_request, callback) =>
        callback(null, { success: true, message: "OK", errorCode: 0 }),
      );

      const removeInfrastructure = { coordinates: [{ x: 0, y: 0, z: 0 }] };
      await sendRemoveInfrastructure("park-1", removeInfrastructure);

      expect(mockSendCommand).toHaveBeenCalledWith(
        {
          parkId: "park-1",
          command: { $case: "removeInfrastructure", removeInfrastructure },
        },
        expect.any(Function),
      );
    });
  });

  describe("sendPlaceBuilding", () => {
    it("wraps the payload in the placeBuilding oneof case", async () => {
      mockSendCommand.mockImplementation((_request, callback) =>
        callback(null, { success: true, message: "OK", errorCode: 0 }),
      );

      const placeBuilding = {
        templateId: "restaurant-1",
        origin: { x: 0, y: 0, z: 0 },
        rotation: 0,
      };
      await sendPlaceBuilding("park-1", placeBuilding);

      expect(mockSendCommand).toHaveBeenCalledWith(
        {
          parkId: "park-1",
          command: { $case: "placeBuilding", placeBuilding },
        },
        expect.any(Function),
      );
    });
  });

  describe("sendRemoveBuilding", () => {
    it("wraps the payload in the removeBuilding oneof case", async () => {
      mockSendCommand.mockImplementation((_request, callback) =>
        callback(null, { success: true, message: "OK", errorCode: 0 }),
      );

      const removeBuilding = { position: { x: 0, y: 0, z: 0 } };
      await sendRemoveBuilding("park-1", removeBuilding);

      expect(mockSendCommand).toHaveBeenCalledWith(
        {
          parkId: "park-1",
          command: { $case: "removeBuilding", removeBuilding },
        },
        expect.any(Function),
      );
    });
  });

  describe("subscribeToEngineStream", () => {
    it("forwards stream data to onTick and listens for stream errors", () => {
      const mockCall = {
        on: vi.fn((event: string, handler: (...args: unknown[]) => void) => {
          if (event === "data")
            handler({ tickCount: "42", dirtyChunksJson: "{}" });
          return mockCall;
        }),
      };
      mockStreamState.mockReturnValue(mockCall);

      const onTick = vi.fn();
      const onError = vi.fn();
      const call = subscribeToEngineStream("park-1", onTick, onError);

      expect(mockStreamState).toHaveBeenCalledWith({ parkId: "park-1" });
      expect(onTick).toHaveBeenCalledWith({
        tickCount: "42",
        dirtyChunksJson: "{}",
      });
      expect(call).toBe(mockCall);
    });
  });
  describe("getMap", () => {
    it("resolves with the map returned by the engine", async () => {
      const mockResponse = {
        minX: 0,
        maxX: 9,
        minY: 0,
        maxY: 7,
        terrain: [],
        infrastructure: [],
        entrance: undefined,
      };
      mockGetMap.mockImplementation((_request, callback) =>
        callback(null, mockResponse),
      );

      const result = await getMap("park-1");

      expect(mockGetMap).toHaveBeenCalledWith(
        { parkId: "park-1" },
        expect.any(Function),
      );
      expect(result).toEqual(mockResponse);
    });

    it("rejects when the engine returns a gRPC error", async () => {
      const mockError: grpc.ServiceError = {
        name: "GrpcError",
        message: "Unavailable",
        code: grpc.status.UNAVAILABLE,
        details: "Service down",
        metadata: new grpc.Metadata(),
      };
      mockGetMap.mockImplementation((_request, callback) =>
        callback(mockError, null),
      );

      await expect(getMap("park-1")).rejects.toEqual(mockError);
    });
  });
});
