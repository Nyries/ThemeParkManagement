import { useState } from "react";
import "./App.css";
import { Park } from "./components/park/Park";
import { AppShell } from "./components/shell/AppShell";
import { Toaster } from "./components/ui/sonner";
import type { SelectionInfo } from "./types/park/selection";
import type { ToolState } from "./types/park/tool";

function App() {
  const [tool, setTool] = useState<ToolState>({mode: null});
  const [selection, setSelection] = useState<SelectionInfo | null>(null);

  return (
    <>
      <AppShell selection={selection} tool={tool} onToolChange={setTool}>
        <Park tool={tool} onToolChange={setTool} onSelectionChange={setSelection} />
      </AppShell>
      <Toaster />
    </>
  );
}

export default App;
