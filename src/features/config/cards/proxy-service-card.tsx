import { Loader2, Play, RefreshCw, RotateCcw, Square } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import type { ProxyServiceStatus } from "@/features/config/types";
import { cn } from "@/lib/utils";

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
    return { label: "Error", variant: "destructive" as const };
  }
  if (!status) {
    return { label: "Unknown", variant: "outline" as const };
  }
  if (status.state === "running") {
    return { label: "Running", variant: "default" as const };
  }
  return { label: "Stopped", variant: "secondary" as const };
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
        <CardTitle>Proxy Service</CardTitle>
        <CardDescription>Safe stop/restart and manual config reload.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3 text-sm">
          <div className="flex items-center gap-2">
            <span className="text-muted-foreground">State</span>
            <Badge variant={badge.variant}>{badge.label}</Badge>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-muted-foreground">Addr</span>
            <span className="font-mono text-xs text-foreground/80">{addr}</span>
          </div>
        </div>
        <Separator />
        <div className="grid gap-2 text-xs text-muted-foreground">
          <p>
            The proxy reads configuration from the JSONC file on disk. Save your changes first to
            apply them.
          </p>
          <p>Saving the config triggers an automatic reload (and safe restart if host/port changes).</p>
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
            Start
          </Button>
          <Button type="button" variant="outline" onClick={onStop} disabled={isWorking || !isRunning}>
            <Square aria-hidden="true" />
            Stop
          </Button>
          <Button type="button" variant="outline" onClick={onRestart} disabled={isWorking || !isRunning || isDirty}>
            <RotateCcw aria-hidden="true" />
            Restart
          </Button>
          <Button type="button" variant="outline" onClick={onReload} disabled={isWorking || isDirty}>
            Reload config
          </Button>
        </div>
        {isDirty ? (
          <div className="rounded-md border border-border/60 bg-background/60 p-3 text-xs text-muted-foreground">
            You have unsaved changes. Restart/Reload is disabled to avoid applying stale disk config.
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}

