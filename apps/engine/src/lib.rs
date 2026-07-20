pub mod map;
pub mod game;
pub mod service;

pub mod simulation {
    tonic::include_proto!("simulation");
}