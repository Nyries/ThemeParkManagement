use serde::Deserialize;
use std::collections::HashMap;

pub enum CatalogSource {
    Embedded(&'static str),
    File(std::path::PathBuf),
}

#[derive(Debug)]
pub enum CatalogLoadError {
    InvalidJson(String),
    DuplicateTemplateId(String),
}

impl std::fmt::Display for CatalogLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for CatalogLoadError {}

#[derive(Deserialize)]
struct RawCatalog {
    templates: Vec<BuildingTemplate>,
}

#[derive(Debug, Default)]
pub struct BuildingCatalog {
    templates: HashMap<String, BuildingTemplate>,
}

impl BuildingCatalog {
    pub fn load(source: CatalogSource) -> Result<BuildingCatalog, CatalogLoadError> {
        let raw = match source {
            CatalogSource::Embedded(json) => json.to_string(),
            CatalogSource::File(path) => std::fs::read_to_string(&path)
                .map_err(|e| CatalogLoadError::InvalidJson(e.to_string()))?,
        };
        let raw_catalog: RawCatalog =
            serde_json::from_str(&raw).map_err(|e| CatalogLoadError::InvalidJson(e.to_string()))?;

        let mut templates = HashMap::new();
        for template in raw_catalog.templates {
            if templates.contains_key(&template.template_id) {
                return Err(CatalogLoadError::DuplicateTemplateId(template.template_id));
            }
            templates.insert(template.template_id.clone(), template);
        }
        Ok(BuildingCatalog { templates })
    }

    pub fn get(&self, template_id: &str) -> Option<&BuildingTemplate> {
        self.templates.get(template_id)
    }
}

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
    pub template_id: String,
    pub name: String,
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
    pub cycle_capacity: Option<u32>,
    pub cycle_duration_ticks: Option<u32>,
    pub price_per_use: Option<u32>,
    pub unlock_biome: Option<String>,
}

#[cfg(test)]
mod test {
    use super::*;

    mod load {
        use super::*;

        const VALID_JSON: &str = r#"{
            "templates": [
                {
                    "template_id": "snack_rapide",
                    "name": "Snack rapide",
                    "category": "ShopUtility",
                    "footprint": [[0,0]],
                    "cost": 500,
                    "visitor_behavior": "short_stay",
                    "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                    "construction_time_ticks": null,
                    "needs_relief": { "faim": 30 },
                    "resource_vector": null,
                    "tags": [],
                    "intensity": null,
                    "cycle_capacity": null,
                    "cycle_duration_ticks": null,
                    "price_per_use": 8,
                    "unlock_biome": null
                }
            ]
        }"#;

        fn write_temp_json(content: &str) -> std::path::PathBuf {
            let path = std::env::temp_dir().join(format!(
                "building_catalog_test_{}.json",
                uuid::Uuid::new_v4()
            ));
            std::fs::write(&path, content).unwrap();
            path
        }

        #[test]
        fn test_load_embedded_valid_json_succeeds() {
            let result = BuildingCatalog::load(CatalogSource::Embedded(VALID_JSON));

            assert!(result.is_ok());
            let catalog = result.unwrap();
            let template = catalog.get("snack_rapide").unwrap();
            assert_eq!(template.name, "Snack rapide");
            assert_eq!(template.category, BuildingCategory::ShopUtility);
        }

        #[test]
        fn test_load_embedded_malformed_json_fails() {
            let result = BuildingCatalog::load(CatalogSource::Embedded("{ not valid json"));

            assert!(matches!(result, Err(CatalogLoadError::InvalidJson(_))));
        }

