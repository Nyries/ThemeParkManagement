use engine::balance::TICK_INTERVAL;
use engine::game::GameWorld;
use engine::map::ParkMap;
use engine::map_template::{MapLoadError, MapSource, MapTemplate};
use engine::service::SimulationEngineService;
use engine::simulation::simulation_service_server::SimulationServiceServer;
use engine::simulation::{VisitorState, WorldStateResponse};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tonic::transport::Server;

const HELP_TEXT: &str =
    "Dev commands: help | reboot | pause | resume | reset {visitors | map} | funds <amount>";

// Shared by startup and `reset map` so both load the exact same embedded
// template the same way.
fn load_default_map() -> Result<ParkMap, MapLoadError> {
    let template = MapTemplate::load(MapSource::Embedded(include_str!(
        "../assets/maps/map-tpm-44.json"
    )))?;
    template.into_park_map()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting the simulation engine!");

    let park_map = load_default_map()?;

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
            let balance;
            {
                let mut w = world_clone.lock().unwrap();
                w.tick(TICK_INTERVAL);
                current_tick = w.tick_count;
                balance = w.balance;

                visitors = w
                    .visitors
                    .iter()
                    .map(|v| VisitorState {
                        id: v.id.clone(),
                        x: v.position.0,
                        y: v.position.1,
                        z: v.position.2,
                    })
                    .collect();

                if !w.paused && current_tick % 100 == 0 {
                    println!("Tick de simulation en cours... Actuel: {}", current_tick);
                }
            }

            let _ = broadcaster.send(WorldStateResponse {
                tick_count: current_tick,
                dirty_chunks_json: "{}".into(),
                visitors,
                balance,
            });

            let elapsed = start.elapsed();
            if elapsed < tick_interval {
                sleep(tick_interval - elapsed).await;
            }
        }
    });

    let world_clone_stdin = Arc::clone(&world);
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        println!("{HELP_TEXT}");
        while let Ok(Some(line)) = lines.next_line().await {
            // Parsed before locking: "help"/"reboot" don't touch world state
            // at all, and reboot's `cargo build` can take seconds — holding
            // the lock for that would freeze the simulation tick loop (which
            // also locks `world_clone`) for the whole build.
            let mut parts = line.split_whitespace();
            match parts.next() {
                Some("help") => {
                    println!("{HELP_TEXT}");
                }
                Some("reboot") => {
                    println!("Currently rebooting");
                    // Run off the async runtime's worker threads: Command::status()
                    // blocks synchronously for the whole build.
                    let status =
                        tokio::task::spawn_blocking(|| Command::new("cargo").arg("build").status())
                            .await;

                    match status {
                        Ok(Ok(s)) if s.success() => {
                            println!("✅ Compilation réussie ! Redémarrage...\n");

                            let bin_path = "./target/debug/engine";

                            #[cfg(unix)]
                            {
                                let err = Command::new(bin_path).exec();
                                // Si exec() revient, c'est qu'il y a eu une erreur
                                eprintln!("Erreur lors du redémarrage avec exec : {err}");
                                break;
                            }

                            #[cfg(not(unix))]
                            {
                                // Fallback multiplateforme (lance un nouveau processus et quitte l'actuel)
                                let _ = Command::new(bin_path).spawn();
                                std::process::exit(0);
                            }
                        }
                        Ok(Ok(_)) => {
                            println!("❌ La compilation a échoué. Correction requise.");
                        }
                        Ok(Err(e)) => {
                            println!("❌ Impossible de lancer cargo build : {e}");
                        }
                        Err(e) => {
                            println!("❌ Erreur interne lors du build : {e}");
                        }
                    }
                }
                Some("pause") => {
                    let mut w = world_clone_stdin.lock().unwrap();
                    w.paused = true;
                    println!("Simulation paused.");
                }
                Some("resume") => {
                    let mut w = world_clone_stdin.lock().unwrap();
                    w.paused = false;
                    println!("Simulation resumed.");
                }
                Some("reset") => match parts.next() {
                    Some("visitors") => {
                        let mut w = world_clone_stdin.lock().unwrap();
                        w.reset_visitors();
                        println!("Visitors reset.");
                    }
                    Some("map") => match load_default_map() {
                        Ok(park_map) => {
                            let mut w = world_clone_stdin.lock().unwrap();
                            w.park_map = park_map;
                            println!("Map reset.");
                        }
                        Err(e) => println!("❌ Failed to reset map: {e}"),
                    },
                    _ => println!("Use either visitors or map."),
                },
                Some("funds") => {
                    let mut w = world_clone_stdin.lock().unwrap();
                    match parts.next() {
                        None => println!("Current balance: {}", w.balance),
                        Some(amount_str) => match amount_str.parse::<f64>() {
                            Ok(amount) => {
                                w.balance += amount;
                                println!("Funds added: {amount}. New balance: {}", w.balance);
                            }
                            Err(_) => println!("Usage: funds [amount]"),
                        },
                    }
                }
                Some(other) => println!("Unknown command: {other}"),
                None => {}
            }
        }
    });

    let addr = "0.0.0.0:50051".parse()?;
    let service = SimulationEngineService {
        world,
        state_sender,
    };

    println!("gRPC server of the engine started on port 50051");

    Server::builder()
        .add_service(SimulationServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
