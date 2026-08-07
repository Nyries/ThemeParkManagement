import {
  CommandResponse,
  InfrastructureKind,
  type CommandRequest,
} from "@app/shared-types";

export function buildPlaceInfrastructureCommand(
  parkId: string,
  x: number,
  y: number,
): CommandRequest {
  return {
    parkId,
    command: {
      $case: "placeInfrastructure",
      placeInfrastructure: {
        kind: InfrastructureKind.INFRASTRUCTURE_KIND_PATH,
        toZ: 0,
        coordinates: [{ x, y, z: 0 }],
      },
    },
  };
}

export function placeInfrastructureAt(
  sendCommand: (request: CommandRequest) => Promise<CommandResponse>,
  parkId: string,
  x: number,
  y: number,
): Promise<CommandResponse> {
  return sendCommand(buildPlaceInfrastructureCommand(parkId, x, y));
}