        #[test]
        fn test_load_duplicate_template_id_fails() {
            let json = r#"{
                "templates": [
                    { "template_id": "dup", "name": "A", "category": "ShopUtility", "footprint": [[0,0]], "cost": 1, "visitor_behavior": "short_stay", "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false }, "construction_time_ticks": null, "needs_relief": {}, "resource_vector": null, "tags": [], "intensity": null, "cycle_capacity": null, "cycle_duration_ticks": null, "price_per_use": 1, "unlock_biome": null },
                    { "template_id": "dup", "name": "B", "category": "ShopUtility", "footprint": [[0,0]], "cost": 1, "visitor_behavior": "short_stay", "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false }, "construction_time_ticks": null, "needs_relief": {}, "resource_vector": null, "tags": [], "intensity": null, "cycle_capacity": null, "cycle_duration_ticks": null, "price_per_use": 1, "unlock_biome": null }
                ]
            }"#;

            let result = BuildingCatalog::load(CatalogSource::Embedded(json));

            assert!(
                matches!(result, Err(CatalogLoadError::DuplicateTemplateId(id)) if id == "dup")
            );
        }

        #[test]
        fn test_get_unknown_template_id_returns_none() {
            let catalog = BuildingCatalog::load(CatalogSource::Embedded(VALID_JSON)).unwrap();

            assert!(catalog.get("does_not_exist").is_none());
        }

        #[test]
        fn test_load_file_valid_json_succeeds() {
            let path = write_temp_json(VALID_JSON);

            let result = BuildingCatalog::load(CatalogSource::File(path.clone()));

            assert!(result.is_ok());
            std::fs::remove_file(&path).unwrap();
        }

        #[test]
        fn test_load_file_malformed_json_fails() {
            let path = write_temp_json("{ not valid json");

            let result = BuildingCatalog::load(CatalogSource::File(path.clone()));

            assert!(matches!(result, Err(CatalogLoadError::InvalidJson(_))));

            std::fs::remove_file(&path).unwrap();
        }

        #[test]
        fn test_load_file_missing_fails() {
            let path = std::env::temp_dir().join("this_catalog_does_not_exist_12345.json");

            let result = BuildingCatalog::load(CatalogSource::File(path));

            assert!(matches!(result, Err(CatalogLoadError::InvalidJson(_))));
        }
    }

    mod deserialize {
        use super::*;

        #[test]
        fn test_attraction_template_deserializes_with_tags_and_intensity() {
            let json = r#"{
                "template_id": "montagnes_russes",
                "name": "Montagnes russes",
                "category": "Attraction",
                "footprint": [[0,0],[1,0],[2,0]],
                "cost": 50000,
                "visitor_behavior": "long_stay",
                "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                "construction_time_ticks": 20000,
                "needs_relief": {},
                "resource_vector": null,
                "tags": ["sensations", "spectacle"],
                "intensity": 5,
                "cycle_capacity": 20,
                "cycle_duration_ticks": 2400,
                "price_per_use": null,
                "unlock_biome": null
            }"#;

            let template: BuildingTemplate = serde_json::from_str(json).unwrap();

            assert_eq!(template.category, BuildingCategory::Attraction);
            assert_eq!(
                template.tags,
                vec!["sensations".to_string(), "spectacle".to_string()]
            );
            assert_eq!(template.intensity, Some(5));
            assert_eq!(template.cycle_duration_ticks, Some(2400));
            assert_eq!(template.price_per_use, None);
        }

        #[test]
        fn test_shop_utility_with_empty_needs_relief_is_valid() {
            let json = r#"{
                "template_id": "boutique_souvenirs",
                "name": "Boutique souvenirs",
                "category": "ShopUtility",
                "footprint": [[0,0]],
                "cost": 8000,
                "visitor_behavior": "short_stay",
                "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                "construction_time_ticks": null,
                "needs_relief": {},
                "resource_vector": null,
                "tags": [],
                "intensity": null,
                "cycle_capacity": null,
                "cycle_duration_ticks": null,
                "price_per_use": 12,
                "unlock_biome": null
            }"#;

            let template: BuildingTemplate = serde_json::from_str(json).unwrap();

            assert!(template.needs_relief.is_empty());
            assert_eq!(template.price_per_use, Some(12));
        }

        #[test]
        fn test_unknown_category_fails_to_deserialize() {
            let json = r#"{
                "template_id": "x", "name": "X", "category": "NotACategory",
                "footprint": [[0,0]], "cost": 1, "visitor_behavior": "short_stay",
                "crossing_flags": { "bridge_above_allowed": false, "tunnel_below_allowed": false },
                "construction_time_ticks": null, "needs_relief": {}, "resource_vector": null,
                "tags": [], "intensity": null, "cycle_capacity": null, "cycle_duration_ticks": null,
                "price_per_use": null, "unlock_biome": null
            }"#;

            let result: Result<BuildingTemplate, _> = serde_json::from_str(json);

            assert!(result.is_err());
        }
    }
}
