import { Separator } from "@/components/ui/separator";
import type { SelectionInfo } from "@/park/selection";
import type { ToolState } from "@/park/tool";

interface InspectorPanelProps {
  selection: SelectionInfo | null;
  tool: ToolState;
  onToolChange: (tool: ToolState) => void;
}

export function InspectorPanel({ selection, tool }: InspectorPanelProps) {
  return (
    <aside className="flex h-full w-80 shrink-0 flex-col overflow-y-auto border-l border-border p-4">
      <ToolContextContent selection={selection} tool={tool} />

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
}: Pick<InspectorPanelProps, "selection" | "tool">) {
  switch (tool.mode) {
    case "terrain":
      return (
        <h2 className="text-sm font-semibold text-foreground">
          Sélection de matériau — à venir
        </h2>
      );
    case "building":
      return (
        <h2 className="text-sm font-semibold text-foreground">
          Catalogue de bâtiments — à venir
        </h2>
      );
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
