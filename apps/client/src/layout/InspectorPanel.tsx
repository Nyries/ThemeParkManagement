import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";
import {
  BUILDING_CATALOG,
  DEFAULT_BUILDING_ID,
  type BuildingCategory,
} from "@/park/buildingCatalog";
import { DEFAULT_MATERIAL_ID, materialColor, TERRAIN_MATERIALS } from "@/park/materials";
import type { SelectionInfo } from "@/park/selection";
import type { ToolState } from "@/park/tool";

const BUILDING_CATEGORIES: { id: BuildingCategory; label: string }[] = [
  { id: "ShopUtility", label: "Commodités" },
  { id: "Attraction", label: "Attractions" },
];

interface InspectorPanelProps {
  selection: SelectionInfo | null;
  tool: ToolState;
  onToolChange: (tool: ToolState) => void;
}

export function InspectorPanel({ selection, tool, onToolChange }: InspectorPanelProps) {
  return (
    <aside className="flex h-full w-80 shrink-0 flex-col overflow-y-auto border-l border-border p-4">
      <ToolContextContent selection={selection} tool={tool} onToolChange={onToolChange} />

      <Separator className="my-4" />

      <div>
        <h3 className="mb-2 text-xs font-medium text-muted-foreground">
          Journal
        </h3>
        <p className="text-sm text-muted-foreground">
          Journal des événements — à venir.
        </p>
      </div>
    </aside>
  );
}

function ToolContextContent({
  selection,
  tool,
  onToolChange,
}: InspectorPanelProps) {
  switch (tool.mode) {
    case "terrain":
      return (
        <div>
          <h2 className="mb-2 text-sm font-semibold text-foreground">
            Matériau
          </h2>
          <div className="flex flex-wrap gap-2">
            {TERRAIN_MATERIALS.map((material) => {
              const active =
                (tool.selectedMaterialId ?? DEFAULT_MATERIAL_ID) ===
                material.id;
              return (
                <button
                  key={material.id}
                  type="button"
                  aria-pressed={active}
                  onClick={() =>
                    onToolChange({ ...tool, selectedMaterialId: material.id })
                  }
                  className={cn(
                    "flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm transition-colors",
                    active
                      ? "border-primary bg-primary/10 text-foreground"
                      : "border-border text-muted-foreground hover:bg-muted hover:text-foreground",
                  )}
                >
                  <span
                    aria-hidden="true"
                    className="size-3 rounded-full border border-black/10"
                    style={{ backgroundColor: materialColor(material.id) }}
                  />
                  {material.label}
                </button>
              );
            })}
          </div>
        </div>
      );
    case "building": {
      const selectedId = tool.selectedBuildingId ?? DEFAULT_BUILDING_ID;
      return (
        <div>
          <h2 className="mb-2 text-sm font-semibold text-foreground">
            Catalogue
          </h2>
          {BUILDING_CATEGORIES.map((category) => (
            <div key={category.id} className="mb-3 last:mb-0">
              <h3 className="mb-1 text-xs font-medium text-muted-foreground">
                {category.label}
              </h3>
              <div className="flex flex-col gap-1">
                {BUILDING_CATALOG.filter(
                  (building) => building.category === category.id,
                ).map((building) => {
                  const active = selectedId === building.templateId;
                  return (
                    <button
                      key={building.templateId}
                      type="button"
                      aria-pressed={active}
                      onClick={() =>
                        onToolChange({
                          ...tool,
                          selectedBuildingId: building.templateId,
                        })
                      }
                      className={cn(
                        "rounded-md border px-3 py-1.5 text-left text-sm transition-colors",
                        active
                          ? "border-primary bg-primary/10 text-foreground"
                          : "border-border text-muted-foreground hover:bg-muted hover:text-foreground",
                      )}
                    >
                      {building.name}
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      );
    }
    case "infrastructure":
      return (
        <h2 className="text-sm font-semibold text-foreground">
          Tracer un chemin sur le canvas
        </h2>
      );
    case "remove":
      return (
        <h2 className="text-sm font-semibold text-foreground">
          Cliquer un élément à retirer
        </h2>
      );
    default:
      return (
        <h2 className="text-sm font-semibold text-foreground">
          {selection ? selection.label : "Aucune sélection"}
        </h2>
      );
  }
}
