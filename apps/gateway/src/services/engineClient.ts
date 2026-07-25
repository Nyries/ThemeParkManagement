import * as grpc from "@grpc/grpc-js";
import {
  ApplyBrush,
  CommandResponse,
  PlaceBuilding,
  RemoveBuilding,
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

export function sendPlaceBuilding(
  parkId: string,
  placeBuilding: PlaceBuilding,
): Promise<CommandResponse> {
  return new Promise((resolve, reject) => {
    engineClient.sendCommand(
      {
        parkId: parkId,
        command: { $case: "placeBuilding", placeBuilding },
      },
      (error: grpc.ServiceError | null, response: CommandResponse) => {
        if (error) reject(error);
        else resolve(response);
      },
    );
  });
}

export function sendRemoveBuilding(
  parkId: string,
  removeBuilding: RemoveBuilding,
): Promise<CommandResponse> {
  return new Promise((resolve, reject) => {
    engineClient.sendCommand(
      {
        parkId: parkId,
        command: { $case: "removeBuilding", removeBuilding },
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
