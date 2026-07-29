mod map;
mod game;
mod service;
mod pathfinding;
mod map_template;

use game::GameWorld;
use service::SimulationEngineService;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tonic::transport::Server;

pub mod simulation {
    tonic::include_proto!("simulation");
}

use simulation::simulation_service_server::SimulationServiceServer;
use simulation::WorldStateResponse;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting the simulation engine!");

    let world = Arc::new(Mutex::new(GameWorld::new()));

    let (state_sender, _) = tokio::sync::broadcast::channel(100);

    let world_clone = Arc::clone(&world);
    let broadcaster = state_sender.clone();

    tokio::spawn(async move {
        let tick_interval = Duration::from_millis(50);
        loop {
            let start = Instant::now();

            let current_tick;
            {
                let mut w = world_clone.lock().unwrap();
                w.update();
                current_tick = w.tick_count;
        
                if current_tick % 100 == 0 {
                    println!("Tick de simulation en cours... Actuel: {}", current_tick);
                }
            }
        
            let _ = broadcaster.send(WorldStateResponse {
                tick_count: current_tick,
                dirty_chunks_json: "{}".into(),
            });

            let elapsed = start.elapsed();
            if elapsed < tick_interval {
                sleep(tick_interval-elapsed).await;
            }
        }
    });

    let addr = "[::1]:50051".parse()?;
    let service = SimulationEngineService {world, state_sender};

    println!("gRPC server of the engine started on port 50051");

    Server::builder()
    .add_service(SimulationServiceServer::new(service))
    .serve(addr)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;


}
