import { CommandRequest, CommandResponse, ErrorCode } from "@app/shared-types";
import {
  sendApplyTerrain,
  sendPlaceBuilding,
  sendPlaceInfrastructure,
  sendRemoveBuilding,
  sendRemoveInfrastructure,
} from "./engineClient";

export async function dispatchCommand(
  request: CommandRequest,
): Promise<CommandResponse> {
  const { parkId, command } = request;

  if (!command) {
    return {
      success: false,
      message: "Missing command",
      errorCode: ErrorCode.ERROR_EMPTY,
    };
  }

  try {
    switch (command?.$case) {
      case "applyTerrain":
        return await sendApplyTerrain(parkId, command.applyTerrain);
      case "placeInfrastructure":
        return await sendPlaceInfrastructure(
          parkId,
          command.placeInfrastructure,
        );
      case "removeInfrastructure":
        return await sendRemoveInfrastructure(
          parkId,
          command.removeInfrastructure,
        );
      case "placeBuilding":
        return await sendPlaceBuilding(parkId, command.placeBuilding);
      case "removeBuilding":
        return await sendRemoveBuilding(parkId, command.removeBuilding);
      default: {
        const unhandled: never = command;
        return unhandled;
      }
    }
  } catch (error) {
    console.error("Engine call failed: ", error);
    return {
      success: false,
      message: "Engine unavailable",
      errorCode: ErrorCode.UNRECOGNIZED,
    };
  }
}
