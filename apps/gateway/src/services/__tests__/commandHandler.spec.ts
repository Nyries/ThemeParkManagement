import { describe, it, expect, vi, beforeEach } from 'vitest';
import { ErrorCode } from '@app/shared-types';

const {
  mockSendApplyTerrain,
  mockSendPlaceInfrastructure,
  mockSendRemoveInfrastructure,
  mockSendPlaceBuilding,
  mockSendRemoveBuilding,
} = vi.hoisted(() => ({
  mockSendApplyTerrain: vi.fn(),
  mockSendPlaceInfrastructure: vi.fn(),
  mockSendRemoveInfrastructure: vi.fn(),
  mockSendPlaceBuilding: vi.fn(),
  mockSendRemoveBuilding: vi.fn(),
}));

vi.mock('../engineClient', () => ({
  sendApplyTerrain: mockSendApplyTerrain,
  sendPlaceInfrastructure: mockSendPlaceInfrastructure,
  sendRemoveInfrastructure: mockSendRemoveInfrastructure,
  sendPlaceBuilding: mockSendPlaceBuilding,
  sendRemoveBuilding: mockSendRemoveBuilding,
}));

import { dispatchCommand } from '../commandHandler';

describe('dispatchCommand', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('routes an applyTerrain command to sendApplyTerrain', async () => {
    const mockResponse = { success: true, message: 'OK', errorCode: ErrorCode.ERROR_NONE };
    mockSendApplyTerrain.mockResolvedValue(mockResponse);

    const applyTerrain = { materialId: 'grass', coordinates: [{ x: 0, y: 0, z: 0 }] };
    const result = await dispatchCommand({
      parkId: 'park-1',
      command: { $case: 'applyTerrain', applyTerrain },
    });

    expect(mockSendApplyTerrain).toHaveBeenCalledWith('park-1', applyTerrain);
    expect(result).toEqual(mockResponse);
  });

  it('routes a placeInfrastructure command to sendPlaceInfrastructure', async () => {
    const mockResponse = { success: true, message: 'OK', errorCode: ErrorCode.ERROR_NONE };
    mockSendPlaceInfrastructure.mockResolvedValue(mockResponse);

    const placeInfrastructure = { kind: 1, toZ: 0, coordinates: [{ x: 0, y: 0, z: 0 }] };
    const result = await dispatchCommand({
      parkId: 'park-1',
      command: { $case: 'placeInfrastructure', placeInfrastructure },
    });

    expect(mockSendPlaceInfrastructure).toHaveBeenCalledWith('park-1', placeInfrastructure);
    expect(result).toEqual(mockResponse);
  });

  it('routes a removeInfrastructure command to sendRemoveInfrastructure', async () => {
    const mockResponse = { success: true, message: 'OK', errorCode: ErrorCode.ERROR_NONE };
    mockSendRemoveInfrastructure.mockResolvedValue(mockResponse);

    const removeInfrastructure = { coordinates: [{ x: 0, y: 0, z: 0 }] };
    const result = await dispatchCommand({
      parkId: 'park-1',
      command: { $case: 'removeInfrastructure', removeInfrastructure },
    });

    expect(mockSendRemoveInfrastructure).toHaveBeenCalledWith('park-1', removeInfrastructure);
    expect(result).toEqual(mockResponse);
  });

  it('routes a placeBuilding command to sendPlaceBuilding', async () => {
    const mockResponse = { success: true, message: 'OK', errorCode: ErrorCode.ERROR_NONE };
    mockSendPlaceBuilding.mockResolvedValue(mockResponse);

    const placeBuilding = { templateId: 'restaurant-1', origin: { x: 0, y: 0, z: 0 }, rotation: 0 };
    const result = await dispatchCommand({
      parkId: 'park-1',
      command: { $case: 'placeBuilding', placeBuilding },
    });

    expect(mockSendPlaceBuilding).toHaveBeenCalledWith('park-1', placeBuilding);
    expect(result).toEqual(mockResponse);
  });

  it('routes a removeBuilding command to sendRemoveBuilding', async () => {
    const mockResponse = { success: true, message: 'OK', errorCode: ErrorCode.ERROR_NONE };
    mockSendRemoveBuilding.mockResolvedValue(mockResponse);

    const removeBuilding = { position: { x: 0, y: 0, z: 0 } };
    const result = await dispatchCommand({
      parkId: 'park-1',
      command: { $case: 'removeBuilding', removeBuilding },
    });

    expect(mockSendRemoveBuilding).toHaveBeenCalledWith('park-1', removeBuilding);
    expect(result).toEqual(mockResponse);
  });

  it('returns an error response without calling the engine when command is missing', async () => {
    const result = await dispatchCommand({ parkId: 'park-1', command: undefined });

    expect(result).toEqual({
      success: false,
      message: 'Missing command',
      errorCode: ErrorCode.ERROR_EMPTY,
    });
    expect(mockSendApplyTerrain).not.toHaveBeenCalled();
  });

  it('forwards a success:false response from the engine without altering it', async () => {
    const mockResponse = {
      success: false,
      message: 'Cell already occupied',
      errorCode: ErrorCode.ERROR_COLLISION,
    };
    mockSendPlaceInfrastructure.mockResolvedValue(mockResponse);

    const result = await dispatchCommand({
      parkId: 'park-1',
      command: {
        $case: 'placeInfrastructure',
        placeInfrastructure: { kind: 1, toZ: 0, coordinates: [] },
      },
    });

    expect(result).toEqual(mockResponse);
  });

  it('returns a generic error response when the engine call rejects (transport failure)', async () => {
    mockSendApplyTerrain.mockRejectedValue(new Error('UNAVAILABLE'));

    const applyTerrain = { materialId: 'grass', coordinates: [] };
    const result = await dispatchCommand({
      parkId: 'park-1',
      command: { $case: 'applyTerrain', applyTerrain },
    });

    expect(result).toEqual({
      success: false,
      message: 'Engine unavailable',
      errorCode: ErrorCode.UNRECOGNIZED,
    });
  });
});
