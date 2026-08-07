import type { CommandRequest, CommandResponse } from "@app/shared-types";
import { useEffect, useRef } from "react";
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

  function sendCommand(request: CommandRequest): Promise<CommandResponse> {
    return new Promise((resolve, reject) => {
      const socket = socketRef.current;
      if (!socket) {
        reject(new Error("Socket not connected"));
        return;
      }
      socket.emit("command", request, resolve);
    });
  }

  return {sendCommand };
}
