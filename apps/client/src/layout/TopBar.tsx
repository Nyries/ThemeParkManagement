import { Bell, Search } from "lucide-react";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";

export function TopBar() {
  return (
    <header className="flex h-14 w-full shrink-0 items-center justify-between border-b border-border px-4">
      <Button variant="outline" size="sm" disabled className="gap-1.5">
        <Search className="size-4" />
        Rechercher
      </Button>

      <div className="flex items-center gap-3">
        <Bell className="size-4 text-muted-foreground" />
        <Avatar size="sm">
          <AvatarFallback>C</AvatarFallback>
        </Avatar>
      </div>
    </header>
  );
}
