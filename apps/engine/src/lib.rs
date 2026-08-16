pub mod balance;
pub mod building_template;
pub mod game;
pub mod map;
pub mod map_template;
pub mod pathfinding;
pub mod queue;
pub mod service;
pub mod visitor;

pub mod simulation {
    tonic::include_proto!("simulation");
}
