use std::collections::HashMap;

use super::{join_queue, queue_at_entrance, visitor_diameter};

use crate::{
    balance::{AFFINITY_DEFAULT, COMFORT_THRESHOLD_DEFAULT, ENTERTAINMENT_RELIEF},
    building_template::{BuildingCatalog, BuildingCategory},
    map::{BuildingId, ParkMap},
    queue::{QueueState, estimated_wait_for},
    visitor::{
        ENTERTAINMENT, Visitor, affinity_for, gain_for, grow_needs, novelty_for, penalty_for,
        relieve_need, score_for, update_satisfaction, utility_for,
    },
};

pub(super) fn redirect_if_expired(
    v: &mut Visitor,
    park_map: &ParkMap,
    old_cell: (i32, i32, i32),
    exit: Option<(i32, i32, i32)>,
) {
    let Some(exit) = exit else { return };
    if v.has_expired() && !v.is_leaving {
        v.is_leaving = true;
        v.target = exit;
        v.path = park_map.path_excluding_start(old_cell, exit);
    }
}

/// Stricter departure trigger than plain visit expiry: cumulative satisfaction collapsed.
pub(super) fn redirect_if_leaving_early(
    v: &mut Visitor,
    park_map: &ParkMap,
    old_cell: (i32, i32, i32),
    exit: Option<(i32, i32, i32)>,
) {
    let Some(exit) = exit else { return };
    if v.should_leave_early() && !v.is_leaving {
        v.is_leaving = true;
        v.target = exit;
        v.path = park_map.path_excluding_start(old_cell, exit);
    }
}

/// Grows needs by distance moved, rolls resulting penalties into satisfaction (no gain
/// side here — that's `relieve_needs_at_arrival`).
pub(super) fn update_needs_and_satisfaction(v: &mut Visitor, distance_moved: f32) {
    grow_needs(&mut v.needs, distance_moved);

    let total_penalty: f32 = v
        .needs
        .iter()
        .map(|(need, &level)| {
            let threshold = v
                .comfort_thresholds
                .get(need)
                .copied()
                .unwrap_or(COMFORT_THRESHOLD_DEFAULT);
            penalty_for(level, threshold)
        })
        .sum();

    v.satisfaction = update_satisfaction(v.satisfaction, 0.0, total_penalty);
}

/// A visitor never stands on a building cell (buildings carry no infrastructure, cf.
/// `is_walkable`), so "using" one means standing on an adjacent walkable cell.
fn adjacent_building(park_map: &ParkMap, cell: (i32, i32, i32)) -> Option<&BuildingId> {
    let (x, y, z) = cell;
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .find_map(|(dx, dy)| park_map.get_building(x + dx, y + dy, z))
}

/// Applies needs relief for the building adjacent to a just-reached cell (if any),
/// credits its `price_per_use` to `balance` (TPM-21), and records the visit for the
/// novelty factor. No-op if nothing is adjacent.
fn relieve_needs_at_arrival(
    v: &mut Visitor,
    park_map: &ParkMap,
    catalog: &BuildingCatalog,
    balance: &mut f64,
    cell: (i32, i32, i32),
    current_tick: u64,
) {
    let Some(building) = adjacent_building(park_map, cell) else {
        return;
    };
    let Some(template) = catalog.get(&building.template_id) else {
        return;
    };

    let mut total_gain = 0.0;
    for (need, &relief) in &template.needs_relief {
        let level_before = v.needs.get(need).copied().unwrap_or(0.0);
        let threshold = v
            .comfort_thresholds
            .get(need)
            .copied()
            .unwrap_or(COMFORT_THRESHOLD_DEFAULT);
        total_gain += gain_for(relief as f32, level_before, threshold);
        relieve_need(&mut v.needs, need, relief as f32);
    }

    // Entertainment isn't declared per-template like the other needs — any Attraction
    // relieves it generically (Wiki des Formules §Besoins et satisfaction).
    if template.category == BuildingCategory::Attraction {
        let level_before = v.needs.get(ENTERTAINMENT).copied().unwrap_or(0.0);
        let threshold = v
            .comfort_thresholds
            .get(ENTERTAINMENT)
            .copied()
            .unwrap_or(COMFORT_THRESHOLD_DEFAULT);
        total_gain += gain_for(ENTERTAINMENT_RELIEF, level_before, threshold);
        relieve_need(&mut v.needs, ENTERTAINMENT, ENTERTAINMENT_RELIEF);
    } else if let Some(price) = template.price_per_use {
        // TPM-21/TPM-41: only ShopUtility is credited here — an Attraction's
        // price_per_use is always null, its revenue is the entry ticket (driven by
        // cumulative satisfaction), a separate mechanism not implemented yet.
        *balance += price as f64;
    }

    v.satisfaction = update_satisfaction(v.satisfaction, total_gain, 0.0);
    v.last_visited.insert(cell, current_tick);
}

