export type DashboardRange = {
  fromTsMs: number | null;
  toTsMs: number | null;
};

export type DashboardSummary = {
  totalRequests: number;
  successRequests: number;
  errorRequests: number;
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  avgLatencyMs: number;
};

export type DashboardProviderStat = {
  provider: string;
  requests: number;
  totalTokens: number;
};

export type DashboardRequestItem = {
  tsMs: number;
  path: string;
  provider: string;
  upstreamId: string;
  model: string | null;
  stream: boolean;
  status: number;
  totalTokens: number | null;
  latencyMs: number;
  upstreamRequestId: string | null;
};

export type DashboardSnapshot = {
  summary: DashboardSummary;
  providers: DashboardProviderStat[];
  recent: DashboardRequestItem[];
  truncated: boolean;
};

