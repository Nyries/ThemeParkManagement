import * as grpc from "@grpc/grpc-js";
import {
  ApplyTerrain,
  CommandResponse,
  PlaceBuilding,
  PlaceInfrastructure,
  RemoveBuilding,
  RemoveInfrastructure,
  WorldStateResponse,
} from "@app/shared-types";
import { SimulationServiceClient } from "@app/shared-types/grpc"


const ENGINE_URL = process.env.ENGINE_GRPC_URL || "localhost:50051";
export const engineClient = new SimulationServiceClient(
  ENGINE_URL,
  grpc.credentials.createInsecure(),
);

export function sendApplyTerrain(
  parkId: string,
  applyTerrain: ApplyTerrain,
): Promise<CommandResponse> {
  return new Promise((resolve, reject) => {
    engineClient.sendCommand(
      {
        parkId: parkId,
        command: { $case: "applyTerrain", applyTerrain },
      },
      (error: grpc.ServiceError | null, response: CommandResponse) => {
        if (error) reject(error);
        else resolve(response);
      },
    );
  });
}

export function sendPlaceInfrastructure(
  parkId: string,
  placeInfrastructure: PlaceInfrastructure,
): Promise<CommandResponse> {
  return new Promise((resolve, reject) => {
    engineClient.sendCommand(
      {
        parkId: parkId,
        command: { $case: "placeInfrastructure", placeInfrastructure },
      },
      (error: grpc.ServiceError | null, response: CommandResponse) => {
        if (error) reject(error);
        else resolve(response);
      },
    );
  });
}

export function sendRemoveInfrastructure(
  parkId: string,
  removeInfrastructure: RemoveInfrastructure,
): Promise<CommandResponse> {
  return new Promise((resolve, reject) => {
    engineClient.sendCommand(
      {
        parkId: parkId,
        command: { $case: "removeInfrastructure", removeInfrastructure },
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
  onError: (err: grpc.ServiceError) => void,
) {
  const call = engineClient.streamState({ parkId: parkId });

  call.on("data", (response: WorldStateResponse) => {
    onTick(response);
  });

  call.on("error", (error: grpc.ServiceError) => {
    onError(error);
  });

  call.on("end", () => {
    console.log("gRCP Flow closed by the engine");
  });

  return call;
}
