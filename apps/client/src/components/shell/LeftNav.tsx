import { Globe, LayoutGrid, Store, Users, Wallet } from "lucide-react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

interface NavEntry {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  active: boolean;
}

const NAV_ENTRIES: NavEntry[] = [
  { label: "Parc", icon: LayoutGrid, active: true },
  { label: "Personnel", icon: Users, active: false },
  { label: "Finances", icon: Wallet, active: false },
  { label: "Marché", icon: Store, active: false },
  { label: "Monde", icon: Globe, active: false },
];

export function LeftNav() {
  return (
    <TooltipProvider>
      <nav className="flex h-full w-60 shrink-0 flex-col gap-1 bg-sidebar p-3 text-sidebar-foreground">
        {NAV_ENTRIES.map((entry) => (
          <NavItem key={entry.label} entry={entry} />
        ))}
        <span className="mt-auto px-3 text-xs text-sidebar-foreground/40">
          v{__APP_VERSION__}
        </span>
      </nav>
    </TooltipProvider>
  );
}

function NavItem({ entry }: { entry: NavEntry }) {
  const Icon = entry.icon;

  const button = (
    <button
      type="button"
      disabled={!entry.active}
      aria-disabled={!entry.active}
      className={cn(
        "flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
        entry.active
          ? "bg-sidebar-accent text-sidebar-accent-foreground"
          : "cursor-not-allowed text-sidebar-foreground/50",
      )}
    >
      <Icon className="size-4 shrink-0" />
      {entry.label}
    </button>
  );

  if (entry.active) {
    return button;
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>{button}</TooltipTrigger>
      <TooltipContent side="right">Bientôt disponible</TooltipContent>
    </Tooltip>
  );
}
