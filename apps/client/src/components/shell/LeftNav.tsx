import { Globe, LayoutGrid, Store, Users, Wallet, Map } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { Button } from "../ui/button";

interface NavEntry {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  active: boolean;
  path: string;
}

const MAIN_NAV_ENTRIES: NavEntry[] = [
  { label: "Parc", icon: LayoutGrid, active: true, path: "" },
  { label: "Personnel", icon: Users, active: false, path: "" },
  { label: "Finances", icon: Wallet, active: false, path: "" },
  { label: "Marché", icon: Store, active: false, path: "" },
  { label: "Monde", icon: Globe, active: false, path: "" },
];

const BOTTOM_NAV_ENTRIES: NavEntry[] = [
  { label: "Éditeur de Carte", icon: Map, active: true, path: "/editor" },
];

export function LeftNav() {
  return (
    <TooltipProvider>
      <nav className="flex h-full w-60 shrink-0 flex-col gap-1 bg-sidebar p-3 text-sidebar-foreground">
        {MAIN_NAV_ENTRIES.map((entry) => (
          <NavItem key={entry.label} entry={entry} />
        ))}
        <div className="mt-auto flex flex-col gap-1 pt-4">
          {BOTTOM_NAV_ENTRIES.map((entry) => (
            <NavItem key={entry.label} entry={entry} />
          ))}
        </div>
        <span className="px-3 pt-1 text-xs text-sidebar-foreground/40">
          v{__APP_VERSION__}
        </span>
      </nav>
    </TooltipProvider>
  );
}

function NavItem({ entry }: { entry: NavEntry }) {
  const Icon = entry.icon;
  const button = (
    <Button
      variant={entry.active ? "secondary" : "ghost"}
      disabled={!entry.active}
      aria-disabled={!entry.active}
    >
      <Icon className="size-4 shrink-0" />
      {entry.label}
    </Button>
  );

  if (entry.active) {
    return button;
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span className="inline-block w-full cursor-not-allowed">
          {button}
        </span>
      </TooltipTrigger>
      <TooltipContent side="right">Bientôt disponible</TooltipContent>
    </Tooltip>
  );
}
