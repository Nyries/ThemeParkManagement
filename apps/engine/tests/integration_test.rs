use engine::game::GameWorld;
use engine::service::SimulationEngineService;
use engine::simulation::RemoveInfrastructure;
use tonic::Request;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tonic::transport::Server;

use engine::simulation::{
    simulation_service_client::SimulationServiceClient, 
    simulation_service_server::SimulationServiceServer,
    CommandRequest, StateRequest,
    command_request::Command,
    ApplyTerrain, Coord, PlaceInfrastructure, InfrastructureKind, PlaceBuilding, RemoveBuilding, Rotation};

async fn spawn_test_server() -> SimulationServiceClient<tonic::transport::Channel> {
    let listener = TcpListener::bind("[::1]:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
    
    let world = Arc::new(Mutex::new(GameWorld::new()));
    let (state_sender, _) = tokio::sync::broadcast::channel(16);
    let service = SimulationEngineService { world, state_sender };

    tokio::spawn(async move {
        Server::builder()
            .add_service(SimulationServiceServer::new(service))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let channel = tonic::transport::Channel::from_shared(format!("http://{addr}"))
        .unwrap()
        .connect()
        .await
        .expect("Impossible to connect to gRPC test server");

    SimulationServiceClient::new(channel)
}

#[tokio::test]
async fn test_send_command_with_apply_terrain_succeeds() {
    let mut client = spawn_test_server().await;

    let apply_terrain  = ApplyTerrain {
        material_id: "grass".into(),
        coordinates: vec![Coord {x:0, y:0, z:0}]
    };
    let request = Request::new(CommandRequest {
        park_id: "1".into(),
        command: Command::ApplyTerrain(apply_terrain).into(),
    });
    
    let response = client.send_command(request).await.expect("Failed calling gRPC SendCommand");

    assert!(response.into_inner().success);
}

#[tokio::test]
async fn test_send_command_with_place_infrastructure_succeeds() {
    let mut client = spawn_test_server().await;

        let place_infrastructure  = PlaceInfrastructure {
            kind: InfrastructureKind::Path.into(),
            to_z: 0,
            coordinates: vec![Coord {x:0, y:0, z:0}]
        };
    let request = Request::new(CommandRequest {
        park_id: "1".into(),
        command: Command::PlaceInfrastructure(place_infrastructure).into(),
    });
    
    let response = client.send_command(request).await.expect("Failed calling gRPC SendCommand");

    assert!(response.into_inner().success);
}

#[tokio::test]
async fn test_send_command_with_remove_infrastructure_succeeds() {
    let mut client = spawn_test_server().await;

    let remove_infrastructure  = RemoveInfrastructure {
        coordinates: [Coord { x: 0, y: 0, z: 0 }].to_vec(),
    };
    let request = Request::new(CommandRequest {
        park_id: "1".into(),
        command: Command::RemoveInfrastructure(remove_infrastructure).into(),
    });
    
    let response = client.send_command(request).await.expect("Failed calling gRPC SendCommand");

    assert!(response.into_inner().success);
}

#[tokio::test]
async fn test_send_command_with_place_building_succeeds() {
    let mut client = spawn_test_server().await;

    let place_building  = PlaceBuilding {
        template_id: "restaurant-1".into(),
        origin: Some(Coord {x:0, y:0, z:0}),
        rotation: Rotation::Deg180.into()
    };
    let request = Request::new(CommandRequest {
        park_id: "1".into(),
        command: Command::PlaceBuilding(place_building).into(),
    });
    
    let response = client.send_command(request).await.expect("Failed calling gRPC SendCommand");

    assert!(response.into_inner().success);
}

#[tokio::test]
async fn test_send_command_with_remove_building_succeeds() {
    let mut client = spawn_test_server().await;

    let remove_building  = RemoveBuilding {
        position: Some(Coord { x: 0, y: 0, z: 0 })
    };
    let request = Request::new(CommandRequest {
        park_id: "1".into(),
        command: Command::RemoveBuilding(remove_building).into(),
    });
    
    let response = client.send_command(request).await.expect("Failed calling gRPC SendCommand");

    assert!(response.into_inner().success);
}

#[tokio::test]
async fn test_send_command_without_command_fails() {
    let mut client = spawn_test_server().await;

    let request = Request::new(CommandRequest {
        park_id: "1".into(),
        command: None
    });
    
    let response = client.send_command(request).await.expect("Failed calling gRPC SendCommand");

    assert!(!response.into_inner().success);
}

#[tokio::test]
async fn stream_state_connects_successfully() {
    let mut client = spawn_test_server().await;

    let stream_request = tonic::Request::new(StateRequest {
        park_id: "park-1".into()
    });

    let stream = client.stream_state(stream_request)
    .await
    .expect("Failed to initialize stream")
    .into_inner();

    let _ = stream;
}