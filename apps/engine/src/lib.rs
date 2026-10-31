pub mod balance;
pub mod building_template;
pub mod game;
pub mod map;
pub mod map_template;
pub mod service;
pub mod visitor;

#[allow(clippy::result_large_err)]
pub mod simulation {
    tonic::include_proto!("simulation");
}
