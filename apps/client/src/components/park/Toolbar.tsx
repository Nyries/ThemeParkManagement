import { Building2, Eraser, Mountain, Route } from "lucide-react";
import { Button } from "../ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "../ui/tooltip";
import type { ToolMode } from "@/types/park/tool";

interface ToolbarProps {
  mode: ToolMode;
  onModeChange: (mode: ToolMode) => void;
}

const TOOLS: {
  mode: Exclude<ToolMode, null>;
  label: string;
  shortcut: string;
  icon: React.ComponentType<{ className?: string }>;
}[] = [
  { mode: "terrain", label: "Terrain", shortcut: "1", icon: Mountain },
  { mode: "infrastructure", label: "Chemin", shortcut: "2", icon: Route },
  { mode: "building", label: "Bâtiment", shortcut: "3", icon: Building2 },
  { mode: "remove", label: "Retirer", shortcut: "4", icon: Eraser },
];

export function Toolbar({ mode, onModeChange }: ToolbarProps) {
  return (
    <TooltipProvider>
      <div className="flex gap-1 rounded-lg border border-border bg-white/80 p-1 shadow-sm backdrop-blur-sm">
        {TOOLS.map((tool) => {
          const Icon = tool.icon;
          const active = mode === tool.mode;
          return (
            <Tooltip key={tool.mode}>
              <TooltipTrigger asChild>
                <Button
                  type="button"
                  variant={active ? "default" : "ghost"}
                  size="icon-sm"
                  aria-pressed={active}
                  aria-label={tool.label}
                  onClick={() => onModeChange(active ? null : tool.mode)}
                >
                  <Icon className="size-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                {tool.label} ({tool.shortcut})
              </TooltipContent>
            </Tooltip>
          );
        })}
      </div>
    </TooltipProvider>
  );
}
