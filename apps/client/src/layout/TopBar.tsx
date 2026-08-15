import { Bell, FerrisWheel, Search } from "lucide-react";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";

export function TopBar() {
  return (
    <header className="flex h-14 w-full shrink-0 items-center justify-between gap-4 border-b border-border px-4">
      <div className="flex items-center gap-2">
        <FerrisWheel className="size-5 text-primary" />
        <span className="font-display text-sm font-semibold text-foreground">
          Park Horizon
        </span>
      </div>

      <div className="flex items-center gap-4">
        <Button variant="outline" size="sm" disabled className="gap-1.5">
          <Search className="size-4" />
          Rechercher
        </Button>

        <Separator orientation="vertical" className="h-6" />

        <div className="flex items-center gap-3">
          <Bell className="size-4 text-muted-foreground" />
          <Avatar size="sm">
            <AvatarFallback>C</AvatarFallback>
          </Avatar>
        </div>
      </div>
    </header>
  );
}
