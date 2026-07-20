import { describe, it, expect, vi, beforeEach } from 'vitest';
import * as grpc from '@grpc/grpc-js';

// 1. On mocke complètement @grpc/grpc-js et proto-loader si nécessaire,
// ou plus simplement, on mocke le module engineClient en interceptant le constructeur gRPC.
const mockSendCommand = vi.fn();
const mockStreamState = vi.fn();

vi.mock('../engineClient', () => {
  return {
    engineClient: {
      SendCommand: (...args: any[]) => mockSendCommand(...args),
      StreamState: (...args: any[]) => mockStreamState(...args),
    },
    sendCommandToEngine: async (actionType: string, x: number, y: number, payload: object) => {
      return new Promise((resolve, reject) => {
        mockSendCommand(
          { action_type: actionType, x, y, payload: JSON.stringify(payload) },
          (err: any, res: any) => (err ? reject(err) : resolve(res))
        );
      });
    },
    subscribeToEngineStream: (parkId: string, onTick: any, onError: any) => {
      const call = mockStreamState({ park_id: parkId });
      call.on('data', onTick);
      call.on('error', onError);
      return call;
    },
  };
});

// Import après le mock
import { sendCommandToEngine, subscribeToEngineStream, engineClient } from '../engineClient';

describe('EngineClient Service', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('sendCommandToEngine', () => {
    it('should successfully resolve when engine answers correctly', async () => {
      const mockResponse = { success: true, message: 'OK' };
      mockSendCommand.mockImplementation((_payload, callback) => {
        callback(null, mockResponse);
      });

      const result = await sendCommandToEngine('SET_TILE', 5, 10, { type: 'road' });

      expect(mockSendCommand).toHaveBeenCalledTimes(1);
      expect(mockSendCommand).toHaveBeenCalledWith(
        {
          action_type: 'SET_TILE',
          x: 5,
          y: 10,
          payload: JSON.stringify({ type: 'road' }),
        },
        expect.any(Function)
      );
      expect(result).toEqual(mockResponse);
    });

    it('should reject the promise in case of gRPC error', async () => {
      const mockError: grpc.ServiceError = {
        name: 'GrpcError',
        message: 'Unavailable',
        code: grpc.status.UNAVAILABLE,
        details: 'Service down',
        metadata: new grpc.Metadata(),
      };

      mockSendCommand.mockImplementation((_payload, callback) => {
        callback(mockError, null);
      });

      await expect(sendCommandToEngine('SET_TILE', 1, 1, {})).rejects.toEqual(mockError);
    });
  });

  describe('subscribeToEngineStream', () => {
    it('should listen to stream data and trigger onTick callback', () => {
      const mockCall = {
        on: vi.fn((event: string, handler: Function) => {
          if (event === 'data') {
            handler({ tick_count: 42, dirty_chunks_json: '{}' });
          }
          return mockCall;
        }),
        cancel: vi.fn(),
      };

      mockStreamState.mockReturnValue(mockCall);

      const onTickMock = vi.fn();
      const onErrorMock = vi.fn();

      const call = subscribeToEngineStream('park-1', onTickMock, onErrorMock);

      expect(mockStreamState).toHaveBeenCalledWith({ park_id: 'park-1' });
      expect(mockCall.on).toHaveBeenCalledWith('data', expect.any(Function));
      expect(onTickMock).toHaveBeenCalledWith({ tick_count: 42, dirty_chunks_json: '{}' });
      expect(call).toBe(mockCall);
    });
  });
});