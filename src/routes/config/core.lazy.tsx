import { createLazyFileRoute } from "@tanstack/react-router";

import { ConfigRoutePage } from "@/features/config/ConfigRoutePage";

export const Route = createLazyFileRoute("/config/core")({
  component: ConfigCoreRoute,
});

function ConfigCoreRoute() {
  return <ConfigRoutePage sectionId="core" />;
}
