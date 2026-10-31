import { useState } from "react";
import "./App.css";
import { Park } from "./components/park/Park";
import { AppShell } from "./components/shell/AppShell";
import { Toaster } from "./components/ui/sonner";
import type { ToolState } from "./types/park/tool";
import { Editor } from "./components/editor/Editor";
import { BrowserRouter, Route, Routes } from "react-router-dom";

function App() {
  const [tool, setTool] = useState<ToolState>({ mode: null });

  return (
    <>
      <BrowserRouter>
        <AppShell>
          <Routes>
            <Route path="/" element={<Park tool={tool} onToolChange={setTool} />}/>
            <Route path="/park" element={<Park tool={tool} onToolChange={setTool} />}/>
            <Route path="/editor" element={<Editor tool={tool} onToolChange={setTool}/>}/>
          </Routes>
        </AppShell>
        <Toaster />
      </BrowserRouter>
    </>
  );
}

export default App;
