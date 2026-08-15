import { InspectorPanel } from "./InspectorPanel";
import { LeftNav } from "./LeftNav";
import { ParkTopBar } from "./ParkTopBar";
import { TopBar } from "./TopBar";

export function AppShell({ children }: { children: React.ReactNode }) {
    return (
        <div className="flex h-screen w-screen flex-col bg-background text-foreground">
            <TopBar />
            <div className="flex flex-1 min-h-0">
                <LeftNav />
                <div className="flex-1 flex flex-col min-w-0">
                    <ParkTopBar />
                    <div className="flex-1 min-h-0 overflow-hidden">{children}</div>
                </div>
                <InspectorPanel selection={null} />
            </div>
        </div>
    )
}