/// Utility of a template for `v`: `needs_relief` dot product, plus the same generic
/// entertainment term `relieve_needs_at_arrival` grants on arrival for any Attraction
/// (catalog templates leave `needs_relief` empty for entertainment) — without this,
/// no Attraction could ever outscore plain wandering in `best_destination`.
fn template_utility(v: &Visitor, template: &crate::building_template::BuildingTemplate) -> f32 {
    let mut utility = utility_for(&v.needs, &template.needs_relief);
    if template.category == BuildingCategory::Attraction {
        utility += v.needs.get(ENTERTAINMENT).copied().unwrap_or(0.0) * ENTERTAINMENT_RELIEF;
    }
    utility
}

/// Picks the reachable cell with the best destination score. Only strictly positive
/// scores count — a cell with no relevant building never outscores wandering. `coût`
/// adds the estimated queue wait (0 if the adjacent building has no active queue) to
/// the plain movement cost, so a busy queue naturally loses out to other targets — and
/// a queue already at capacity is excluded outright (renoncement): a visitor never
/// walks toward a full queue only to find no room, it picks another target immediately.
fn best_destination(
    v: &Visitor,
    park_map: &ParkMap,
    catalog: &BuildingCatalog,
    queues: &HashMap<String, QueueState>,
    old_cell: (i32, i32, i32),
    current_tick: u64,
) -> Option<(i32, i32, i32)> {
    let diameter = visitor_diameter();
    park_map
        .infrastructure
        .keys()
        .filter(|&&cell| cell != old_cell)
        .filter_map(|&cell| {
            let building = adjacent_building(park_map, cell);
            let queue = building.and_then(|b| queues.get(&b.building_id));
            if queue.is_some_and(|q| q.is_full(diameter)) {
                return None;
            }
            // A building with an active queue is never targeted at its door — TPM-156:
            // the real destination is the queue's entrance (chain tail), so a visitor
            // walks to the back of the line instead of cutting straight to the front.
            let target_cell = match queue {
                Some(q) if !q.chain.is_empty() => *q.chain.last().unwrap(),
                _ => cell,
            };
            let (_, movement_cost) = park_map.find_path(old_cell, target_cell)?;
            let template = building.and_then(|b| catalog.get(&b.template_id));
            let utility = template.map(|t| template_utility(v, t)).unwrap_or(0.0);
            let affinity = template
                .map(|template| affinity_for(&v.profile, &template.tags))
                .unwrap_or(AFFINITY_DEFAULT);
            let novelty = novelty_for(v.last_visited.get(&target_cell).copied(), current_tick);
            let wait = match (queue, template) {
                (Some(queue), Some(t)) => estimated_wait_for(queue, t),
                _ => 0.0,
            };
            let cost = movement_cost as f32 + wait;
            let score = score_for(utility, affinity, novelty, cost);
            Some((target_cell, score))
        })
        .filter(|&(_, score)| score > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(cell, _)| cell)
}

pub(super) fn assign_new_target_if_arrived(
    v: &mut Visitor,
    park_map: &ParkMap,
    catalog: &BuildingCatalog,
    queues: &mut HashMap<String, QueueState>,
    balance: &mut f64,
    old_cell: (i32, i32, i32),
    current_tick: u64,
) {
    if v.path.is_empty() && !v.is_leaving {
        // Just reached a queue's entrance: hand off to FIFO advancement (TPM-45)
        // instead of the ordinary arrival cycle — no needs relief, no A* target here.
        if let Some(attraction_id) = queue_at_entrance(queues, old_cell) {
            let attraction_id = attraction_id.to_string();
            if join_queue(v, queues, &attraction_id) {
                return;
            }
            // Queue filled up on the way here: renoncement, fall through and re-target.
        }

        relieve_needs_at_arrival(v, park_map, catalog, balance, old_cell, current_tick);

        let new_target = best_destination(v, park_map, catalog, queues, old_cell, current_tick)
            .or_else(|| park_map.random_walkable_cell(old_cell))
            .unwrap_or(old_cell);
        v.target = new_target;
        v.path = park_map.path_excluding_start(old_cell, new_target);
    }
}

