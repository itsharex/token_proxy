import { createLazyFileRoute } from "@tanstack/react-router";

import { ConfigRoutePage } from "@/features/config/ConfigRoutePage";

export const Route = createLazyFileRoute("/config/dashboard")({
  component: ConfigDashboardRoute,
});

function ConfigDashboardRoute() {
  return <ConfigRoutePage sectionId="dashboard" />;
}
