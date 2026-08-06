import { describe, it, expect, vi, beforeAll, afterAll, beforeEach } from "vitest";
import { createServer } from "http";
import type { AddressInfo } from "net";
import { Server } from "socket.io";
import { io as ioClient, type Socket as ClientSocket } from "socket.io-client";
import type { CommandResponse } from "@app/shared-types";

const { mockDispatchCommand } = vi.hoisted(() => ({
  mockDispatchCommand: vi.fn(),
}));

vi.mock("../services/commandHandler", () => ({
  dispatchCommand: mockDispatchCommand,
}));

import { registerCommandHandlers } from "../socket";

describe("registerCommandHandlers (integration)", () => {
  let httpServer: ReturnType<typeof createServer>;
  let clientSocket: ClientSocket;

  beforeAll(async () => {
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
    vi.clearAllMocks();
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
});
