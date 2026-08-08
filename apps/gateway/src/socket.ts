import { CommandRequest, CommandResponse } from "@app/shared-types";
import { Server } from "socket.io";
import { dispatchCommand } from "./services/commandHandler";
import { getMap, subscribeToEngineStream } from "./services/engineClient";
import { shouldRelay } from "./services/relayThrottle";

const PARK_ID = "default";

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

    const call = subscribeToEngineStream(
      PARK_ID,
      (state) => {
        if (shouldRelay(state.tickCount, state.visitors.length)) {
          socket.emit("worldState", state);
        }
      },
      (err) => {
        console.error("Engine stream error: ", err);
      },
    );

    getMap(PARK_ID)
      .then((map) => {
        socket.emit("map", map);
      })
      .catch((err) => {
        console.error("Failed to fetch map: ", err);
      });

    socket.on("disconnect", () => {
      call.cancel();
      console.log(`Client disconnected: ${socket.id}`);
    });
  });
}
