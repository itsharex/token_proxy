import { Outlet } from "@tanstack/react-router";
import { useI18n } from "@/lib/i18n";

import "./App.css";

function App() {
  // 订阅语言状态：触发全局重渲染，让 Paraglide 文案在切换语言后即时更新。
  useI18n();

  return (
    <main className="app-shell">
      <div data-slot="app-shell" className="relative z-10 h-full min-h-0">
        <Outlet />
      </div>
    </main>
  );
}

export default App;
