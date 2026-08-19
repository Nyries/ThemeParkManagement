import { useState } from "react";
import "./App.css";
import { Park } from "./components/park/Park";
import { AppShell } from "./components/shell/AppShell";
import { Toaster } from "./components/ui/sonner";
import type { ToolState } from "./types/park/tool";

function App() {
  const [tool, setTool] = useState<ToolState>({mode: null});

  return (
    <>
      <AppShell>
        <Park tool={tool} onToolChange={setTool} />
      </AppShell>
      <Toaster />
    </>
  );
}

export default App;
