import { CommandRequest, CommandResponse } from "@app/shared-types";
import { Server } from "socket.io";
import { dispatchCommand } from "./services/commandHandler";

export function registerCommandHandlers(io: Server) {
  io.on("connection", (socket) => {
    console.log(`Client connected: ${socket.id}`);

    socket.on(
      "command",
      async (
        request: CommandRequest,
        ack: (response: CommandResponse) => void,
      ) => {
        const response = await dispatchCommand(request);
        ack(response);
      },
    );

    socket.on("disconnect", () => {
      console.log(`Client disconnected: ${socket.id}`);
    });
  });
}
