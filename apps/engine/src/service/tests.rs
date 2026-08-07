use super::*;
use crate::{game::GameWorld, map::Parcel, simulation::{ApplyTerrain, Coord, InfrastructureKind, PlaceBuilding, PlaceInfrastructure, RemoveBuilding, RemoveInfrastructure, Rotation}};
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
    let world = service.world.lock().unwrap();
    assert!(inner.success);
    assert_eq!(inner.message, "Action executed and registered by the engine");
    assert_eq!(world.park_map.get_terrain(0, 0, 0), Some(&"grass".to_string()))
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
    let world = service.world.lock().unwrap();
    assert!(inner.success);
    assert_eq!(inner.message, "Action executed and registered by the engine");
    assert_eq!(world.park_map.get_infrastructure(0, 0, 0), Some(&InfrastructureShape::Path))
    
}

    #[tokio::test]
async fn test_send_command_with_remove_infrastructure_succeeds() {
    // 1. Initialization of a world and a test service
    let service = build_service();
    service.world.lock().unwrap().park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);

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
    let world = service.world.lock().unwrap();
    assert!(inner.success);
    assert_eq!(inner.message, "Action executed and registered by the engine");
    assert!(world.park_map.get_infrastructure(0, 0, 0).is_none());
}

#[tokio::test]
async fn test_send_command_with_place_building_succeeds() {
    // 1. Initialization of a world and a test service
    let service = build_service();
    service.world.lock().unwrap().park_map.parcels.push(Parcel {
        id: "p2".into(),
        cells: vec![(0,1), (1,1)],
        unlocked: true,
        price: 0,
    });
    
    // 2. Creating a mock gRPC request
    let place_building  = PlaceBuilding {
        template_id: "restaurant-1".into(),
        origin: Some(Coord {x:0, y:0, z:0}),
        rotation: Rotation::Deg0.into()
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
    assert_eq!(inner.message, "Action executed and registered by the engine");
    assert!(world.park_map.get_building(0, 0, 0).is_some());
    assert!(world.park_map.get_building(0, 1, 0).is_some());
    assert!(world.park_map.get_building(1, 1, 0).is_some());
}

#[tokio::test]
async fn test_send_command_with_remove_building_succeeds() {
    // 1. Initialization of a world and a test service
    let service = build_service();
    service.world.lock().unwrap().park_map.set_building(0, 0, 0, BuildingId {
        building_id: "building-1".into(),
        template_id: "restaurant-1".into(),
    });

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
    let world = service.world.lock().unwrap();
    assert!(inner.success);
    assert_eq!(inner.message, "Action executed and registered by the engine");
    assert!(world.park_map.get_building(0, 0, 0).is_none());
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

#[tokio::test]
async fn test_get_map_returns_terrain_and_infrastructure_cells() {
    // 1. Initialization of a world and a test service
    let service = build_service();
    service.world.lock().unwrap().park_map.set_terrain(0, 0, 0, "grass".into());
    service.world.lock().unwrap().park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Ramp { to_z: 1 });

    // 2. Creating a mock gRPC request
    let request = Request::new(StateRequest { park_id: "1".into() });

    // 3. Calling the gRPC method
    let response = service.get_map(request).await;

    // 4. Assertions
    assert!(response.is_ok());
    let inner = response.unwrap().into_inner();
    assert_eq!(inner.terrain, vec![TerrainCell {
        coord: Some(Coord { x: 0, y: 0, z: 0 }),
        material_id: "grass".into(),
    }]);
    assert_eq!(inner.infrastructure, vec![InfrastructureCell {
        coord: Some(Coord { x: 1, y: 0, z: 0 }),
        kind: InfrastructureKind::Ramp.into(),
        to_z: 1,
    }]);
}

#[tokio::test]
async fn test_get_map_returns_the_map_bounds() {
    // 1. Initialization of a world and a test service
    let service = build_service();

    // 2. Creating a mock gRPC request
    let request = Request::new(StateRequest { park_id: "1".into() });

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
    let request = Request::new(StateRequest { park_id: "1".into() });

    // 3. Calling the gRPC method
    let response = service.get_map(request).await;

    // 4. Assertions
    let inner = response.unwrap().into_inner();
    assert_eq!(inner.entrance, Some(Coord { x: 2, y: 3, z: 0 }));
}

#[tokio::test]
async fn test_get_map_returns_no_entrance_when_unset() {
    // 1. Initialization of a world and a test service
    let service = build_service();

    // 2. Creating a mock gRPC request
    let request = Request::new(StateRequest { park_id: "1".into() });

    // 3. Calling the gRPC method
    let response = service.get_map(request).await;

    // 4. Assertions
    let inner = response.unwrap().into_inner();
    assert_eq!(inner.entrance, None);
}
