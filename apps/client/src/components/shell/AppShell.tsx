import { LeftNav } from "./LeftNav";
import { TopBar } from "./TopBar";

interface AppShellProps {
    children: React.ReactNode;
}

export function AppShell({ children }: AppShellProps) {
    return (
        <div className="flex h-screen w-screen flex-col bg-background text-foreground">
            <TopBar />
            <div className="flex flex-1 min-h-0">
                <LeftNav />
                <div className="flex-1 flex flex-col min-w-0">
                    <div className="flex-1 min-h-0 overflow-hidden">{children}</div>
                </div>
            </div>
        </div>
    )
}