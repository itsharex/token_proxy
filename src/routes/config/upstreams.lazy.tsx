import { createLazyFileRoute } from "@tanstack/react-router";

import { ConfigRoutePage } from "@/features/config/ConfigRoutePage";

export const Route = createLazyFileRoute("/config/upstreams")({
  component: ConfigUpstreamsRoute,
});

function ConfigUpstreamsRoute() {
  return <ConfigRoutePage sectionId="upstreams" />;
}
