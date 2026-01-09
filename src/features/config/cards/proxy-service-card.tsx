import { Loader2, Play, RefreshCw, RotateCcw, Square } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import type { ProxyServiceRequestState, ProxyServiceStatus } from "@/features/config/types";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages.js";

export type ProxyServiceViewProps = {
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

type ProxyServiceContentProps = ProxyServiceViewProps & { className?: string };

type ProxyServiceStatusRowProps = {
  badge: ReturnType<typeof resolveBadge>;
  addr: string;
};

function ProxyServiceStatusRow({ badge, addr }: ProxyServiceStatusRowProps) {
  return (
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
  );
}

function ProxyServiceHelp() {
  return (
    <div className="grid gap-2 text-xs text-muted-foreground">
      <p>{m.proxy_service_help_1()}</p>
      <p>{m.proxy_service_help_2()}</p>
    </div>
  );
}

type ProxyServiceErrorProps = {
  message: string;
};

function ProxyServiceError({ message }: ProxyServiceErrorProps) {
  if (!message) {
    return null;
  }
  return (
    <div className="rounded-md border border-destructive/30 bg-destructive/5 p-3 text-xs text-destructive">
      {message}
    </div>
  );
}

type ProxyServiceActionsProps = {
  isWorking: boolean;
  isRunning: boolean;
  isDirty: boolean;
  onRefresh: () => void;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  onReload: () => void;
};

function ProxyServiceActions({
  isWorking,
  isRunning,
  isDirty,
  onRefresh,
  onStart,
  onStop,
  onRestart,
  onReload,
}: ProxyServiceActionsProps) {
  return (
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
  );
}

type ProxyServiceDirtyNoticeProps = {
  isDirty: boolean;
};

function ProxyServiceDirtyNotice({ isDirty }: ProxyServiceDirtyNoticeProps) {
  if (!isDirty) {
    return null;
  }
  return (
    <div className="rounded-md border border-border/60 bg-background/60 p-3 text-xs text-muted-foreground">
      {m.proxy_service_unsaved_notice()}
    </div>
  );
}

function ProxyServiceContent({
  status,
  requestState,
  message,
  isDirty,
  onRefresh,
  onStart,
  onStop,
  onRestart,
  onReload,
  className,
}: ProxyServiceContentProps) {
  const isWorking = requestState === "working";
  const isRunning = status?.state === "running";
  const errorMessage = message || status?.last_error || "";
  const badge = resolveBadge(status, message);
  const addr = status?.addr || "--";

  return (
    <div className={cn("space-y-4", className)}>
      <ProxyServiceStatusRow badge={badge} addr={addr} />
      <Separator />
      <ProxyServiceHelp />
      <ProxyServiceError message={errorMessage} />
      <ProxyServiceActions
        isWorking={isWorking}
        isRunning={isRunning}
        isDirty={isDirty}
        onRefresh={onRefresh}
        onStart={onStart}
        onStop={onStop}
        onRestart={onRestart}
        onReload={onReload}
      />
      <ProxyServiceDirtyNotice isDirty={isDirty} />
    </div>
  );
}

type ProxyServicePanelProps = ProxyServiceViewProps & { className?: string };

export function ProxyServicePanel({ className, ...props }: ProxyServicePanelProps) {
  return (
    <section data-slot="proxy-service-panel" className={cn("space-y-3", className)}>
      <div className="space-y-1">
        <p className="text-sm font-semibold text-foreground">{m.proxy_service_title()}</p>
        <p className="text-xs text-muted-foreground">{m.proxy_service_desc()}</p>
      </div>
      <ProxyServiceContent {...props} />
    </section>
  );
}
