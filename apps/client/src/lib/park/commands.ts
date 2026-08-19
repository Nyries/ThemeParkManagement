import {
  InfrastructureKind,
  Rotation,
  type CommandRequest,
  type CommandResponse,
} from "@app/shared-types";

function buildApplyTerrainCommand(
  parkId: string,
  materialId: string,
  x: number,
  y: number,
): CommandRequest {
  return {
    parkId,
    command: {
      $case: "applyTerrain",
      applyTerrain: {
        materialId: materialId,
        coordinates: [{ x, y, z: 0 }],
      },
    },
  };
}

export function applyTerrainAt(
  sendCommand: (request: CommandRequest) => Promise<CommandResponse>,
  parkId: string,
  materialId: string,
  x: number,
  y: number,
): Promise<CommandResponse> {
  return sendCommand(buildApplyTerrainCommand(parkId, materialId, x, y));
}

function buildPlaceInfrastructureCommand(
  parkId: string,
  x: number,
  y: number,
): CommandRequest {
  return {
    parkId,
    command: {
      $case: "placeInfrastructure",
      placeInfrastructure: {
        // Hard-coded right now
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

function buildRemoveInfrastructureCommand(
  parkId: string,
  x: number,
  y: number,
): CommandRequest {
  return {
    parkId,
    command: {
      $case: "removeInfrastructure",
      removeInfrastructure: {
        coordinates: [{ x, y, z: 0 }],
      },
    },
  };
}

export function removeInfrastructureAt(
  sendCommand: (request: CommandRequest) => Promise<CommandResponse>,
  parkId: string,
  x: number,
  y: number,
): Promise<CommandResponse> {
  return sendCommand(buildRemoveInfrastructureCommand(parkId, x, y));
}

function buildPlaceBuildingCommand(
  parkId: string,
  template_id: string,
  x: number,
  y: number,
  rotation: Rotation,
): CommandRequest {
  return {
    parkId,
    command: {
      $case: "placeBuilding",
      placeBuilding: {
        templateId: template_id,
        origin: { x, y, z: 0 },
        rotation: rotation,
      },
    },
  };
}

export function placeBuildingAt(
  sendCommand: (request: CommandRequest) => Promise<CommandResponse>,
  parkId: string,
  template_id: string,
  x: number,
  y: number,
  rotation: Rotation,
): Promise<CommandResponse> {
  return sendCommand(
    buildPlaceBuildingCommand(parkId, template_id, x, y, rotation),
  );
}

function buildRemoveBuildingCommand(
  parkId: string,
  x: number,
  y: number,
): CommandRequest {
  return {
    parkId,
    command: {
      $case: "removeBuilding",
      removeBuilding: {
        position: { x, y, z: 0 },
      },
    },
  };
}

export function removeBuildingAt(
  sendCommand: (request: CommandRequest) => Promise<CommandResponse>,
  parkId: string,
  x: number,
  y: number,
): Promise<CommandResponse> {
  return sendCommand(buildRemoveBuildingCommand(parkId, x, y));
}
