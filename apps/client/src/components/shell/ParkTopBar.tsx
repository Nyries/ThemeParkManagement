import { Users, Wallet } from "lucide-react";

interface ParkTopBarProps {
  balance: number;
  visitorCount: number;
}

const balanceFormatter = new Intl.NumberFormat("fr-FR", {
  style: "currency",
  currency: "EUR",
  maximumFractionDigits: 0,
});

export function ParkTopBar({ balance, visitorCount }: ParkTopBarProps) {
  return (
    <header className="flex h-12 shrink-0 items-center justify-between border-b border-border px-4">
      <span className="text-sm font-medium text-foreground">
        Prairie Meadows
      </span>
      <div className="flex gap-2">
        <MetricCard
          icon={Wallet}
          value={balanceFormatter.format(balance)}
        />
        <MetricCard
          icon={Users}
          value={`${visitorCount} visiteur${visitorCount === 1 ? "" : "s"}`}
        />
      </div>
    </header>
  );
}

function MetricCard({
  icon: Icon,
  value,
}: {
  icon: React.ComponentType<{ className?: string }>;
  value: string;
}) {
  return (
    <div className="flex items-center gap-1.5 rounded-md border border-border bg-muted/40 px-2.5 py-1">
      <Icon className="size-3.5 text-muted-foreground" />
      <span className="text-sm font-medium text-foreground">{value}</span>
    </div>
  );
}
