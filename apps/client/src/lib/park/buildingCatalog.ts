export type BuildingCategory = "ShopUtility" | "Attraction";

export interface BuildingTemplate {
  templateId: string;
  name: string;
  category: BuildingCategory;
  footprint: { x: number; y: number }[];
}

// Stub catalogue until TPM-163 exposes it via a dedicated RPC — duplicated
// from apps/engine/assets/catalog/buildings.json (template_id/name/category/
// footprint only; cost, needs_relief, tags, etc. aren't used client-side yet).
export const BUILDING_CATALOG: BuildingTemplate[] = [
  {
    templateId: "quick_snack_stand",
    name: "Quick Snack Stand",
    category: "ShopUtility",
    footprint: [{ x: 0, y: 0 }],
  },
  {
    templateId: "sit_down_restaurant",
    name: "Sit-Down Restaurant",
    category: "ShopUtility",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 0, y: 1 },
      { x: 1, y: 1 },
    ],
  },
  {
    templateId: "drink_stand",
    name: "Drink Stand",
    category: "ShopUtility",
    footprint: [{ x: 0, y: 0 }],
  },
  {
    templateId: "premium_juice_bar",
    name: "Premium Juice Bar",
    category: "ShopUtility",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
    ],
  },
  {
    templateId: "simple_bench",
    name: "Simple Bench",
    category: "ShopUtility",
    footprint: [{ x: 0, y: 0 }],
  },
  {
    templateId: "shaded_rest_area",
    name: "Shaded Rest Area",
    category: "ShopUtility",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 0, y: 1 },
    ],
  },
  {
    templateId: "standard_restrooms",
    name: "Standard Restrooms",
    category: "ShopUtility",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
    ],
  },
  {
    templateId: "premium_restrooms",
    name: "Premium Restrooms",
    category: "ShopUtility",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 0, y: 1 },
      { x: 1, y: 1 },
    ],
  },
  {
    templateId: "souvenir_shop",
    name: "Souvenir Shop",
    category: "ShopUtility",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
    ],
  },
  {
    templateId: "candy_shop",
    name: "Candy Shop",
    category: "ShopUtility",
    footprint: [{ x: 0, y: 0 }],
  },
  {
    templateId: "souvenir_photo_stand",
    name: "Souvenir Photo Stand",
    category: "ShopUtility",
    footprint: [{ x: 0, y: 0 }],
  },
  {
    templateId: "family_carousel",
    name: "Family Carousel",
    category: "Attraction",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 0, y: 1 },
      { x: 1, y: 1 },
    ],
  },
  {
    templateId: "park_train",
    name: "Park Train",
    category: "Attraction",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 2, y: 0 },
      { x: 2, y: 1 },
    ],
  },
  {
    templateId: "ferris_wheel",
    name: "Ferris Wheel",
    category: "Attraction",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 2, y: 0 },
      { x: 1, y: 1 },
    ],
  },
  {
    templateId: "spinning_teacups",
    name: "Spinning Teacups",
    category: "Attraction",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 0, y: 1 },
      { x: 1, y: 1 },
    ],
  },
  {
    templateId: "bumper_cars",
    name: "Bumper Cars",
    category: "Attraction",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 2, y: 0 },
      { x: 0, y: 1 },
      { x: 1, y: 1 },
      { x: 2, y: 1 },
    ],
  },
  {
    templateId: "pirate_ship",
    name: "Pirate Ship",
    category: "Attraction",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 1, y: 1 },
      { x: 2, y: 1 },
    ],
  },
  {
    templateId: "drop_tower",
    name: "Drop Tower",
    category: "Attraction",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 0, y: 1 },
      { x: 1, y: 1 },
    ],
  },
  {
    templateId: "roller_coaster",
    name: "Roller Coaster",
    category: "Attraction",
    footprint: [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 2, y: 0 },
      { x: 2, y: 1 },
      { x: 2, y: 2 },
      { x: 1, y: 2 },
    ],
  },
];

export const DEFAULT_BUILDING_ID = "sit_down_restaurant";

export function findBuildingTemplate(templateId: string): BuildingTemplate {
  return (
    BUILDING_CATALOG.find((building) => building.templateId === templateId) ??
    BUILDING_CATALOG[0]
  );
}
