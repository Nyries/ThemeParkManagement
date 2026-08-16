// apps/client/src/hooks/__tests__/useParkSocket.spec.ts
import { describe, it, expect, vi } from "vitest";
import { renderHook } from "@testing-library/react";

const { mockEmit, mockOn, mockOnce, mockOff, mockDisconnect } = vi.hoisted(
  () => ({
    mockEmit: vi.fn(),
    mockOn: vi.fn(),
    mockOnce: vi.fn(),
    mockOff: vi.fn(),
    mockDisconnect: vi.fn(),
  }),
);

vi.mock("socket.io-client", () => ({
  io: vi.fn(() => ({
    emit: mockEmit,
    on: mockOn,
    once: mockOnce,
    off: mockOff,
    disconnect: mockDisconnect,
  })),
}));

import { useParkSocket } from "../useParkSocket";

describe("useParkSocket", () => {
  it("emits a command event and resolves with the ack response", async () => {
    const mockResponse = { success: true, message: "OK", errorCode: 0 };
    mockEmit.mockImplementation((_event, _request, ack) => ack(mockResponse));

    const { result } = renderHook(() => useParkSocket());
    const request = { parkId: "park-1", command: undefined };

    const response = await result.current.sendCommand(request);

    expect(mockEmit).toHaveBeenCalledWith(
      "command",
      request,
      expect.any(Function),
    );
    expect(response).toEqual(mockResponse);
  });
  describe("onWorldState", () => {
    it("registers a listener for the worldState event", () => {
      const { result } = renderHook(() => useParkSocket());
      const callback = vi.fn();

      result.current.onWorldState(callback);

      expect(mockOn).toHaveBeenCalledWith("worldState", callback);
    });

    it("returns an unsubscribe function that removes the listener", () => {
      const { result } = renderHook(() => useParkSocket());
      const callback = vi.fn();

      const unsubscribe = result.current.onWorldState(callback);
      unsubscribe();

      expect(mockOff).toHaveBeenCalledWith("worldState", callback);
    });
  });
  describe("onMap", () => {
    it("resolves with the map pushed once by the server", async () => {
      const mockMap = {
        minX: 0,
        maxX: 9,
        minY: 0,
        maxY: 7,
        terrain: [],
        infrastructure: [],
        entrance: undefined,
      };
      mockOnce.mockImplementation((_event, callback) => callback(mockMap));

      const { result } = renderHook(() => useParkSocket());
      const map = await result.current.onMap();

      expect(mockOnce).toHaveBeenCalledWith("map", expect.any(Function));
      expect(map).toEqual(mockMap);
    });
  });

  it("disconnects the socket on unmount", () => {
    const { unmount } = renderHook(() => useParkSocket());
    unmount();

    expect(mockDisconnect).toHaveBeenCalled();
  });
});
