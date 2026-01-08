import { useCallback, useEffect, useMemo, useState } from "react";

import { AlertCircle, RefreshCcw } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { readDashboardSnapshot } from "@/features/dashboard/api";
import type { DashboardRange, DashboardSnapshot } from "@/features/dashboard/types";
import { parseError } from "@/lib/error";
import { cn } from "@/lib/utils";

const NUMBER_FORMAT = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 });
const PERCENT_FORMAT = new Intl.NumberFormat(undefined, { style: "percent", maximumFractionDigits: 1 });

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

type BadgeVariant = "default" | "secondary" | "destructive" | "outline";

function statusToVariant(status: number): BadgeVariant {
  if (status >= 200 && status < 300) {
    return "default";
  }
  if (status >= 400) {
    return "destructive";
  }
  if (status >= 300) {
    return "secondary";
  }
  return "outline";
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
  const [snapshot, setSnapshot] = useState<DashboardSnapshot | null>(null);
  const [status, setStatus] = useState<"idle" | "loading" | "error">("idle");
  const [statusMessage, setStatusMessage] = useState("");

  const loadSnapshot = useCallback(async () => {
    setStatus("loading");
    setStatusMessage("");
    try {
      const range = resolveRange(preset);
      const data = await readDashboardSnapshot(range, 100);
      setSnapshot(data);
      setStatus("idle");
    } catch (error) {
      setStatus("error");
      setStatusMessage(parseError(error));
    }
  }, [preset]);

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
              value={NUMBER_FORMAT.format(snapshot?.summary.totalRequests ?? 0)}
              hint={derived ? `成功率 ${PERCENT_FORMAT.format(derived.successRate)}` : "—"}
            />
            <StatCard
              title="错误数"
              value={NUMBER_FORMAT.format(snapshot?.summary.errorRequests ?? 0)}
              hint={derived ? `错误率 ${PERCENT_FORMAT.format(derived.errorRate)}` : "—"}
            />
            <StatCard
              title="总 Tokens"
              value={NUMBER_FORMAT.format(snapshot?.summary.totalTokens ?? 0)}
              hint={`输入 ${NUMBER_FORMAT.format(snapshot?.summary.inputTokens ?? 0)}${
                snapshot?.summary.cachedTokens
                  ? `（cached ${NUMBER_FORMAT.format(snapshot?.summary.cachedTokens ?? 0)}）`
                  : ""
              } · 输出 ${NUMBER_FORMAT.format(snapshot?.summary.outputTokens ?? 0)}`}
            />
            <StatCard
              title="平均延迟"
              value={`${NUMBER_FORMAT.format(snapshot?.summary.avgLatencyMs ?? 0)} ms`}
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
                            {NUMBER_FORMAT.format(stat.requests)} requests
                          </p>
                        </div>
                        <div className="flex items-center gap-2">
                          <Badge variant="outline" className="border-border/60">
                            {NUMBER_FORMAT.format(stat.totalTokens)} tokens
                          </Badge>
                          {stat.cachedTokens ? (
                            <Badge variant="secondary" className="border border-border/60">
                              {NUMBER_FORMAT.format(stat.cachedTokens)} cached
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
                <CardDescription>最近 100 条（按时间倒序）</CardDescription>
              </CardHeader>
              <CardContent className="pt-0">
                {snapshot?.recent.length ? (
                  <div className="overflow-hidden rounded-lg border border-border/60">
                    <table className="w-full text-sm">
                      <thead className="bg-muted/50 text-xs text-muted-foreground">
                        <tr>
                          <th className="px-3 py-2 text-left font-medium">时间</th>
                          <th className="px-3 py-2 text-left font-medium">路径</th>
                          <th className="px-3 py-2 text-left font-medium">Provider</th>
                          <th className="px-3 py-2 text-left font-medium">状态</th>
                          <th className="px-3 py-2 text-right font-medium">Tokens</th>
                          <th className="px-3 py-2 text-right font-medium">延迟</th>
                        </tr>
                      </thead>
                      <tbody>
                        {snapshot.recent.map((item) => (
                          <tr
                            key={`${item.tsMs}-${item.upstreamRequestId ?? item.upstreamId}`}
                            className="border-t border-border/60 bg-background/70 hover:bg-accent/30"
                          >
                            <td className="whitespace-nowrap px-3 py-2 text-xs text-muted-foreground">
                              {new Date(item.tsMs).toLocaleString()}
                            </td>
                            <td className="max-w-[220px] truncate px-3 py-2">
                              <span className="font-medium text-foreground">{item.path}</span>
                            </td>
                            <td className="max-w-[160px] truncate px-3 py-2 text-xs text-muted-foreground">
                              {item.provider}
                              <span className="text-muted-foreground/70"> · {item.upstreamId}</span>
                            </td>
                            <td className="px-3 py-2">
                              <Badge variant={statusToVariant(item.status)}>{item.status}</Badge>
                            </td>
                            <td className="px-3 py-2 text-right font-medium text-foreground">
                              <div className="flex flex-col items-end gap-0.5">
                                <span>
                                  {item.totalTokens === null ? "—" : NUMBER_FORMAT.format(item.totalTokens)}
                                </span>
                                {item.cachedTokens ? (
                                  <span className="text-xs font-normal text-muted-foreground">
                                    cached {NUMBER_FORMAT.format(item.cachedTokens)}
                                  </span>
                                ) : null}
                              </div>
                            </td>
                            <td className="px-3 py-2 text-right text-xs text-muted-foreground">
                              {NUMBER_FORMAT.format(item.latencyMs)} ms
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
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
