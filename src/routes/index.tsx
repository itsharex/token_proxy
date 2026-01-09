import { createFileRoute, redirect } from "@tanstack/react-router";

import { DEFAULT_CONFIG_SECTION, getSectionRoute } from "@/features/config/sections";

export const Route = createFileRoute("/")({
  beforeLoad: () => {
    // Tauri 启动默认走 /，桌面端无地址栏，直接跳到带菜单的默认分区。
    throw redirect({
      to: getSectionRoute(DEFAULT_CONFIG_SECTION),
      replace: true,
    });
  },
});
