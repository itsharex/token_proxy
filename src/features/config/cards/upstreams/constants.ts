import type { ColumnVisibility, UpstreamColumnDefinition } from "@/features/config/cards/upstreams/types";

export const UPSTREAM_COLUMNS: readonly UpstreamColumnDefinition[] = [
  {
    id: "id",
    label: "Id",
    defaultVisible: true,
    headerClassName: "w-[14rem]",
    cellClassName: "w-[14rem] max-w-[14rem]",
  },
  {
    id: "provider",
    label: "Provider",
    defaultVisible: true,
    headerClassName: "w-[12rem]",
    cellClassName: "w-[12rem] max-w-[12rem]",
  },
  { id: "baseUrl", label: "Base URL", defaultVisible: false, cellClassName: "min-w-[18rem]" },
  { id: "apiKey", label: "API Key", defaultVisible: false, cellClassName: "min-w-[18rem]" },
  {
    id: "priority",
    label: "Priority",
    defaultVisible: true,
    headerClassName: "w-[8rem]",
    cellClassName: "w-[8rem]",
  },
  {
    id: "index",
    label: "Index",
    defaultVisible: false,
    headerClassName: "w-[8rem]",
    cellClassName: "w-[8rem]",
  },
  {
    id: "status",
    label: "Status",
    defaultVisible: true,
    headerClassName: "w-[8rem]",
    cellClassName: "w-[8rem]",
  },
];

export function createDefaultColumnVisibility() {
  const visibility: ColumnVisibility = {
    id: true,
    provider: true,
    baseUrl: false,
    apiKey: false,
    priority: true,
    index: false,
    status: true,
  };
  for (const column of UPSTREAM_COLUMNS) {
    visibility[column.id] = column.defaultVisible;
  }
  return visibility;
}

const DEFAULT_PROVIDER_OPTIONS = ["openai", "openai-response", "claude"] as const;

export function mergeProviderOptions(values: readonly string[]) {
  const seen = new Set<string>();
  const merged: string[] = [];
  for (const option of DEFAULT_PROVIDER_OPTIONS) {
    if (!seen.has(option)) {
      seen.add(option);
      merged.push(option);
    }
  }
  for (const option of values) {
    if (!seen.has(option)) {
      seen.add(option);
      merged.push(option);
    }
  }
  return merged;
}

export function toMaskedApiKey(value: string) {
  return value.trim() ? "••••••••" : "";
}

export function toStatusLabel(enabled: boolean) {
  return enabled ? "Enabled" : "Disabled";
}

export function getUpstreamLabel(index: number) {
  return `Upstream ${index + 1}`;
}
