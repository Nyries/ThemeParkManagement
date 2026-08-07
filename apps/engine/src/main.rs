use engine::balance::TICK_INTERVAL;
use engine::game::GameWorld;
use engine::service::SimulationEngineService;
use engine::map_template::{MapSource, MapTemplate};
use engine::simulation::simulation_service_server::SimulationServiceServer;
use engine::simulation::{VisitorState, WorldStateResponse};

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting the simulation engine!");

    // Loading park_map
    let template = MapTemplate::load(MapSource::Embedded(include_str!("../assets/maps/first-map.json")))?;
    let park_map = template.into_park_map()?;

    let mut world = GameWorld::new();
    world.park_map = park_map;
    let world = Arc::new(Mutex::new(world));

    let (state_sender, _) = tokio::sync::broadcast::channel(100);

    let world_clone = Arc::clone(&world);
    let broadcaster = state_sender.clone();

    tokio::spawn(async move {
        let tick_interval = Duration::from_millis(50);
        loop {
            let start = Instant::now();

            let current_tick;
            let visitors;
            {
                let mut w = world_clone.lock().unwrap();
                w.tick(TICK_INTERVAL);
                current_tick = w.tick_count;

                visitors = w.visitors.iter().map(|v| VisitorState {
                    id: v.id.clone(),
                    x: v.position.0,
                    y: v.position.1,
                    z: v.position.2,
                }).collect();
        
                if current_tick % 100 == 0 {
                    println!("Tick de simulation en cours... Actuel: {}", current_tick);
                }
            }
        
            let _ = broadcaster.send(WorldStateResponse {
                tick_count: current_tick,
                dirty_chunks_json: "{}".into(),
                visitors,
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

// #[cfg(test)]
// mod tests {
//     use super::*;


// }