pub(super) fn recompute_path_if_blocked(
    v: &mut Visitor,
    park_map: &ParkMap,
    old_cell: (i32, i32, i32),
) {
    if let Some(&next) = v.path.first()
        && !park_map.is_walkable(next.0, next.1, next.2)
    {
        v.path = park_map.path_excluding_start(old_cell, v.target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balance::VISITOR_RADIUS;
    use crate::building_template::CatalogSource;
    use crate::map::{Bounds3d, InfrastructureShape, ParkMap};

    mod redirect_if_expired {
        use super::*;
        use crate::balance::VISIT_DURATION_TICKS;

        fn expired_visitor_at(position: (i32, i32, i32)) -> Visitor {
            Visitor {
                id: "a".into(),
                position: (position.0 as f32, position.1 as f32, position.2 as f32),
                path: vec![],
                target: position,
                ticks_since_spawn: VISIT_DURATION_TICKS,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            }
        }

        #[test]
        fn test_does_nothing_when_there_is_no_exit() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = expired_visitor_at((0, 0, 0));

            redirect_if_expired(&mut v, &park_map, (0, 0, 0), None);

            assert!(!v.is_leaving);
        }

        #[test]
        fn test_does_nothing_when_visitor_has_not_expired() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = expired_visitor_at((0, 0, 0));
            v.ticks_since_spawn = 0;

            redirect_if_expired(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert!(!v.is_leaving);
        }

        #[test]
        fn test_does_not_overwrite_target_when_visitor_is_already_leaving() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = expired_visitor_at((0, 0, 0));
            v.is_leaving = true;
            v.target = (3, 3, 0);

            redirect_if_expired(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert_eq!(v.target, (3, 3, 0));
        }

        #[test]
        fn test_sets_is_leaving_and_retargets_exit_when_expired() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = expired_visitor_at((0, 0, 0));

            redirect_if_expired(&mut v, &park_map, (0, 0, 0), Some((4, 4, 0)));

            assert!(v.is_leaving);
            assert_eq!(v.target, (4, 4, 0));
        }

        #[test]
        fn test_computes_path_toward_exit() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = expired_visitor_at((0, 0, 0));

            redirect_if_expired(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert_eq!(v.path, vec![(1, 0, 0)]);
        }
    }

    mod redirect_if_leaving_early {
        use super::*;
        use crate::balance::EARLY_DEPARTURE_SATISFACTION_THRESHOLD;

        fn dissatisfied_visitor_at(position: (i32, i32, i32)) -> Visitor {
            Visitor {
                id: "a".into(),
                position: (position.0 as f32, position.1 as f32, position.2 as f32),
                path: vec![],
                target: position,
                satisfaction: EARLY_DEPARTURE_SATISFACTION_THRESHOLD - 1.0,
                is_leaving: false,
                ..Default::default()
            }
        }

        #[test]
        fn test_does_nothing_when_there_is_no_exit() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = dissatisfied_visitor_at((0, 0, 0));

            redirect_if_leaving_early(&mut v, &park_map, (0, 0, 0), None);

            assert!(!v.is_leaving);
        }

        #[test]
        fn test_does_nothing_when_satisfaction_is_above_the_threshold() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = dissatisfied_visitor_at((0, 0, 0));
            v.satisfaction = 0.0;

            redirect_if_leaving_early(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert!(!v.is_leaving);
        }

        #[test]
        fn test_does_not_overwrite_target_when_visitor_is_already_leaving() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = dissatisfied_visitor_at((0, 0, 0));
            v.is_leaving = true;
            v.target = (3, 3, 0);

            redirect_if_leaving_early(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert_eq!(v.target, (3, 3, 0));
        }

        #[test]
        fn test_sets_is_leaving_and_retargets_exit_when_dissatisfied() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = dissatisfied_visitor_at((0, 0, 0));

            redirect_if_leaving_early(&mut v, &park_map, (0, 0, 0), Some((4, 4, 0)));

            assert!(v.is_leaving);
            assert_eq!(v.target, (4, 4, 0));
        }

        #[test]
        fn test_computes_path_toward_exit() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = dissatisfied_visitor_at((0, 0, 0));

            redirect_if_leaving_early(&mut v, &park_map, (0, 0, 0), Some((1, 0, 0)));

            assert_eq!(v.path, vec![(1, 0, 0)]);
        }
    }

    mod update_needs_and_satisfaction {
        use super::*;

        #[test]
        fn test_grows_every_core_need() {
            let mut v = Visitor::new("a".into(), (0.0, 0.0, 0.0));

            update_needs_and_satisfaction(&mut v, 0.0);

            for need in crate::visitor::CORE_NEEDS {
                assert!(v.needs[need] > 0.0, "{need} should have grown");
            }
        }

        #[test]
        fn test_satisfaction_stays_neutral_while_all_needs_are_comfortable() {
            let mut v = Visitor::new("a".into(), (0.0, 0.0, 0.0));

            update_needs_and_satisfaction(&mut v, 0.0);

            // A single tick of growth from 0 stays far under any comfort threshold,
            // so no penalty should apply yet.
            assert_eq!(v.satisfaction, 0.0);
        }

        #[test]
        fn test_satisfaction_drops_once_a_need_crosses_its_comfort_threshold() {
            let mut v = Visitor::new("a".into(), (0.0, 0.0, 0.0));
            for need in crate::visitor::CORE_NEEDS {
                v.needs.insert(need.to_string(), 100.0);
                v.comfort_thresholds.insert(need.to_string(), 70.0);
            }

            update_needs_and_satisfaction(&mut v, 0.0);

            assert!(v.satisfaction < 0.0);
        }

        #[test]
        fn test_missing_comfort_threshold_falls_back_to_the_default() {
            let mut v = Visitor::new("a".into(), (0.0, 0.0, 0.0));
            v.needs.insert(crate::visitor::HUNGER.to_string(), 100.0);
            v.comfort_thresholds.remove(crate::visitor::HUNGER);

            // Should not panic looking up a missing threshold, and should still
            // penalize the over-threshold need using COMFORT_THRESHOLD_DEFAULT.
            update_needs_and_satisfaction(&mut v, 0.0);

            assert!(v.satisfaction < 0.0);
        }
    }

    mod assign_new_target_if_arrived {
        use super::*;

        fn arrived_visitor() -> Visitor {
            Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path: vec![],
                target: (0, 0, 0),
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            }
        }

        fn empty_catalog() -> BuildingCatalog {
            BuildingCatalog::default()
        }

        fn catalog_with_snack_stand() -> BuildingCatalog {
            BuildingCatalog::load(CatalogSource::Embedded(
                r#"{
                    "templates": [
                        {
                            "template_id": "snack_stand",
                            "name": "Snack Stand",
                            "category": "ShopUtility",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "short_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": { "hunger": 40 },
                            "tags": []
                        }
                    ]
                }"#,
            ))
            .unwrap()
        }

        fn catalog_with_priced_snack_stand() -> BuildingCatalog {
            BuildingCatalog::load(CatalogSource::Embedded(
                r#"{
                    "templates": [
                        {
                            "template_id": "snack_stand",
                            "name": "Snack Stand",
                            "category": "ShopUtility",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "short_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": { "hunger": 40 },
                            "tags": [],
                            "price_per_use": 8
                        }
                    ]
                }"#,
            ))
            .unwrap()
        }

        // TPM-41: an Attraction's `price_per_use` is always null in the real catalog, but
        // this fixture sets one anyway to prove the credit is gated on category, not just
        // on the field being present.
        fn catalog_with_priced_attraction() -> BuildingCatalog {
            BuildingCatalog::load(CatalogSource::Embedded(
                r#"{
                    "templates": [
                        {
                            "template_id": "coaster",
                            "name": "Coaster",
                            "category": "Attraction",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "long_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": {},
                            "tags": ["thrill"],
                            "price_per_use": 15
                        }
                    ]
                }"#,
            ))
            .unwrap()
        }

        fn catalog_with_thrill_and_family_rides() -> BuildingCatalog {
            BuildingCatalog::load(CatalogSource::Embedded(
                r#"{
                    "templates": [
                        {
                            "template_id": "thrill_ride",
                            "name": "Thrill Ride",
                            "category": "Attraction",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "short_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": { "hunger": 10 },
                            "tags": ["thrill"]
                        },
                        {
                            "template_id": "family_ride",
                            "name": "Family Ride",
                            "category": "Attraction",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "short_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": { "hunger": 10 },
                            "tags": ["family"]
                        }
                    ]
                }"#,
            ))
            .unwrap()
        }

        fn catalog_with_attraction_with_no_declared_needs_relief() -> BuildingCatalog {
            // Mirrors the real catalog: every Attraction template has an empty
            // `needs_relief` (entertainment relief is only granted on arrival).
            BuildingCatalog::load(CatalogSource::Embedded(
                r#"{
                    "templates": [
                        {
                            "template_id": "coaster",
                            "name": "Coaster",
                            "category": "Attraction",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "long_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": {},
                            "tags": ["thrill"]
                        }
                    ]
                }"#,
            ))
            .unwrap()
        }

        fn catalog_with_two_identical_coasters() -> BuildingCatalog {
            BuildingCatalog::load(CatalogSource::Embedded(
                r#"{
                    "templates": [
                        {
                            "template_id": "coaster",
                            "name": "Coaster",
                            "category": "Attraction",
                            "footprint": [[0,0]],
                            "cost": 100,
                            "visitor_behavior": "long_stay",
                            "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                            "needs_relief": {},
                            "tags": ["thrill"],
                            "cycle_capacity": 1,
                            "cycle_duration_ticks": 1000
                        }
                    ]
                }"#,
            ))
            .unwrap()
        }

        #[test]
        fn test_prefers_the_attraction_with_the_shorter_estimated_queue_wait() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(-5, 5, -5, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path); // adjacent to busy
            park_map.set_infrastructure(-1, 0, 0, InfrastructureShape::Path); // adjacent to free
            park_map.set_building(
                2,
                0,
                0,
                BuildingId {
                    building_id: "busy".into(),
                    template_id: "coaster".into(),
                },
            );
            park_map.set_building(
                -2,
                0,
                0,
                BuildingId {
                    building_id: "free".into(),
                    template_id: "coaster".into(),
                },
            );
            let mut queues: HashMap<String, QueueState> = HashMap::new();
            queues.insert(
                "busy".to_string(),
                QueueState {
                    chain: vec![(1, 0, 0)],
                    occupants: (0..5).map(|i| i.to_string()).collect(),
                },
            );
            let mut v = arrived_visitor();
            v.needs
                .insert(crate::visitor::ENTERTAINMENT.to_string(), 90.0);

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_two_identical_coasters(),
                &mut queues,
                &mut 0.0,
                (0, 0, 0),
                0,
            );

            assert_eq!(v.target, (-1, 0, 0)); // free: no queue wait, same utility/affinity/cost otherwise
        }

        #[test]
        fn test_excludes_a_queue_at_capacity_even_though_it_would_otherwise_score_highest() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(-5, 5, -5, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path); // adjacent to the full attraction
            park_map.set_building(
                2,
                0,
                0,
                BuildingId {
                    building_id: "full".into(),
                    template_id: "coaster".into(),
                },
            );
            let mut queues: HashMap<String, QueueState> = HashMap::new();
            queues.insert(
                "full".to_string(),
                QueueState {
                    chain: vec![(1, 0, 0)], // capacity 1 (single-cell chain)
                    occupants: vec!["someone_already_waiting".to_string()].into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs
                .insert(crate::visitor::ENTERTAINMENT.to_string(), 90.0); // strongly wants the attraction

            // Direct call: the only candidate scores highest on utility/affinity/cost,
            // but must still be excluded outright since its queue is already full.
            let result = best_destination(
                &v,
                &park_map,
                &catalog_with_two_identical_coasters(),
                &queues,
                (0, 0, 0),
                0,
            );

            assert_eq!(result, None);
        }

        #[test]
        fn test_targets_the_queue_entrance_never_the_door_adjacent_cell() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(-5, 5, -5, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(
                1,
                0,
                0,
                InfrastructureShape::Queue {
                    attraction_id: BuildingId {
                        building_id: "coaster".into(),
                        template_id: "coaster".into(),
                    },
                },
            ); // head of the chain, adjacent to the building's door
            park_map.set_infrastructure(
                2,
                0,
                0,
                InfrastructureShape::Queue {
                    attraction_id: BuildingId {
                        building_id: "coaster".into(),
                        template_id: "coaster".into(),
                    },
                },
            ); // tail: the actual entrance
            park_map.set_building(
                1,
                1,
                0,
                BuildingId {
                    building_id: "coaster".into(),
                    template_id: "coaster".into(),
                },
            ); // adjacent to the head, (1,0,0)
            let mut queues: HashMap<String, QueueState> = HashMap::new();
            queues.insert(
                "coaster".to_string(),
                QueueState {
                    chain: vec![(1, 0, 0), (2, 0, 0)], // head -> tail
                    occupants: Default::default(),
                },
            );
            let mut v = arrived_visitor();
            v.needs
                .insert(crate::visitor::ENTERTAINMENT.to_string(), 90.0);

            let result = best_destination(
                &v,
                &park_map,
                &catalog_with_two_identical_coasters(),
                &queues,
                (0, 0, 0),
                0,
            );

            assert_eq!(
                result,
                Some((2, 0, 0)),
                "must target the queue's tail (entrance), never the door-adjacent head cell"
            );
        }

        #[test]
        fn test_prefers_an_attraction_with_no_declared_needs_relief_over_wandering_when_entertainment_is_urgent()
         {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path); // plain candidate
            park_map.set_infrastructure(2, 0, 0, InfrastructureShape::Path); // adjacent to the coaster
            park_map.set_building(
                3,
                0,
                0,
                BuildingId {
                    building_id: "b1".into(),
                    template_id: "coaster".into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs
                .insert(crate::visitor::ENTERTAINMENT.to_string(), 90.0);

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_attraction_with_no_declared_needs_relief(),
                &mut HashMap::new(),
                &mut 0.0,
                (0, 0, 0),
                0,
            );

            assert_eq!(v.target, (2, 0, 0));
        }

        #[test]
        fn test_prefers_the_building_matching_the_visitors_profile_affinity_when_utility_and_cost_are_equal()
         {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(-5, 5, -5, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path); // adjacent to family_ride
            park_map.set_infrastructure(-1, 0, 0, InfrastructureShape::Path); // adjacent to thrill_ride
            park_map.set_building(
                2,
                0,
                0,
                BuildingId {
                    building_id: "family".into(),
                    template_id: "family_ride".into(),
                },
            );
            park_map.set_building(
                -2,
                0,
                0,
                BuildingId {
                    building_id: "thrill".into(),
                    template_id: "thrill_ride".into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs.insert(crate::visitor::HUNGER.to_string(), 90.0);
            v.profile = crate::visitor::visitor_profiles()
                .into_iter()
                .find(|p| p.name == "Ados")
                .unwrap();

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_thrill_and_family_rides(),
                &mut HashMap::new(),
                &mut 0.0,
                (0, 0, 0),
                0,
            );

            assert_eq!(v.target, (-1, 0, 0)); // Ados: thrill affinity beats family affinity
        }

        #[test]
        fn test_does_nothing_when_path_is_not_empty() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = arrived_visitor();
            v.path = vec![(2, 2, 0)];

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &empty_catalog(),
                &mut HashMap::new(),
                &mut 0.0,
                (0, 0, 0),
                0,
            );

            assert_eq!(v.path, vec![(2, 2, 0)]);
        }

        #[test]
        fn test_does_nothing_when_visitor_is_leaving() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = arrived_visitor();
            v.is_leaving = true;

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &empty_catalog(),
                &mut HashMap::new(),
                &mut 0.0,
                (0, 0, 0),
                0,
            );

            assert!(v.path.is_empty());
            assert_eq!(v.target, (0, 0, 0));
        }

        #[test]
        fn test_falls_back_to_random_walk_when_no_building_scores_positively() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = arrived_visitor();

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &empty_catalog(),
                &mut HashMap::new(),
                &mut 0.0,
                (0, 0, 0),
                0,
            );

            assert_eq!(v.target, (1, 0, 0)); // only other candidate, deterministic
            assert_eq!(v.path, vec![(1, 0, 0)]);
        }

        #[test]
        fn test_stays_immobile_with_no_panic_when_no_other_cell_is_reachable() {
            // Only walkable cell is the visitor's own; must not re-freeze by retargeting itself.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            let mut v = arrived_visitor();

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &empty_catalog(),
                &mut HashMap::new(),
                &mut 0.0,
                (0, 0, 0),
                0,
            );

            assert!(v.path.is_empty());
        }

        #[test]
        fn test_prefers_a_cell_adjacent_to_a_relevant_building_over_plain_wandering() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path); // plain candidate
            park_map.set_infrastructure(2, 0, 0, InfrastructureShape::Path); // link to keep the path contiguous
            park_map.set_infrastructure(3, 0, 0, InfrastructureShape::Path); // adjacent to the stand
            park_map.set_building(
                4,
                0,
                0,
                BuildingId {
                    building_id: "b1".into(),
                    template_id: "snack_stand".into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs.insert(crate::visitor::HUNGER.to_string(), 90.0); // starving

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_snack_stand(),
                &mut HashMap::new(),
                &mut 0.0,
                (0, 0, 0),
                0,
            );

            assert_eq!(v.target, (3, 0, 0));
        }

        #[test]
        fn test_relieves_needs_and_records_the_visit_on_arrival_next_to_a_building() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_building(
                1,
                0,
                0,
                BuildingId {
                    building_id: "b1".into(),
                    template_id: "snack_stand".into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs.insert(crate::visitor::HUNGER.to_string(), 90.0);
            v.comfort_thresholds
                .insert(crate::visitor::HUNGER.to_string(), 70.0);

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_snack_stand(),
                &mut HashMap::new(),
                &mut 0.0,
                (0, 0, 0),
                42,
            );

            assert_eq!(v.needs[crate::visitor::HUNGER], 50.0); // 90 - 40 relief
            assert!(v.satisfaction > 0.0, "relief should have granted a gain");
            assert_eq!(v.last_visited.get(&(0, 0, 0)), Some(&42));
        }

        #[test]
        fn test_credits_the_balance_with_the_shoputilitys_price_per_use_on_arrival() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_building(
                1,
                0,
                0,
                BuildingId {
                    building_id: "b1".into(),
                    template_id: "snack_stand".into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs.insert(crate::visitor::HUNGER.to_string(), 90.0);
            let mut balance = 1000.0;

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_priced_snack_stand(),
                &mut HashMap::new(),
                &mut balance,
                (0, 0, 0),
                0,
            );

            assert_eq!(balance, 1008.0); // 1000 + price_per_use (8)
        }

        #[test]
        fn test_never_credits_the_balance_for_an_attraction_even_with_a_declared_price_per_use() {
            // TPM-41: an Attraction's revenue is the entry ticket (satisfaction-driven), a
            // separate mechanism not implemented yet — never this direct per-use credit.
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_building(
                1,
                0,
                0,
                BuildingId {
                    building_id: "b1".into(),
                    template_id: "coaster".into(),
                },
            );
            let mut v = arrived_visitor();
            v.needs
                .insert(crate::visitor::ENTERTAINMENT.to_string(), 90.0);
            let mut balance = 1000.0;

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &catalog_with_priced_attraction(),
                &mut HashMap::new(),
                &mut balance,
                (0, 0, 0),
                0,
            );

            assert_eq!(balance, 1000.0);
        }

        #[test]
        fn test_joins_the_queue_instead_of_the_ordinary_arrival_cycle_when_arriving_at_its_entrance()
         {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(-5, 5, -5, 5, -1, 1));
            let mut queues: HashMap<String, QueueState> = HashMap::new();
            queues.insert(
                "coaster".to_string(),
                QueueState {
                    chain: vec![(1, 0, 0), (2, 0, 0)], // (2,0,0) is the entrance
                    occupants: Default::default(),
                },
            );
            let mut v = arrived_visitor();
            v.position = (2.0, 0.0, 0.0);
            v.needs.insert(crate::visitor::HUNGER.to_string(), 90.0); // would normally relieve needs here

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &empty_catalog(),
                &mut queues,
                &mut 0.0,
                (2, 0, 0),
                0,
            );

            assert_eq!(v.queue_attraction, Some("coaster".to_string()));
            assert_eq!(queues["coaster"].occupants, vec!["a".to_string()]);
            assert!(v.path.is_empty());
            // No ordinary arrival side effect: needs relief never ran for this arrival.
            assert_eq!(v.needs[crate::visitor::HUNGER], 90.0);
        }

        #[test]
        fn test_renounces_and_picks_a_new_target_when_the_queue_filled_up_before_arrival() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(-5, 5, -5, 5, -1, 1));
            park_map.set_infrastructure(2, 0, 0, InfrastructureShape::Path); // old_cell itself
            park_map.set_infrastructure(3, 0, 0, InfrastructureShape::Path); // alternate wander target
            let mut queues: HashMap<String, QueueState> = HashMap::new();
            queues.insert(
                "coaster".to_string(),
                QueueState {
                    chain: vec![(1, 0, 0), (2, 0, 0)],
                    occupants: Default::default(),
                },
            );
            // Fill to exact capacity rather than guessing a number, so the test doesn't
            // silently start passing/failing for the wrong reason if capacity math changes.
            let capacity =
                crate::queue::queue_capacity(&queues["coaster"].chain, 2.0 * VISITOR_RADIUS);
            queues.get_mut("coaster").unwrap().occupants =
                (0..capacity).map(|i| format!("filler{i}")).collect();
            let mut v = arrived_visitor();
            v.position = (2.0, 0.0, 0.0);

            assign_new_target_if_arrived(
                &mut v,
                &park_map,
                &empty_catalog(),
                &mut queues,
                &mut 0.0,
                (2, 0, 0),
                0,
            );

            assert_eq!(
                v.queue_attraction, None,
                "must not have joined a full queue"
            );
            assert_eq!(
                queues["coaster"].occupants.len(),
                capacity,
                "occupant count unchanged"
            );
            assert!(
                !v.path.is_empty(),
                "should have picked a new target instead"
            );
        }
    }

    mod recompute_path_if_blocked {
        use super::*;

        fn visitor_with_path(path: Vec<(i32, i32, i32)>, target: (i32, i32, i32)) -> Visitor {
            Visitor {
                id: "a".into(),
                position: (0.0, 0.0, 0.0),
                path,
                target,
                ticks_since_spawn: 0,
                heading: (0.0, 0.0, 0.0),
                is_leaving: false,
                ..Default::default()
            }
        }

        #[test]
        fn test_does_nothing_when_path_is_empty() {
            let park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            let mut v = visitor_with_path(vec![], (0, 0, 0));

            recompute_path_if_blocked(&mut v, &park_map, (0, 0, 0));

            assert!(v.path.is_empty());
        }

        #[test]
        fn test_does_nothing_when_next_cell_is_still_walkable() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 0, 0, InfrastructureShape::Path);
            let mut v = visitor_with_path(vec![(1, 0, 0)], (1, 0, 0));

            recompute_path_if_blocked(&mut v, &park_map, (0, 0, 0));

            assert_eq!(v.path, vec![(1, 0, 0)]);
        }

        #[test]
        fn test_recomputes_path_when_next_cell_becomes_unwalkable() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(0, 1, 0, InfrastructureShape::Path);
            park_map.set_infrastructure(1, 1, 0, InfrastructureShape::Path);
            // (1,0,0) is deliberately left without infrastructure: simulates the blocked cell
            let mut v = visitor_with_path(vec![(1, 0, 0)], (1, 1, 0));

            recompute_path_if_blocked(&mut v, &park_map, (0, 0, 0));

            assert_ne!(v.path.first(), Some(&(1, 0, 0)));
            assert!(!v.path.is_empty(), "an alternate route exists via (0,1,0)");
        }

        #[test]
        fn test_clears_path_when_no_alternate_route_exists() {
            let mut park_map = ParkMap::new("m".into(), Bounds3d::new(0, 5, 0, 5, -1, 1));
            park_map.set_infrastructure(0, 0, 0, InfrastructureShape::Path);
            // (1,0,0) is both the target and now unwalkable, no other route exists
            let mut v = visitor_with_path(vec![(1, 0, 0)], (1, 0, 0));

            recompute_path_if_blocked(&mut v, &park_map, (0, 0, 0));

            assert!(v.path.is_empty());
        }
    }
}
