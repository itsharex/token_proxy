import { Loader2, Play, RefreshCw, RotateCcw, Square } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import type { ProxyServiceStatus } from "@/features/config/types";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages.js";

type ProxyServiceRequestState = "idle" | "working" | "error";

type ProxyServiceCardProps = {
  status: ProxyServiceStatus | null;
  requestState: ProxyServiceRequestState;
  message: string;
  isDirty: boolean;
  onRefresh: () => void;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  onReload: () => void;
};

function resolveBadge(status: ProxyServiceStatus | null, message: string) {
  const hasError = Boolean(message || status?.last_error);
  if (hasError) {
    return { label: m.proxy_service_badge_error(), variant: "destructive" as const };
  }
  if (!status) {
    return { label: m.proxy_service_badge_unknown(), variant: "outline" as const };
  }
  if (status.state === "running") {
    return { label: m.proxy_service_badge_running(), variant: "default" as const };
  }
  return { label: m.proxy_service_badge_stopped(), variant: "secondary" as const };
}

export function ProxyServiceCard({
  status,
  requestState,
  message,
  isDirty,
  onRefresh,
  onStart,
  onStop,
  onRestart,
  onReload,
}: ProxyServiceCardProps) {
  const isWorking = requestState === "working";
  const isRunning = status?.state === "running";
  const errorMessage = message || status?.last_error || "";
  const badge = resolveBadge(status, message);
  const addr = status?.addr || "--";

  return (
    <Card data-slot="proxy-service-card">
      <CardHeader>
        <CardTitle>{m.proxy_service_title()}</CardTitle>
        <CardDescription>{m.proxy_service_desc()}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3 text-sm">
          <div className="flex items-center gap-2">
            <span className="text-muted-foreground">{m.proxy_service_state_label()}</span>
            <Badge variant={badge.variant}>{badge.label}</Badge>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-muted-foreground">{m.proxy_service_addr_label()}</span>
            <span className="font-mono text-xs text-foreground/80">{addr}</span>
          </div>
        </div>
        <Separator />
        <div className="grid gap-2 text-xs text-muted-foreground">
          <p>{m.proxy_service_help_1()}</p>
          <p>{m.proxy_service_help_2()}</p>
        </div>
        {errorMessage ? (
          <div className="rounded-md border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive">
            {errorMessage}
          </div>
        ) : null}
        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" variant="outline" size="icon" onClick={onRefresh} disabled={isWorking}>
            <RefreshCw className={cn("size-4", isWorking && "animate-spin")} aria-hidden="true" />
          </Button>
          <Button type="button" onClick={onStart} disabled={isWorking || isRunning}>
            {isWorking ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Play aria-hidden="true" />}
            {m.proxy_service_start()}
          </Button>
          <Button type="button" variant="outline" onClick={onStop} disabled={isWorking || !isRunning}>
            <Square aria-hidden="true" />
            {m.proxy_service_stop()}
          </Button>
          <Button type="button" variant="outline" onClick={onRestart} disabled={isWorking || !isRunning || isDirty}>
            <RotateCcw aria-hidden="true" />
            {m.proxy_service_restart()}
          </Button>
          <Button type="button" variant="outline" onClick={onReload} disabled={isWorking || isDirty}>
            {m.proxy_service_reload_config()}
          </Button>
        </div>
        {isDirty ? (
          <div className="rounded-md border border-border/60 bg-background/60 p-3 text-xs text-muted-foreground">
            {m.proxy_service_unsaved_notice()}
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}
