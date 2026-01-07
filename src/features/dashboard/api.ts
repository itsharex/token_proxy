import { invoke } from "@tauri-apps/api/core";

import type { DashboardRange, DashboardSnapshot } from "@/features/dashboard/types";

export async function readDashboardSnapshot(range: DashboardRange, limit?: number) {
  return await invoke<DashboardSnapshot>("read_dashboard_snapshot", {
    range,
    limit,
  });
}

