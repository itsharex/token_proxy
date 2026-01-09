import { createFileRoute, redirect } from "@tanstack/react-router";

import { DEFAULT_CONFIG_SECTION, getSectionRoute } from "@/features/config/sections";

export const Route = createFileRoute("/config/")({
  beforeLoad: () => {
    // 进入 /config 时统一跳转到默认分区。
    throw redirect({
      to: getSectionRoute(DEFAULT_CONFIG_SECTION),
      replace: true,
    });
  },
});
