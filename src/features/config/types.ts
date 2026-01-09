import { m } from "@/paraglide/messages.js";

export const UPSTREAM_STRATEGIES = [
  { value: "priority_round_robin", label: () => m.upstream_strategy_priority_round_robin() },
  { value: "priority_fill_first", label: () => m.upstream_strategy_priority_fill_first() },
] as const;

export type UpstreamStrategy = (typeof UPSTREAM_STRATEGIES)[number]["value"];

export type UpstreamConfig = {
  id: string;
  provider: string;
  base_url: string;
  api_key: string | null;
  priority: number | null;
  index: number | null;
  enabled: boolean;
  model_mappings: Record<string, string>;
};

export type ProxyConfigFile = {
  host: string;
  port: number;
  local_api_key: string | null;
  log_path: string;
  enable_api_format_conversion: boolean;
  upstream_strategy: UpstreamStrategy;
  upstreams: UpstreamConfig[];
};

export type ConfigResponse = {
  path: string;
  config: ProxyConfigFile;
};

export type ProxyServiceState = "running" | "stopped";

export type ProxyServiceStatus = {
  state: ProxyServiceState;
  addr: string | null;
  last_error: string | null;
};

export type UpstreamForm = {
  id: string;
  provider: string;
  baseUrl: string;
  apiKey: string;
  priority: string;
  index: string;
  enabled: boolean;
  modelMappings: ModelMappingForm[];
};

export type ModelMappingForm = {
  id: string;
  pattern: string;
  target: string;
};

export type ConfigForm = {
  host: string;
  port: string;
  localApiKey: string;
  logPath: string;
  enableApiFormatConversion: boolean;
  upstreamStrategy: UpstreamStrategy;
  upstreams: UpstreamForm[];
};
