import type { CommandRequest, CommandResponse } from "@app/shared-types";
import type { WorldStateResponse } from "@app/shared-types/grpc";
import { useCallback, useEffect, useRef } from "react";
import { io, Socket } from "socket.io-client";

const GATEWAY_URL = "http://localhost:4000";

export function useParkSocket() {
  const socketRef = useRef<Socket | null>(null);

  useEffect(() => {
    const socket = io(GATEWAY_URL);
    socketRef.current = socket;

    return () => {
      socket.disconnect();
      socketRef.current = null;
    };
  }, []);

  const sendCommand = useCallback(
    (request: CommandRequest): Promise<CommandResponse> => {
      return new Promise((resolve, reject) => {
        const socket = socketRef.current;
        if (!socket) {
          reject(new Error("Socket not connected"));
          return;
        }
        socket.emit("command", request, resolve);
      });
    },
    [],
  );

  const onWorldState = useCallback(
    (callback: (state: WorldStateResponse) => void): (() => void) => {
      const socket = socketRef.current;
      if (!socket) {
        return () => {};
      }
      socket.on("worldState", callback);
      return () => {
        socket.off("worldState", callback);
      };
    },
    [],
  );

  return { sendCommand, onWorldState };
}
