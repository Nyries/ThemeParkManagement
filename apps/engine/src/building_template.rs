use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub enum BuildingCategory {
    ShopUtility,
    Attraction,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VisitorBehavior {
    //TO DISCUSS
    LongStay,
    ShortStay,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CrossingFlags {
    pub bridge_above_allowed: bool,
    pub tunnel_below_allowed: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ResourceFlow {
    //TO DISCUSS
    pub resource_id: String,
    pub amount_per_tick: i32,
}

#[derive(Deserialize, Debug)]
pub struct BuildingTemplate {
    pub template_id : String,
    pub name : String,
    pub category: BuildingCategory,
    pub footprint: Vec<(i32, i32)>,
    pub cost: u32,
    pub visitor_behavior: VisitorBehavior,
    pub crossing_flags: CrossingFlags,
    pub construction_time_ticks: Option<u32>,
    pub needs_relief: HashMap<String, u32>,
    pub resource_vector: Option<Vec<ResourceFlow>>,
    pub tags: Vec<String>,
    pub intensity: Option<u8>,
    pub cycle_capacity : Option<u32>,
    pub cycle_duration_tick: Option<u32>,
    pub price_per_use: Option<u32>,
    pub biome_exclusive: Option<String>,
}