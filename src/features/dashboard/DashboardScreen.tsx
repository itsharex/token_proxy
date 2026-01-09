import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";

import { AlertCircle, RefreshCcw } from "lucide-react";

import { LanguageSwitcher } from "@/components/LanguageSwitcher";
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
import { m } from "@/paraglide/messages.js";

type DashboardScreenVariant = "standalone" | "embedded";

type DashboardScreenProps = {
  variant?: DashboardScreenVariant;
  headerLeading?: ReactNode;
};

const PERCENT_FORMAT = new Intl.NumberFormat(undefined, { style: "percent", maximumFractionDigits: 1 });

const RECENT_PAGE_SIZE = 50;

const RANGE_PRESETS = [
  { value: "1h", label: () => m.dashboard_range_1h() },
  { value: "24h", label: () => m.dashboard_range_24h() },
  { value: "7d", label: () => m.dashboard_range_7d() },
  { value: "30d", label: () => m.dashboard_range_30d() },
  { value: "all", label: () => m.dashboard_range_all() },
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

export function DashboardScreen({ variant = "standalone", headerLeading }: DashboardScreenProps) {
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

  const content = (
    <div className="space-y-6 py-6 pr-6">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="min-w-0">
          <h1 className="title-font truncate text-2xl font-semibold text-foreground">
            {m.dashboard_title()}
          </h1>
          <p className="truncate text-sm text-muted-foreground">{m.dashboard_subtitle()}</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {headerLeading}
          <LanguageSwitcher triggerClassName="h-10 w-[140px]" />
          {snapshot?.truncated ? (
            <Badge variant="secondary" className="border border-border/60">
              {m.dashboard_truncated()}
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
              <SelectValue placeholder={m.dashboard_range_placeholder()} />
            </SelectTrigger>
            <SelectContent>
              {RANGE_PRESETS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label()}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button type="button" onClick={loadSnapshot} disabled={status === "loading"}>
            <RefreshCcw className={cn("mr-2 size-4", status === "loading" && "animate-spin")} />
            {m.common_refresh()}
          </Button>
        </div>
      </div>

      {status === "error" && statusMessage ? (
        <Alert variant="destructive">
          <AlertCircle className="size-4" aria-hidden="true" />
          <div>
            <AlertTitle>{m.dashboard_load_failed()}</AlertTitle>
            <AlertDescription>{statusMessage}</AlertDescription>
          </div>
        </Alert>
      ) : null}

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <StatCard
          title={m.dashboard_stat_requests()}
          value={formatInteger(snapshot?.summary.totalRequests ?? 0)}
          hint={
            derived
              ? m.dashboard_hint_success_rate({ rate: PERCENT_FORMAT.format(derived.successRate) })
              : "—"
          }
        />
        <StatCard
          title={m.dashboard_stat_errors()}
          value={formatInteger(snapshot?.summary.errorRequests ?? 0)}
          hint={
            derived ? m.dashboard_hint_error_rate({ rate: PERCENT_FORMAT.format(derived.errorRate) }) : "—"
          }
        />
        <StatCard
          title={m.dashboard_stat_total_tokens()}
          value={formatInteger(snapshot?.summary.totalTokens ?? 0)}
          hint={(() => {
            const input = formatInteger(snapshot?.summary.inputTokens ?? 0);
            const output = formatInteger(snapshot?.summary.outputTokens ?? 0);
            const cached = snapshot?.summary.cachedTokens
              ? formatInteger(snapshot?.summary.cachedTokens ?? 0)
              : null;
            return cached
              ? m.dashboard_tokens_hint_with_cache({ input, cached, output })
              : m.dashboard_tokens_hint_no_cache({ input, output });
          })()}
        />
        <StatCard
          title={m.dashboard_stat_latency_ms()}
          value={formatInteger(snapshot?.summary.avgLatencyMs ?? 0)}
          hint={m.dashboard_latency_hint()}
        />
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        <Card data-slot="dashboard-providers">
          <CardHeader>
            <CardTitle className="text-base">{m.dashboard_providers_title()}</CardTitle>
            <CardDescription>{m.dashboard_providers_desc()}</CardDescription>
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
                        {m.dashboard_provider_requests({ count: formatInteger(stat.requests) })}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="border-border/60">
                        {formatInteger(stat.totalTokens)}
                      </Badge>
                      {stat.cachedTokens ? (
                        <Badge variant="secondary" className="border border-border/60">
                          {m.dashboard_cached({ count: formatInteger(stat.cachedTokens) })}
                        </Badge>
                      ) : null}
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">{m.dashboard_no_data()}</p>
            )}
          </CardContent>
        </Card>

        <Card data-slot="dashboard-recent">
          <CardHeader>
            <CardTitle className="text-base">{m.dashboard_recent_title()}</CardTitle>
            <CardDescription>
              {m.dashboard_recent_desc({
                pageSize: String(RECENT_PAGE_SIZE),
                total: formatInteger(pagination.totalRequests),
              })}
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
                  {m.dashboard_prev_page()}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  disabled={!pagination.canNext || status === "loading"}
                  onClick={() => setPage((current) => Math.min(pagination.totalPages, current + 1))}
                >
                  {m.dashboard_next_page()}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                {m.dashboard_page_indicator({
                  page: String(pagination.page),
                  totalPages: String(pagination.totalPages),
                })}
              </p>
            </CardAction>
          </CardHeader>
          <CardContent className="pt-0">
            {snapshot?.recent.length ? (
              <RecentRequestsTable items={snapshot.recent} scrollKey={`${preset}-${pagination.page}`} />
            ) : (
              <p className="text-sm text-muted-foreground">{m.dashboard_no_data()}</p>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );

  if (variant === "embedded") {
    return <section data-slot="dashboard-screen">{content}</section>;
  }

  return (
    <section data-slot="dashboard-screen" className="h-full min-h-0">
      <ScrollArea data-slot="dashboard-scroll" className="h-full min-h-0">
        {content}
      </ScrollArea>
    </section>
  );
}
