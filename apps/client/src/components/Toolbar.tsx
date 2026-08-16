import { Building2, Eraser, Mountain, Route } from "lucide-react";
import { Button } from "./ui/button";
import type { ToolMode } from "@/park/tool";

interface ToolbarProps {
  mode: ToolMode;
  onModeChange: (mode: ToolMode) => void;
}

const TOOLS: { mode: Exclude<ToolMode, null>; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
  { mode: "terrain", label: "Terrain", icon: Mountain },
  { mode: "infrastructure", label: "Chemin", icon: Route },
  { mode: "building", label: "Bâtiment", icon: Building2 },
  { mode: "remove", label: "Retirer", icon: Eraser },
];

export function Toolbar({ mode, onModeChange }: ToolbarProps) {
  return (
    <div className="absolute right-4 top-4 z-10 flex gap-1 rounded-lg border border-border bg-white/80 p-1 shadow-sm backdrop-blur-sm">
      {TOOLS.map((tool) => {
        const Icon = tool.icon;
        const active = mode === tool.mode;
        return (
          <Button
            key={tool.mode}
            type="button"
            variant={active ? "default" : "ghost"}
            size="icon-sm"
            aria-pressed={active}
            aria-label={tool.label}
            onClick={() => onModeChange(active ? null : tool.mode)}
          >
            <Icon className="size-4" />
          </Button>
        );
      })}
    </div>
  );
}
