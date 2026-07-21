import * as grpc from "@grpc/grpc-js";
import {
  ApplyBrush,
  CommandResponse,
  PlaceEntity,
  RemoveEntity,
  SimulationServiceClient,
  WorldStateResponse,
} from "@app/shared-types";


const ENGINE_URL = process.env.ENGINE_GRPC_URL || "localhost:50051";
export const engineClient = new SimulationServiceClient(
  ENGINE_URL,
  grpc.credentials.createInsecure(),
);

export function sendApplyBrush(
  parkId: string,
  applyBrush: ApplyBrush,
): Promise<CommandResponse> {
  return new Promise((resolve, reject) => {
    engineClient.sendCommand(
      {
        parkId: parkId,
        command: { $case: "applyBrush", applyBrush },
      },
      (error: grpc.ServiceError | null, response: CommandResponse) => {
        if (error) reject(error);
        else resolve(response);
      },
    );
  });
}

export function sendPlaceEntity(
  parkId: string,
  placeEntity: PlaceEntity,
): Promise<CommandResponse> {
  return new Promise((resolve, reject) => {
    engineClient.sendCommand(
      {
        parkId: parkId,
        command: { $case: "placeEntity", placeEntity },
      },
      (error: grpc.ServiceError | null, response: CommandResponse) => {
        if (error) reject(error);
        else resolve(response);
      },
    );
  });
}

export function sendRemoveEntity(
  parkId: string,
  removeEntity: RemoveEntity,
): Promise<CommandResponse> {
  return new Promise((resolve, reject) => {
    engineClient.sendCommand(
      {
        parkId: parkId,
        command: { $case: "removeEntity", removeEntity },
      },
      (error: grpc.ServiceError | null, response: CommandResponse) => {
        if (error) reject(error);
        else resolve(response);
      },
    );
  });
}

export function subscribeToEngineStream(
  parkId: string,
  onTick: (state: WorldStateResponse) => void,
  onError: (err: any) => void,
) {
  const call = engineClient.streamState({ parkId: parkId });

  call.on("data", (response: WorldStateResponse) => {
    onTick(response);
  });

  call.on("error", (error: any) => {
    onError(error);
  });

  call.on("end", () => {
    console.log("gRCP Flow closed by the engine");
  });

  return call;
}
