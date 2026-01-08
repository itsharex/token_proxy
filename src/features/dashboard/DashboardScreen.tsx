import { useCallback, useEffect, useMemo, useState } from "react";

import { AlertCircle, RefreshCcw } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { readDashboardSnapshot } from "@/features/dashboard/api";
import { RecentRequestsTable } from "@/features/dashboard/RecentRequestsTable";
import { formatInteger } from "@/features/dashboard/format";
import type { DashboardRange, DashboardSnapshot } from "@/features/dashboard/types";
import { parseError } from "@/lib/error";
import { cn } from "@/lib/utils";

const PERCENT_FORMAT = new Intl.NumberFormat(undefined, { style: "percent", maximumFractionDigits: 1 });

const RECENT_PAGE_SIZE = 50;

const RANGE_PRESETS = [
  { value: "1h", label: "最近 1 小时" },
  { value: "24h", label: "最近 24 小时" },
  { value: "7d", label: "最近 7 天" },
  { value: "30d", label: "最近 30 天" },
  { value: "all", label: "全部" },
] as const;

type RangePreset = (typeof RANGE_PRESETS)[number]["value"];

const RANGE_VALUES: ReadonlySet<string> = new Set(RANGE_PRESETS.map((preset) => preset.value));

function toRangePreset(value: string): RangePreset | null {
  return RANGE_VALUES.has(value) ? (value as RangePreset) : null;
}

function resolveRange(preset: RangePreset): DashboardRange {
  if (preset === "all") {
    return { fromTsMs: null, toTsMs: null };
  }
  const now = Date.now();
  const ms =
    preset === "1h"
      ? 60 * 60 * 1000
      : preset === "24h"
        ? 24 * 60 * 60 * 1000
        : preset === "7d"
          ? 7 * 24 * 60 * 60 * 1000
          : 30 * 24 * 60 * 60 * 1000;
  return { fromTsMs: now - ms, toTsMs: now };
}

type StatCardProps = {
  title: string;
  value: string;
  hint: string;
};

function StatCard({ title, value, hint }: StatCardProps) {
  return (
    <Card data-slot="dashboard-stat-card" className="shadow-sm">
      <CardHeader className="gap-1 pb-4">
        <CardTitle className="text-sm font-semibold">{title}</CardTitle>
        <CardDescription className="text-xs">{hint}</CardDescription>
      </CardHeader>
      <CardContent className="pt-0">
        <p className="title-font text-2xl font-semibold tracking-tight text-foreground">{value}</p>
      </CardContent>
    </Card>
  );
}

