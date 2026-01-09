import { Outlet } from "@tanstack/react-router";

import "./App.css";

function App() {
  return (
    <main className="app-shell">
      <div data-slot="app-shell" className="relative z-10 h-full min-h-0">
        <Outlet />
      </div>
    </main>
  );
}

export default App;
