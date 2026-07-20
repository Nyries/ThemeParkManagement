use crate::game::GameWorld;
use crate::map::TileType;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::simulation::{
    simulation_service_server:: SimulationService, 
    CommandRequest, CommandResponse, StateRequest, WorldStateResponse
};

#[derive(Clone)]
pub struct SimulationEngineService {
    pub world: Arc<Mutex<GameWorld>>,
    pub state_sender: tokio::sync::broadcast::Sender<WorldStateResponse>,
}

#[tonic::async_trait]
impl SimulationService for SimulationEngineService {
    async fn send_command(
        &self,
        request: Request<CommandRequest>,
    ) -> Result<Response<CommandResponse>, Status> {
        let req = request.into_inner();
        println!("Order received from Gateway : {} en ({}, {})", req.action_type, req.x, req.y);
        
        if req.action_type == "SET_TILE" {
            let mut world = self.world.lock().unwrap();
            world.park_map.set_tile(req.x, req.y, TileType::Path, 0);
        }

        Ok(Response::new(CommandResponse { 
            success: true, 
            message: "Action executed and registered by the engine".into(),
        }))
    }

    type StreamStateStream = ReceiverStream<Result<WorldStateResponse, Status>>;

    async fn stream_state(
        &self,
        request: Request<StateRequest>
    ) -> Result<Response<Self::StreamStateStream>, Status> {
        let req = request.into_inner();
        println!("A client connected to the stream with the park : {}", req.park_id);
        
        let (tx, rx) = mpsc::channel(128);
        let mut broadcast_rx = self.state_sender.subscribe();

        tokio::spawn(async move {
            while let Ok(state) = broadcast_rx.recv().await {
                if tx.send(Ok(state)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameWorld;
    use std::sync::{Arc, Mutex};
    use tonic::Request;

    #[tokio::test]
    async fn test_send_command_handler() {
        // 1. Initialization of a world and a test service
        let world = Arc::new(Mutex::new(GameWorld::new()));
        let (state_sender, _) = tokio::sync::broadcast::channel(16);
        let service = SimulationEngineService { world, state_sender };

        // 2. Creating a mock gRPC request
        let request = Request::new(CommandRequest {
            action_type: "SET_TILE".into(),
            x: 5,
            y: 10,
            payload: "{}".into(),
        });

        // 3. Calling the gRPC method
        let response = service.send_command(request).await;

        // 4. Assertions
        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert!(inner.success);
        assert_eq!(inner.message, "Action executed and registered by the engine");
    }
}