export function DashboardScreen() {
  const [preset, setPreset] = useState<RangePreset>("24h");
  const [page, setPage] = useState(1);
  const [snapshot, setSnapshot] = useState<DashboardSnapshot | null>(null);
  const [status, setStatus] = useState<"idle" | "loading" | "error">("idle");
  const [statusMessage, setStatusMessage] = useState("");

  const loadSnapshot = useCallback(async () => {
    setStatus("loading");
    setStatusMessage("");
    try {
      const range = resolveRange(preset);
      const offset = (page - 1) * RECENT_PAGE_SIZE;
      const data = await readDashboardSnapshot(range, offset);
      setSnapshot(data);
      setStatus("idle");
    } catch (error) {
      setStatus("error");
      setStatusMessage(parseError(error));
    }
  }, [page, preset]);

  useEffect(() => {
    void loadSnapshot();
  }, [loadSnapshot]);

  const derived = useMemo(() => {
    if (!snapshot) {
      return null;
    }
    const totalRequests = snapshot.summary.totalRequests;
    const errorRate = totalRequests > 0 ? snapshot.summary.errorRequests / totalRequests : 0;
    const successRate = totalRequests > 0 ? snapshot.summary.successRequests / totalRequests : 0;
    const providers = snapshot.providers
      .slice()
      .sort((a, b) => b.totalTokens - a.totalTokens)
      .slice(0, 10);
    return { errorRate, successRate, providers };
  }, [snapshot]);

  const pagination = useMemo(() => {
    const totalRequests = snapshot?.summary.totalRequests ?? 0;
    const totalPages = Math.max(1, Math.ceil(totalRequests / RECENT_PAGE_SIZE));
    const safePage = Math.min(Math.max(1, page), totalPages);

    return {
      totalRequests,
      totalPages,
      page: safePage,
      canPrev: safePage > 1,
      canNext: safePage < totalPages,
    };
  }, [page, snapshot?.summary.totalRequests]);

  useEffect(() => {
    if (page !== pagination.page) {
      setPage(pagination.page);
    }
  }, [page, pagination.page]);

  return (
    <section data-slot="dashboard-screen" className="h-full min-h-0">
      <ScrollArea data-slot="dashboard-scroll" className="h-full min-h-0">
        <div className="space-y-6 p-6">
          <div className="flex flex-wrap items-end justify-between gap-3">
            <div className="min-w-0">
              <h1 className="title-font truncate text-2xl font-semibold text-foreground">Dashboard</h1>
              <p className="truncate text-sm text-muted-foreground">请求与 token 使用概览</p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              {snapshot?.truncated ? (
                <Badge variant="secondary" className="border border-border/60">
                  Truncated
                </Badge>
              ) : null}
              <Select
                value={preset}
                onValueChange={(value) => {
                  const next = toRangePreset(value);
                  if (next) {
                    setPreset(next);
                    setPage(1);
                  }
                }}
              >
                <SelectTrigger className="h-10 w-[160px]">
                  <SelectValue placeholder="选择范围" />
                </SelectTrigger>
                <SelectContent>
                  {RANGE_PRESETS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button type="button" onClick={loadSnapshot} disabled={status === "loading"}>
                <RefreshCcw className={cn("mr-2 size-4", status === "loading" && "animate-spin")} />
                刷新
              </Button>
            </div>
          </div>

          {status === "error" && statusMessage ? (
            <Alert variant="destructive">
              <AlertCircle className="size-4" aria-hidden="true" />
              <div>
                <AlertTitle>加载失败</AlertTitle>
                <AlertDescription>{statusMessage}</AlertDescription>
              </div>
            </Alert>
          ) : null}

          <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <StatCard
              title="请求数"
              value={formatInteger(snapshot?.summary.totalRequests ?? 0)}
              hint={derived ? `成功率 ${PERCENT_FORMAT.format(derived.successRate)}` : "—"}
            />
            <StatCard
              title="错误数"
              value={formatInteger(snapshot?.summary.errorRequests ?? 0)}
              hint={derived ? `错误率 ${PERCENT_FORMAT.format(derived.errorRate)}` : "—"}
            />
            <StatCard
              title="总 Tokens"
              value={formatInteger(snapshot?.summary.totalTokens ?? 0)}
              hint={`输入 ${formatInteger(snapshot?.summary.inputTokens ?? 0)}${
                snapshot?.summary.cachedTokens
                  ? `（cached ${formatInteger(snapshot?.summary.cachedTokens ?? 0)}）`
                  : ""
              } · 输出 ${formatInteger(snapshot?.summary.outputTokens ?? 0)}`}
            />
            <StatCard
              title="延迟 (ms)"
              value={formatInteger(snapshot?.summary.avgLatencyMs ?? 0)}
              hint="按请求均值"
            />
          </div>

          <div className="grid gap-4 xl:grid-cols-2">
            <Card data-slot="dashboard-providers">
              <CardHeader>
                <CardTitle className="text-base">Providers</CardTitle>
                <CardDescription>按 Tokens 排序（Top 10）</CardDescription>
              </CardHeader>
              <CardContent className="pt-0">
                {derived?.providers.length ? (
                  <div className="space-y-2">
                    {derived.providers.map((stat) => (
                      <div
                        key={stat.provider}
                        className="flex items-center justify-between gap-3 rounded-lg border border-border/60 bg-background/70 px-3 py-2"
                      >
                        <div className="min-w-0">
                          <p className="truncate text-sm font-medium text-foreground">{stat.provider}</p>
                          <p className="truncate text-xs text-muted-foreground">
                            {formatInteger(stat.requests)} requests
                          </p>
                        </div>
                        <div className="flex items-center gap-2">
                          <Badge variant="outline" className="border-border/60">
                            {formatInteger(stat.totalTokens)}
                          </Badge>
                          {stat.cachedTokens ? (
                            <Badge variant="secondary" className="border border-border/60">
                              {formatInteger(stat.cachedTokens)} cached
                            </Badge>
                          ) : null}
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">暂无数据</p>
                )}
              </CardContent>
            </Card>

            <Card data-slot="dashboard-recent">
              <CardHeader>
                <CardTitle className="text-base">Recent Requests</CardTitle>
                <CardDescription>
                  按时间倒序 · 每页 {RECENT_PAGE_SIZE} 条 · 共 {formatInteger(pagination.totalRequests)} 条
                </CardDescription>
                <CardAction className="flex flex-col items-end gap-2">
                  <div className="flex items-center gap-2">
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      disabled={!pagination.canPrev || status === "loading"}
                      onClick={() => setPage((current) => Math.max(1, current - 1))}
                    >
                      上一页
                    </Button>
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      disabled={!pagination.canNext || status === "loading"}
                      onClick={() => setPage((current) => Math.min(pagination.totalPages, current + 1))}
                    >
                      下一页
                    </Button>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    第 {pagination.page} / {pagination.totalPages} 页
                  </p>
                </CardAction>
              </CardHeader>
	              <CardContent className="pt-0">
	                {snapshot?.recent.length ? (
	                  <RecentRequestsTable items={snapshot.recent} scrollKey={`${preset}-${pagination.page}`} />
	                ) : (
	                  <p className="text-sm text-muted-foreground">暂无数据</p>
	                )}
	              </CardContent>
            </Card>
          </div>
        </div>
      </ScrollArea>
    </section>
  );
}
