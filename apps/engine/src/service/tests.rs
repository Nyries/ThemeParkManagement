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