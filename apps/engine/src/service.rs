use crate::{game::GameWorld, map::Material, simulation::ErrorCode};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::simulation::{
    simulation_service_server:: SimulationService, 
    CommandRequest, CommandResponse, StateRequest, WorldStateResponse,
    command_request::Command
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
        let mut action_type: String = "None".to_string();
        let mut world = self.world.lock().unwrap();
        let mut outcome = Ok(());
        
        let result: Result<(), ErrorCode> = match &req.command {
            Some(Command::ApplyTerrain(b)) => {
                // Validation of the command
                for coord in &b.coordinates {
                    outcome = world.park_map.can_apply_terrain(coord.x, coord.y, coord.z);
                    if outcome.is_err() {
                        break;
                    }
                    if outcome.is_ok() {
                    world.park_map.set_terrain(coord.x, coord.y, coord.z, Material { material_id: b.material_id.clone() });
                    }
                }
                action_type = "ApplyTerrain".to_string();

                
                outcome
            },
            Some(Command::PlaceInfrastructure(p)) => {
                outcome
            }
            
            Some(Command::RemoveInfrastructure(r)) => {
                outcome
            }
            Some(Command::PlaceBuilding(p)) => {
                // Validation of the command
                // outcome = world.park_map.can_place_building(p.origin, footprint, rotation, template_id)
                outcome
                
            }
            Some(Command::RemoveBuilding(r)) => {
                outcome
            }
            None => Err(ErrorCode::ErrorEmpty),
        };
        println!("Order received from Gateway: {action_type}");

        let mut error_code = ErrorCode::ErrorNone;
        if action_type == "Empty" {
            error_code = ErrorCode::ErrorEmpty;
            return Ok(Response::new(CommandResponse {
                success: false,
                error_code: error_code.into(),
                message: "No command given".into()
            }));
        } else if req.park_id == "" {
            error_code = ErrorCode::ErrorEmpty;
            return Ok(Response::new(CommandResponse {
                success: false,
                error_code: error_code.into(),
                message: "park_id is empty".into()
            }));
        }

        Ok(Response::new(CommandResponse { 
            success: true, 
            error_code: error_code.into(),
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
    use crate::{game::GameWorld, simulation::{ApplyTerrain, Coord, InfrastructureKind, PlaceBuilding, PlaceInfrastructure, RemoveBuilding, RemoveInfrastructure, Rotation}};
    use std::sync::{Arc, Mutex};
    use tonic::Request;

    fn build_service() -> SimulationEngineService {
        let world = Arc::new(Mutex::new(GameWorld::new()));
        let (state_sender, _) = tokio::sync::broadcast::channel(16);
        SimulationEngineService { world, state_sender }
    }

    #[tokio::test]
    async fn test_send_command_with_apply_terrain_succeeds() {
        // 1. Initialization of a world and a test service
        let service = build_service();

        // 2. Creating a mock gRPC request
        let apply_brush  = ApplyTerrain {
            material_id: "grass".into(),
            coordinates: vec![Coord {x:0, y:0, z:0}]
        };
        let request = Request::new(CommandRequest {
            park_id: "1".into(),
            command: Command::ApplyTerrain(apply_brush).into(),
        });

        // 3. Calling the gRPC method
        let response = service.send_command(request).await;

        // 4. Assertions
        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert!(inner.success);
        assert_eq!(inner.message, "Action executed and registered by the engine");
    }

    #[tokio::test]
    async fn test_send_command_with_place_infrastructure_succeeds() {
        // 1. Initialization of a world and a test service
        let service = build_service();

        // 2. Creating a mock gRPC request
        let place_infrastructure  = PlaceInfrastructure {
            kind: InfrastructureKind::Path.into(),
            to_z: 0,
            coordinates: vec![Coord {x:0, y:0, z:0}]
        };
        let request = Request::new(CommandRequest {
            park_id: "1".into(),
            command: Command::PlaceInfrastructure(place_infrastructure).into(),
        });

        // 3. Calling the gRPC method
        let response = service.send_command(request).await;

        // 4. Assertions
        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert!(inner.success);
        assert_eq!(inner.message, "Action executed and registered by the engine");
    }

        #[tokio::test]
    async fn test_send_command_with_remove_infrastructure_succeeds() {
        // 1. Initialization of a world and a test service
        let service = build_service();

        // 2. Creating a mock gRPC request
        let remove_infrasture  = RemoveInfrastructure {
            coordinates: [Coord { x: 0, y: 0, z: 0 }].to_vec()
        };
        let request = Request::new(CommandRequest {
            park_id: "1".into(),
            command: Command::RemoveInfrastructure(remove_infrasture).into(),
        });

        // 3. Calling the gRPC method
        let response = service.send_command(request).await;

        // 4. Assertions
        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert!(inner.success);
        assert_eq!(inner.message, "Action executed and registered by the engine");
    }

    #[tokio::test]
    async fn test_send_command_with_place_building_succeeds() {
        // 1. Initialization of a world and a test service
        let service = build_service();

        // 2. Creating a mock gRPC request
        let place_building  = PlaceBuilding {
            template_id: "restaurant-1".into(),
            origin: Some(Coord {x:0, y:0, z:0}),
            rotation: Rotation::Deg180.into()
        };
        let request = Request::new(CommandRequest {
            park_id: "1".into(),
            command: Command::PlaceBuilding(place_building).into(),
        });

        // 3. Calling the gRPC method
        let response = service.send_command(request).await;

        // 4. Assertions
        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert!(inner.success);
        assert_eq!(inner.message, "Action executed and registered by the engine");
    }

    #[tokio::test]
    async fn test_send_command_with_remove_building_succeeds() {
        // 1. Initialization of a world and a test service
        let service = build_service();

        // 2. Creating a mock gRPC request
        let remove_building  = RemoveBuilding {
            position: Some(Coord { x: 0, y: 0, z: 0 })
        };
        let request = Request::new(CommandRequest {
            park_id: "1".into(),
            command: Command::RemoveBuilding(remove_building).into(),
        });

        // 3. Calling the gRPC method
        let response = service.send_command(request).await;

        // 4. Assertions
        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert!(inner.success);
        assert_eq!(inner.message, "Action executed and registered by the engine");
    }

    #[tokio::test]
    async fn test_send_command_without_command_fails() {
        let service = build_service();
        let request = Request::new(CommandRequest {
            park_id: "1".into(),
            command: None
        });

        let response = service.send_command(request).await;

        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert!(!inner.success);
        assert_eq!(inner.message, "No command given");
    }

    #[tokio::test]
    async fn test_send_command_with_empty_park_id_fails() {
        let service = build_service();

        let apply_terrain  = ApplyTerrain {
            material_id: "grass".into(),
            coordinates: vec![Coord {x:0, y:0, z:0}]
        };
        let request = Request::new(CommandRequest {
            park_id: "".into(),
            command: Command::ApplyTerrain(apply_terrain).into()
        });

        let response = service.send_command(request).await;

        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert!(!inner.success);
        assert_eq!(inner.message, "park_id is empty");
    }
}