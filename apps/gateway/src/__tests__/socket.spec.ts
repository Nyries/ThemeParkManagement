import { describe, it, expect, vi, beforeAll, afterAll, beforeEach } from "vitest";
import { createServer } from "http";
import type { AddressInfo } from "net";
import { Server } from "socket.io";
import { io as ioClient, type Socket as ClientSocket } from "socket.io-client";
import type { CommandResponse, WorldStateResponse } from "@app/shared-types";

const { mockDispatchCommand, mockSubscribeToEngineStream, mockCall } = vi.hoisted(() => ({
  mockDispatchCommand: vi.fn(),
  mockSubscribeToEngineStream: vi.fn(),
  mockCall: { cancel: vi.fn() },
}));

vi.mock("../services/commandHandler", () => ({
  dispatchCommand: mockDispatchCommand,
}));

vi.mock("../services/engineClient", () => ({
  subscribeToEngineStream: mockSubscribeToEngineStream,
}));

import { registerCommandHandlers } from "../socket";

describe("registerCommandHandlers (integration)", () => {
  let httpServer: ReturnType<typeof createServer>;
  let clientSocket: ClientSocket;

  beforeAll(async () => {
    mockSubscribeToEngineStream.mockReturnValue(mockCall);

    httpServer = createServer();
    const io = new Server(httpServer);
    registerCommandHandlers(io);

    await new Promise<void>((resolve) => httpServer.listen(0, resolve));
    const port = (httpServer.address() as AddressInfo).port;
    clientSocket = ioClient(`http://localhost:${port}`);
    await new Promise<void>((resolve) => clientSocket.on("connect", resolve));
  });

  afterAll(() => {
    clientSocket.close();
    httpServer.close();
  });

  beforeEach(() => {
    mockDispatchCommand.mockClear();
    mockCall.cancel.mockClear();
  });

  it("dispatches the received command and acks the response back to the emitting client", async () => {
    const mockResponse: CommandResponse = { success: true, message: "OK", errorCode: 0 };
    mockDispatchCommand.mockResolvedValue(mockResponse);

    const request = {
      parkId: "park-1",
      command: {
        $case: "applyTerrain" as const,
        applyTerrain: { materialId: "grass", coordinates: [] },
      },
    };

    const ack = await new Promise<CommandResponse>((resolve) => {
      clientSocket.emit("command", request, resolve);
    });

    expect(mockDispatchCommand).toHaveBeenCalledWith(request);
    expect(ack).toEqual(mockResponse);
  });

  it("opens exactly one engine stream subscription for the connected client", () => {
    expect(mockSubscribeToEngineStream).toHaveBeenCalledTimes(1);
    expect(mockSubscribeToEngineStream).toHaveBeenCalledWith(
      "default",
      expect.any(Function),
      expect.any(Function),
    );
  });

  it("relays a world state pushed by the engine to the client", async () => {
    const state: WorldStateResponse = { tickCount: 0, dirtyChunksJson: "{}", visitors: [] };
    const onTick = mockSubscribeToEngineStream.mock.calls[0][1];

    const received = new Promise((resolve) => clientSocket.once("worldState", resolve));
    onTick(state);

    await expect(received).resolves.toEqual(state);
  });

  it("cancels the engine stream subscription when the client disconnects", async () => {
    const port = (httpServer.address() as AddressInfo).port;
    const secondClient = ioClient(`http://localhost:${port}`);
    await new Promise<void>((resolve) => secondClient.on("connect", resolve));

    secondClient.close();
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(mockCall.cancel).toHaveBeenCalled();
  });
});
