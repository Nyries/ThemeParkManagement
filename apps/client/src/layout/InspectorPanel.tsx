import { Separator } from "@/components/ui/separator";
import type { SelectionInfo } from "@/park/selection";

interface InspectorPanelProps {
  selection: SelectionInfo | null;
}

export function InspectorPanel({ selection }: InspectorPanelProps) {
  return (
    <aside className="flex h-full w-80 shrink-0 flex-col overflow-y-auto border-l border-border p-4">
      {selection ? (
        <div>
          <h2 className="text-sm font-semibold text-foreground">
            {selection.label}
          </h2>
        </div>
      ) : (
        <h2 className="text-sm font-semibold text-foreground">
          Aucune sélection
        </h2>
      )}

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
