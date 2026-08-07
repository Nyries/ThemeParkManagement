// apps/client/src/hooks/__tests__/useParkSocket.spec.ts
import { describe, it, expect, vi } from "vitest";
import { renderHook } from "@testing-library/react";

const { mockEmit, mockDisconnect } = vi.hoisted(() => ({
  mockEmit: vi.fn(),
  mockDisconnect: vi.fn(),
}));

vi.mock("socket.io-client", () => ({
  io: vi.fn(() => ({
    emit: mockEmit,
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

    expect(mockEmit).toHaveBeenCalledWith("command", request, expect.any(Function));
    expect(response).toEqual(mockResponse);
  });

  it("disconnects the socket on unmount", () => {
    const { unmount } = renderHook(() => useParkSocket());
    unmount();

    expect(mockDisconnect).toHaveBeenCalled();
  });
});
