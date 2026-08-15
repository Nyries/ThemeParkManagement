import "./App.css";
import { Park } from "./components/Park";
import { AppShell } from "./layout/AppShell";

function App() {
  return (
    <AppShell>
      <Park />
    </AppShell>
  );
}

export default App;
