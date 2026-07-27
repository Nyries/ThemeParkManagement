use crate::{game::GameWorld, map::InfrastructureShape, simulation::{ErrorCode, InfrastructureKind}};
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
        let mut action_type: String = "Empty".to_string();
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
                    world.park_map.set_terrain(coord.x, coord.y, coord.z, b.material_id.clone());
                    }
                }
                action_type = "ApplyTerrain".to_string();
                
                outcome
            },
            Some(Command::PlaceInfrastructure(p)) => {
                let shape_result: Result<InfrastructureShape, ErrorCode> = match InfrastructureKind::try_from(p.kind) {
                    Ok(InfrastructureKind::Path) => Ok(InfrastructureShape::Path),
                    Ok(InfrastructureKind::Ramp) => Ok(InfrastructureShape::Ramp { to_z: p.to_z }),
                    Ok(InfrastructureKind::Stairs) => Ok(InfrastructureShape::Stairs { to_z: p.to_z }),
                    _ => Err(ErrorCode::ErrorEmpty),
                };

                let coords: Vec<(i32, i32, i32)> = p.coordinates.iter().map(|c| (c.x, c.y, c.z)).collect();

                outcome = match shape_result {
                    Ok(shape) => {
                        let result = world.park_map.can_place_infrastructure(shape.clone(), p.to_z, &coords);
                        if result.is_ok() {
                            for (x, y, z) in coords {
                                world.park_map.set_infrastructure(x, y, z, shape.clone());
                            }
                        }
                        result
                    }
                    Err(e) => Err(e),
                };
                action_type = "PlaceInfrastructure".to_string();
                outcome
            }
            
            Some(Command::RemoveInfrastructure(r)) => {
                let coords: Vec<(i32, i32, i32)> = r.coordinates.iter().map(|c| (c.x, c.y, c.z)).collect();
                outcome = world.park_map.can_remove_infrastructure(&coords);
                if outcome.is_ok() {
                    for (x, y, z) in coords {
                        world.park_map.remove_infrasture(x, y, z);
                    }
                }
                action_type = "RemoveInfrastructure".to_string();
                outcome
            }
            Some(Command::PlaceBuilding(p)) => {
                // Validation of the command
                // outcome = world.park_map.can_place_building(p.origin, footprint, rotation, template_id)
                action_type = "PlaceBuilding".to_string();
                outcome
                
            }
            Some(Command::RemoveBuilding(r)) => {
                action_type = "RemoveBuilding".to_string();
                outcome
            }
            None => {
                Err(ErrorCode::ErrorEmpty)},
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
mod tests;