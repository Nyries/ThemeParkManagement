use engine::game::GameWorld;
use engine::service::SimulationEngineService;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tonic::transport::Server;

use engine::simulation::{
    simulation_service_client::SimulationServiceClient, 
    simulation_service_server::SimulationServiceServer,
    CommandRequest, StateRequest};

#[tokio::test]
async fn test_full_grpc_server_integration() {
    let addr: SocketAddr = "[::1]:50052"
        .parse()
        .unwrap();

    let world = Arc::new(Mutex::new(GameWorld::new()));
    let (state_sender, _) = tokio::sync::broadcast::channel(16);
    let service = SimulationEngineService { world, state_sender };

    let listener = TcpListener::bind(addr).await.unwrap();
    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

    tokio::spawn(async move {
        Server::builder()
            .add_service(SimulationServiceServer::new(service))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let channel = tonic::transport::Channel::from_static("http://[::1]:50052")
        .connect()
        .await
        .expect("Impossible de connect to gRPC test server");

    let mut client = SimulationServiceClient::new(channel);

    let request = tonic::Request::new(CommandRequest {
        action_type: "SET_TILE".into(),
        x: 42,
        y: 84,
        payload: "{}".into(),
    });

    let response = client.send_command(request).await.expect("Failed calling gRPC SendCommand");
    let inner = response.into_inner();
    assert!(inner.success);

    let stream_request = tonic::Request::new(StateRequest {
        park_id: "central-parc-1".into(),
    });

    let mut stream = client
        .stream_state(stream_request)
        .await
        .expect("Failed to initialize stream")
        .into_inner();

    assert!(true);
}