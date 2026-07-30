pub mod map;
pub mod game;
pub mod service;
pub mod pathfinding;
pub mod map_template;

pub mod simulation {
    tonic::include_proto!("simulation");
}