export const UPSTREAM_STRATEGIES = [
  { value: "priority_round_robin", label: "Priority Round Robin" },
  { value: "priority_fill_first", label: "Priority Fill First" },
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
};

export type ProxyConfigFile = {
  host: string;
  port: number;
  local_api_key: string | null;
  log_path: string;
  upstream_strategy: UpstreamStrategy;
  upstreams: UpstreamConfig[];
};

export type ConfigResponse = {
  path: string;
  config: ProxyConfigFile;
};

export type UpstreamForm = {
  id: string;
  provider: string;
  baseUrl: string;
  apiKey: string;
  priority: string;
  index: string;
  enabled: boolean;
};

export type ConfigForm = {
  host: string;
  port: string;
  localApiKey: string;
  logPath: string;
  upstreamStrategy: UpstreamStrategy;
  upstreams: UpstreamForm[];
};
