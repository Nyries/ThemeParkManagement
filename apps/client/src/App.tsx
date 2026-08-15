import { useState } from "react";
import "./App.css";
import { Park } from "./components/Park";
import { AppShell } from "./layout/AppShell";
import { Toaster } from "./components/ui/sonner";
import type { SelectionInfo } from "./park/selection";

function App() {
  const [selection, setSelection] = useState<SelectionInfo | null>(null);

  return (
    <>
      <AppShell selection={selection}>
        <Park onSelectionChange={setSelection} />
      </AppShell>
      <Toaster />
    </>
  );
}

export default App;
