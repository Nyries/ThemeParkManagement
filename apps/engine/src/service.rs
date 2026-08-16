use crate::{
    game::GameWorld,
    map::{BuildingId, InfrastructureShape},
    simulation::Rotation,
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use crate::simulation::{
    BuildingCell, CommandRequest, CommandResponse, Coord, ErrorCode, InfrastructureCell,
    InfrastructureKind, MapResponse, StateRequest, TerrainCell, WorldStateResponse,
    command_request::Command, simulation_service_server::SimulationService,
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
        ErrorCode::ErrorInsufficientFunds => "Not enough funds",
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
                        world.park_map.set_terrain(
                            coord.x,
                            coord.y,
                            coord.z,
                            b.material_id.clone(),
                        );
                    }
                }
                action_type = "ApplyTerrain".to_string();

                outcome
            }
            Some(Command::PlaceInfrastructure(p)) => {
                let shape_result: Result<InfrastructureShape, ErrorCode> =
                    match InfrastructureKind::try_from(p.kind) {
                        Ok(InfrastructureKind::Path) => Ok(InfrastructureShape::Path),
                        Ok(InfrastructureKind::Ramp) => {
                            Ok(InfrastructureShape::Ramp { to_z: p.to_z })
                        }
                        Ok(InfrastructureKind::Stairs) => {
                            Ok(InfrastructureShape::Stairs { to_z: p.to_z })
                        }
                        _ => Err(ErrorCode::ErrorEmpty),
                    };

                let coords: Vec<(i32, i32, i32)> =
                    p.coordinates.iter().map(|c| (c.x, c.y, c.z)).collect();

                outcome = match shape_result {
                    Ok(shape) => {
                        let result = world.park_map.can_place_infrastructure(
                            &world.building_catalog,
                            shape.clone(),
                            p.to_z,
                            &coords,
                        );
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
                let coords: Vec<(i32, i32, i32)> =
                    r.coordinates.iter().map(|c| (c.x, c.y, c.z)).collect();
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
                outcome = match world.building_catalog.get(&p.template_id) {
                    Some(template) => {
                        let footprint = template.footprint.clone();
                        let cost = template.cost;
                        match (p.origin.as_ref(), Rotation::try_from(p.rotation)) {
                            (Some(origin), Ok(rotation)) => {
                                let mut result = world.park_map.can_place_building(
                                    (origin.x, origin.y, origin.z),
                                    &footprint,
                                    rotation,
                                );
                                if result.is_ok() && world.balance < cost as f64 {
                                    result = Err(ErrorCode::ErrorInsufficientFunds);
                                }
                                if result.is_ok() {
                                    world.balance -= cost as f64;
                                    let building_id = BuildingId {
                                        building_id: uuid::Uuid::new_v4().to_string(),
                                        template_id: p.template_id.clone(),
                                    };
                                    world.park_map.place_building(
                                        (origin.x, origin.y, origin.z),
                                        &footprint,
                                        rotation,
                                        building_id,
                                    );
                                }
                                result
                            }
                            _ => Err(ErrorCode::ErrorEmpty),
                        }
                    }
                    None => Err(ErrorCode::ErrorInvalidTemplate),
                };
                action_type = "PlaceBuilding".to_string();
                outcome
            }
            Some(Command::RemoveBuilding(r)) => {
                outcome = match r.position.as_ref() {
                    Some(origin) => {
                        let result = world
                            .park_map
                            .can_remove_building(origin.x, origin.y, origin.z);
                        if result.is_ok() {
                            world.park_map.remove_building(origin.x, origin.y, origin.z);
                        }
                        result
                    }
                    _ => Err(ErrorCode::ErrorEmpty),
                };
                action_type = "RemoveBuilding".to_string();
                outcome
            }
            None => Err(ErrorCode::ErrorEmpty),
        };
        println!("Order received from Gateway: {action_type}");

        if action_type == "Empty" {
            return Ok(Response::new(CommandResponse {
                success: false,
                error_code: ErrorCode::ErrorEmpty.into(),
                message: "No command given".into(),
            }));
        } else if req.park_id.is_empty() {
            return Ok(Response::new(CommandResponse {
                success: false,
                error_code: ErrorCode::ErrorEmpty.into(),
                message: "park_id is empty".into(),
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
                error_code: code.into(),
                message: message_for(code).into(),
            })),
        }
    }

    type StreamStateStream = ReceiverStream<Result<WorldStateResponse, Status>>;

    async fn stream_state(
        &self,
        request: Request<StateRequest>,
    ) -> Result<Response<Self::StreamStateStream>, Status> {
        let req = request.into_inner();
        println!(
            "A client connected to the stream with the park : {}",
            req.park_id
        );

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

    async fn get_map(
        &self,
        _request: Request<StateRequest>,
    ) -> Result<Response<MapResponse>, Status> {
        let world = self.world.lock().unwrap();
        let map = &world.park_map;

        let terrain = map
            .terrain
            .iter()
            .map(|(&(x, y, z), material_id)| TerrainCell {
                coord: Some(Coord { x, y, z }),
                material_id: material_id.clone(),
            })
            .collect();

        let infrastructure = map
            .infrastructure
            .iter()
            .map(|(&(x, y, z), shape)| {
                let (kind, to_z) = match shape {
                    InfrastructureShape::Path => (InfrastructureKind::Path, 0),
                    InfrastructureShape::Ramp { to_z } => (InfrastructureKind::Ramp, *to_z),
                    InfrastructureShape::Stairs { to_z } => (InfrastructureKind::Stairs, *to_z),
                };
                InfrastructureCell {
                    coord: Some(Coord { x, y, z }),
                    kind: kind as i32,
                    to_z,
                }
            })
            .collect();

        let building = map
            .building
            .iter()
            .map(|(&(x, y, z), building_id)| BuildingCell {
                coord: Some(Coord { x, y, z }),
                building_id: building_id.building_id.clone(),
                template_id: building_id.template_id.clone(),
            })
            .collect();

        let entrance = map.entrance.map(|(x, y, z)| Coord { x, y, z });

        Ok(Response::new(MapResponse {
            min_x: map.bounds.min_x,
            max_x: map.bounds.max_x,
            min_y: map.bounds.min_y,
            max_y: map.bounds.max_y,
            terrain,
            infrastructure,
            entrance,
            building,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::GameWorld,
        map::Parcel,
        simulation::{
            ApplyTerrain, BuildingCell, Coord, InfrastructureKind, PlaceBuilding,
            PlaceInfrastructure, RemoveBuilding, RemoveInfrastructure, Rotation,
        },
    };
    use std::sync::{Arc, Mutex};
    use tonic::Request;

    fn build_service() -> SimulationEngineService {
        let world = Arc::new(Mutex::new(GameWorld::new()));
        world.lock().unwrap().park_map.parcels.push(Parcel {
            id: "p1".into(),
            cells: vec![(0, 0)],
            unlocked: true,
            price: 0,
        });
        let (state_sender, _) = tokio::sync::broadcast::channel(16);
        SimulationEngineService {
            world,
            state_sender,
        }
    }

    #[tokio::test]
    async fn test_send_command_with_apply_terrain_succeeds() {
        // 1. Initialization of a world and a test service
        let service = build_service();

        // 2. Creating a mock gRPC request
        let apply_brush = ApplyTerrain {
            material_id: "grass".into(),
            coordinates: vec![Coord { x: 0, y: 0, z: 0 }],
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
        let world = service.world.lock().unwrap();
        assert!(inner.success);
        assert_eq!(
            inner.message,
            "Action executed and registered by the engine"
        );
        assert_eq!(
            world.park_map.get_terrain(0, 0, 0),
            Some(&"grass".to_string())
        )
    }

    #[tokio::test]
    async fn test_send_command_with_place_infrastructure_succeeds() {
        // 1. Initialization of a world and a test service
        let service = build_service();

        // 2. Creating a mock gRPC request
        let place_infrastructure = PlaceInfrastructure {
            kind: InfrastructureKind::Path.into(),
            to_z: 0,
            coordinates: vec![Coord { x: 0, y: 0, z: 0 }],
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
        let world = service.world.lock().unwrap();
        assert!(inner.success);
        assert_eq!(
            inner.message,
            "Action executed and registered by the engine"
        );
        assert_eq!(
            world.park_map.get_infrastructure(0, 0, 0),
            Some(&InfrastructureShape::Path)
        )
    }

    #[tokio::test]
    async fn test_send_command_with_remove_infrastructure_succeeds() {
        // 1. Initialization of a world and a test service
        let service = build_service();
        service
            .world
            .lock()
            .unwrap()
            .park_map
            .set_infrastructure(0, 0, 0, InfrastructureShape::Path);

        // 2. Creating a mock gRPC request
        let remove_infrasture = RemoveInfrastructure {
            coordinates: [Coord { x: 0, y: 0, z: 0 }].to_vec(),
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
        let world = service.world.lock().unwrap();
        assert!(inner.success);
        assert_eq!(
            inner.message,
            "Action executed and registered by the engine"
        );
        assert!(world.park_map.get_infrastructure(0, 0, 0).is_none());
    }

    #[tokio::test]
    async fn test_send_command_with_place_building_succeeds() {
        // 1. Initialization of a world and a test service
        let service = build_service();
        service.world.lock().unwrap().park_map.parcels.push(Parcel {
            id: "p2".into(),
            cells: vec![(1, 0), (0, 1), (1, 1)],
            unlocked: true,
            price: 0,
        });
        service.world.lock().unwrap().balance = 5000.0; // sit_down_restaurant costs 3000, above the 1000.0 default

        // 2. Creating a mock gRPC request
        let place_building = PlaceBuilding {
            template_id: "sit_down_restaurant".into(),
            origin: Some(Coord { x: 0, y: 0, z: 0 }),
            rotation: Rotation::Deg0.into(),
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
        let world = service.world.lock().unwrap();
        assert!(inner.success);
        assert_eq!(
            inner.message,
            "Action executed and registered by the engine"
        );
        assert!(world.park_map.get_building(0, 0, 0).is_some());
        assert!(world.park_map.get_building(1, 0, 0).is_some());
        assert!(world.park_map.get_building(0, 1, 0).is_some());
        assert!(world.park_map.get_building(1, 1, 0).is_some());
    }

    #[tokio::test]
    async fn test_send_command_with_place_building_debits_balance_on_success() {
        let service = build_service();
        service.world.lock().unwrap().park_map.parcels.push(Parcel {
            id: "p2".into(),
            cells: vec![(1, 0), (0, 1), (1, 1)],
            unlocked: true,
            price: 0,
        });
        service.world.lock().unwrap().balance = 5000.0;

        let place_building = PlaceBuilding {
            template_id: "sit_down_restaurant".into(), // cost: 3000
            origin: Some(Coord { x: 0, y: 0, z: 0 }),
            rotation: Rotation::Deg0.into(),
        };
        let request = Request::new(CommandRequest {
            park_id: "1".into(),
            command: Command::PlaceBuilding(place_building).into(),
        });

        let response = service.send_command(request).await;

        assert!(response.unwrap().into_inner().success);
        assert_eq!(service.world.lock().unwrap().balance, 2000.0);
    }

    #[tokio::test]
    async fn test_send_command_with_place_building_fails_when_funds_insufficient() {
        let service = build_service();
        service.world.lock().unwrap().park_map.parcels.push(Parcel {
            id: "p2".into(),
            cells: vec![(1, 0), (0, 1), (1, 1)],
            unlocked: true,
            price: 0,
        });
        service.world.lock().unwrap().balance = 100.0; // sit_down_restaurant costs 3000

        let place_building = PlaceBuilding {
            template_id: "sit_down_restaurant".into(),
            origin: Some(Coord { x: 0, y: 0, z: 0 }),
            rotation: Rotation::Deg0.into(),
        };
        let request = Request::new(CommandRequest {
            park_id: "1".into(),
            command: Command::PlaceBuilding(place_building).into(),
        });

        let response = service.send_command(request).await;

        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert!(!inner.success);
        assert_eq!(inner.error_code, ErrorCode::ErrorInsufficientFunds as i32);
        assert_eq!(inner.message, "Not enough funds");
        let world = service.world.lock().unwrap();
        assert!(
            world.park_map.get_building(0, 0, 0).is_none(),
            "building must not be placed when funds are insufficient"
        );
        assert_eq!(
            world.balance, 100.0,
            "balance must not be debited when the placement is rejected"
        );
    }

    #[tokio::test]
    async fn test_send_command_with_remove_building_succeeds() {
        // 1. Initialization of a world and a test service
        let service = build_service();
        service.world.lock().unwrap().park_map.set_building(
            0,
            0,
            0,
            BuildingId {
                building_id: "building-1".into(),
                template_id: "restaurant-1".into(),
            },
        );

        // 2. Creating a mock gRPC request
        let remove_building = RemoveBuilding {
            position: Some(Coord { x: 0, y: 0, z: 0 }),
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
        let world = service.world.lock().unwrap();
        assert!(inner.success);
        assert_eq!(
            inner.message,
            "Action executed and registered by the engine"
        );
        assert!(world.park_map.get_building(0, 0, 0).is_none());
    }

    #[tokio::test]
    async fn test_send_command_without_command_fails() {
        let service = build_service();
        let request = Request::new(CommandRequest {
            park_id: "1".into(),
            command: None,
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

        let apply_terrain = ApplyTerrain {
            material_id: "grass".into(),
            coordinates: vec![Coord { x: 0, y: 0, z: 0 }],
        };
        let request = Request::new(CommandRequest {
            park_id: "".into(),
            command: Command::ApplyTerrain(apply_terrain).into(),
        });

        let response = service.send_command(request).await;

        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert!(!inner.success);
        assert_eq!(inner.message, "park_id is empty");
    }

    #[tokio::test]
    async fn test_get_map_returns_terrain_and_infrastructure_cells() {
        // 1. Initialization of a world and a test service
        let service = build_service();
        service
            .world
            .lock()
            .unwrap()
            .park_map
            .set_terrain(0, 0, 0, "grass".into());
        service.world.lock().unwrap().park_map.set_infrastructure(
            1,
            0,
            0,
            InfrastructureShape::Ramp { to_z: 1 },
        );

        // 2. Creating a mock gRPC request
        let request = Request::new(StateRequest {
            park_id: "1".into(),
        });

        // 3. Calling the gRPC method
        let response = service.get_map(request).await;

        // 4. Assertions
        assert!(response.is_ok());
        let inner = response.unwrap().into_inner();
        assert_eq!(
            inner.terrain,
            vec![TerrainCell {
                coord: Some(Coord { x: 0, y: 0, z: 0 }),
                material_id: "grass".into(),
            }]
        );
        assert_eq!(
            inner.infrastructure,
            vec![InfrastructureCell {
                coord: Some(Coord { x: 1, y: 0, z: 0 }),
                kind: InfrastructureKind::Ramp.into(),
                to_z: 1,
            }]
        );
    }

    #[tokio::test]
    async fn test_get_map_returns_the_map_bounds() {
        // 1. Initialization of a world and a test service
        let service = build_service();

        // 2. Creating a mock gRPC request
        let request = Request::new(StateRequest {
            park_id: "1".into(),
        });

        // 3. Calling the gRPC method
        let response = service.get_map(request).await;

        // 4. Assertions
        let inner = response.unwrap().into_inner();
        let bounds = &service.world.lock().unwrap().park_map.bounds;
        assert_eq!(inner.min_x, bounds.min_x);
        assert_eq!(inner.max_x, bounds.max_x);
        assert_eq!(inner.min_y, bounds.min_y);
        assert_eq!(inner.max_y, bounds.max_y);
    }

    #[tokio::test]
    async fn test_get_map_returns_the_entrance_when_set() {
        // 1. Initialization of a world and a test service
        let service = build_service();
        service.world.lock().unwrap().park_map.entrance = Some((2, 3, 0));

        // 2. Creating a mock gRPC request
        let request = Request::new(StateRequest {
            park_id: "1".into(),
        });

        // 3. Calling the gRPC method
        let response = service.get_map(request).await;

        // 4. Assertions
        let inner = response.unwrap().into_inner();
        assert_eq!(inner.entrance, Some(Coord { x: 2, y: 3, z: 0 }));
    }

    #[tokio::test]
    async fn test_get_map_returns_building_cells() {
        // 1. Initialization of a world and a test service
        let service = build_service();
        service.world.lock().unwrap().park_map.set_building(
            0,
            0,
            0,
            BuildingId {
                building_id: "building-1".into(),
                template_id: "restaurant-1".into(),
            },
        );

        // 2. Creating a mock gRPC request
        let request = Request::new(StateRequest {
            park_id: "1".into(),
        });

        // 3. Calling the gRPC method
        let response = service.get_map(request).await;

        // 4. Assertions
        let inner = response.unwrap().into_inner();
        assert_eq!(
            inner.building,
            vec![BuildingCell {
                coord: Some(Coord { x: 0, y: 0, z: 0 }),
                building_id: "building-1".into(),
                template_id: "restaurant-1".into(),
            }]
        );
    }

    #[tokio::test]
    async fn test_get_map_returns_no_entrance_when_unset() {
        // 1. Initialization of a world and a test service
        let service = build_service();

        // 2. Creating a mock gRPC request
        let request = Request::new(StateRequest {
            park_id: "1".into(),
        });

        // 3. Calling the gRPC method
        let response = service.get_map(request).await;

        // 4. Assertions
        let inner = response.unwrap().into_inner();
        assert_eq!(inner.entrance, None);
    }
}
