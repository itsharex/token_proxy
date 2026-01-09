import { createLazyFileRoute } from "@tanstack/react-router";

import { ConfigRoutePage } from "@/features/config/ConfigRoutePage";

export const Route = createLazyFileRoute("/config/file")({
  component: ConfigFileRoute,
});

function ConfigFileRoute() {
  return <ConfigRoutePage sectionId="file" />;
}
