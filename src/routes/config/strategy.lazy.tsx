import { createLazyFileRoute } from "@tanstack/react-router";

import { ConfigRoutePage } from "@/features/config/ConfigRoutePage";

export const Route = createLazyFileRoute("/config/strategy")({
  component: ConfigStrategyRoute,
});

function ConfigStrategyRoute() {
  return <ConfigRoutePage sectionId="strategy" />;
}
