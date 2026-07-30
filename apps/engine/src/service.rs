use crate::{game::GameWorld, map::{BuildingId, InfrastructureShape, footprint_for}, simulation::Rotation};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::simulation::{
    simulation_service_server:: SimulationService, 
    CommandRequest, CommandResponse, StateRequest, WorldStateResponse,
    command_request::Command, ErrorCode, InfrastructureKind
};


#[derive(Clone)]
pub struct SimulationEngineService {
    pub world: Arc<Mutex<GameWorld>>,
    pub state_sender: tokio::sync::broadcast::Sender<WorldStateResponse>,
}

fn message_for(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::ErrorNone => "Action executed and registered by the engine",
        ErrorCode::ErrorOutOfBounds => "Target cell is out of bounds or on a locked level/parcel",
        ErrorCode::ErrorCollision => "Target cell is already occupied",
        ErrorCode::ErrorCrossingNotAllowed => "Crossing not allowed here",
        ErrorCode::ErrorEmpty => "Invalid or empty command",
        ErrorCode::ErrorInvalidTemplate => "Invalid template",
        ErrorCode::ErrorInsufficientFunds => "Not enough funds"
    }
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
                let footprint = footprint_for(&p.template_id);
                outcome = match (p.origin.as_ref(), Rotation::try_from(p.rotation)) {
                    (Some(origin), Ok(rotation)) => {
                        let result = world.park_map.can_place_building((origin.x, origin.y, origin.z), &footprint, rotation);
                        if result.is_ok() {
                            let building_id = BuildingId {
                                building_id: uuid::Uuid::new_v4().to_string(),
                                template_id: p.template_id.clone(),
                            };
                            world.park_map.place_building((origin.x, origin.y, origin.z), &footprint, rotation, building_id);
                        }
                        result
                    }
                    _ => Err(ErrorCode::ErrorEmpty),
                };
                action_type = "PlaceBuilding".to_string();
                outcome
            }
            Some(Command::RemoveBuilding(r)) => {
                outcome =  match r.position.as_ref() {
                    Some(origin) => {
                        let result = world.park_map.can_remove_building(origin.x, origin.y, origin.z);
                        if result.is_ok() {
                            world.park_map.remove_building(origin.x, origin.y, origin.z);
                        }    
                        result
                    }
                    _ => Err(ErrorCode::ErrorEmpty)
                };
                action_type = "RemoveBuilding".to_string();
                outcome
            }
            None => {
                Err(ErrorCode::ErrorEmpty)},
        };
        println!("Order received from Gateway: {action_type}");

        if action_type == "Empty" {
            return Ok(Response::new(CommandResponse {
                success: false,
                error_code: ErrorCode::ErrorEmpty.into(),
                message: "No command given".into()
            }));
        } else if req.park_id.is_empty() {
            return Ok(Response::new(CommandResponse {
                success: false,
                error_code: ErrorCode::ErrorEmpty.into(),
                message: "park_id is empty".into()
            }));
        }

        match result {
            Ok(()) => Ok(Response::new(CommandResponse {
                success: true,
                error_code: ErrorCode::ErrorNone.into(),
                message: "Action executed and registered by the engine".into(),
            })),
            Err(code) => Ok(Response::new(CommandResponse { 
                success: false,
                error_code: code.into() ,
                message: message_for(code).into(), 
            })),
        }

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