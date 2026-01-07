import { useMemo, useState } from "react";

import { LayoutDashboard, Settings2 } from "lucide-react";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ConfigScreen } from "@/features/config/ConfigScreen";
import { DashboardScreen } from "@/features/dashboard/DashboardScreen";
import { cn } from "@/lib/utils";

import "./App.css";

type AppPage = "dashboard" | "config";

function App() {
  const [page, setPage] = useState<AppPage>("dashboard");

  const title = useMemo(() => (page === "dashboard" ? "Dashboard" : "Config"), [page]);

  return (
    <main className="app-shell">
      <Tabs
        data-slot="app-shell"
        value={page}
        onValueChange={(value) => setPage(value as AppPage)}
        className="relative z-10 flex h-full min-h-0 flex-col"
      >
        <header
          data-slot="app-header"
          className="flex items-center justify-between gap-3 border-b border-border/60 bg-background/70 px-6 py-4 backdrop-blur"
        >
          <div className="min-w-0">
            <p className="title-font truncate text-base font-semibold text-foreground">Token Proxy</p>
            <p className="truncate text-xs text-muted-foreground">{title}</p>
          </div>
          <TabsList className="h-10">
            <TabsTrigger value="dashboard" className="gap-2">
              <LayoutDashboard className="size-4" aria-hidden="true" />
              Dashboard
            </TabsTrigger>
            <TabsTrigger value="config" className="gap-2">
              <Settings2 className="size-4" aria-hidden="true" />
              Config
            </TabsTrigger>
          </TabsList>
        </header>

        <TabsContent value="dashboard" className={cn("mt-0 min-h-0 flex-1")}>
          <DashboardScreen />
        </TabsContent>
        <TabsContent value="config" className={cn("mt-0 min-h-0 flex-1")}>
          <ConfigScreen />
        </TabsContent>
      </Tabs>
    </main>
  );
}

export default App;
