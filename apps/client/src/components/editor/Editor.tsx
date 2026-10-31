import type { ToolState } from "@/types/park/tool";
import { InspectorPanel } from "../shell/InspectorPanel";
import { Button } from "../ui/button";
import { CirclePlus, Folder } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "../ui/tooltip";

interface EditorProps {
  tool: ToolState;
  onToolChange: (tool: ToolState) => void;
}

export function Editor({ tool, onToolChange }: EditorProps) {
  return (
    <div className="flex h-full w-full">
      <div className="flex flex-1 min-w-0 justify-center items-center gap-4">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="secondary"
                className="flex-1 h-1/2 justify-center max-w-sm flex-col gap-4 text-lg rounded-lg"
              >
                <Folder className="size-50"></Folder>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              Modifier une map existante
            </TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="secondary"
                className="flex-1 h-1/2 justify-center max-w-sm flex-col gap-4 text-lg rounded-lg"
              >
                <CirclePlus className="size-50"></CirclePlus>
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              Créer une nouvelle map
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>
      <InspectorPanel
        selection={null}
        tool={tool}
        onToolChange={onToolChange}
      ></InspectorPanel>
    </div>
  );
}
