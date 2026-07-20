import * as grpc from "@grpc/grpc-js";
import * as protoLoader from "@grpc/proto-loader";
import path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PROTO_PATH = path.join(
  __dirname,
  "../../../../packages/proto/simulation.proto",
);

const packageDefinition = protoLoader.loadSync(PROTO_PATH, {
  keepCase: true,
  longs: String,
  enums: String,
  defaults: true,
  oneofs: true,
});

const protoDescriptor = grpc.loadPackageDefinition(packageDefinition) as any;
const simulationPackage = protoDescriptor.simulation;

const ENGINE_URL = process.env.ENGINE_GRPC_URL || "localhost:50051";
export const engineClient = new simulationPackage.SimulationService(
  ENGINE_URL,
  grpc.credentials.createInsecure(),
);

export function sendCommandToEngine(
  actionType: string,
  x: number,
  y: number,
  payload: object,
) {
  return new Promise((resolve, reject) => {
    engineClient.SendCommand(
      {
        actionType: actionType,
        x,
        y,
        payload: JSON.stringify(payload),
      },
      (error: grpc.ServiceError | null, response: any) => {
        if (error) {
          reject(error);
        } else {
          resolve(response);
        }
      },
    );
  });
}

export function subscribeToEngineStream(
  parkId: string,
  onTick: (state: any) => void,
  onError: (err: any) => void,
) {
  const call = engineClient.StreamState({ park_id: parkId });

  call.on("data", (response: any) => {
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
