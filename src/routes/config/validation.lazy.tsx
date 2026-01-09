import { createLazyFileRoute } from "@tanstack/react-router";

import { ConfigRoutePage } from "@/features/config/ConfigRoutePage";

export const Route = createLazyFileRoute("/config/validation")({
  component: ConfigValidationRoute,
});

function ConfigValidationRoute() {
  return <ConfigRoutePage sectionId="validation" />;
}
