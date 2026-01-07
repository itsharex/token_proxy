import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import {
  type ConfigForm,
  type UpstreamForm,
  UPSTREAM_STRATEGIES,
} from "@/features/config/types";

export type StatusBadge = {
  label: string;
  variant: "default" | "secondary" | "destructive" | "outline";
};

const UPSTREAM_STRATEGY_VALUES: ReadonlySet<string> = new Set(
  UPSTREAM_STRATEGIES.map((strategy) => strategy.value)
);

function toUpstreamStrategy(value: string): ConfigForm["upstreamStrategy"] | null {
  return UPSTREAM_STRATEGY_VALUES.has(value) ? (value as ConfigForm["upstreamStrategy"]) : null;
}

type HeaderSectionProps = {
  statusBadge: StatusBadge;
};

export function HeaderSection({ statusBadge }: HeaderSectionProps) {
  return (
    <header data-slot="header-section" className="mb-10 flex flex-col gap-3">
      <div className="flex items-center justify-between gap-4">
        <div className="space-y-1">
          <p className="text-xs uppercase tracking-[0.3em] text-muted-foreground">
            Local Gateway
          </p>
          <h1 className="title-font text-3xl text-foreground sm:text-4xl">
            Token Proxy Control Deck
          </h1>
        </div>
        <Badge variant={statusBadge.variant}>{statusBadge.label}</Badge>
      </div>
      <p className="max-w-2xl text-sm text-muted-foreground">
        Configure upstream pools and credentials for your OpenAI-compatible proxy. Request format
        routing is handled internally and does not need configuration.
      </p>
    </header>
  );
}

type ProxyCoreCardProps = {
  form: ConfigForm;
  showLocalKey: boolean;
  onToggleLocalKey: () => void;
  onChange: (patch: Partial<ConfigForm>) => void;
};

export function ProxyCoreCard({
  form,
  showLocalKey,
  onToggleLocalKey,
  onChange,
}: ProxyCoreCardProps) {
  return (
    <Card data-slot="proxy-core-card">
      <CardHeader>
        <CardTitle>Proxy Core</CardTitle>
        <CardDescription>Listening address, access control, and log output.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="grid gap-2">
            <Label htmlFor="proxy-host">Host</Label>
            <Input
              id="proxy-host"
              value={form.host}
              onChange={(event) => onChange({ host: event.target.value })}
              placeholder="127.0.0.1"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="proxy-port">Port</Label>
            <Input
              id="proxy-port"
              value={form.port}
              onChange={(event) => onChange({ port: event.target.value })}
              placeholder="9208"
              inputMode="numeric"
            />
          </div>
        </div>
        <div className="grid gap-2">
          <Label htmlFor="proxy-key">Local API Key</Label>
          <div className="flex flex-wrap items-center gap-2">
            <Input
              id="proxy-key"
              type={showLocalKey ? "text" : "password"}
              value={form.localApiKey}
              onChange={(event) => onChange({ localApiKey: event.target.value })}
              placeholder="Optional"
            />
            <Button type="button" variant="outline" size="sm" onClick={onToggleLocalKey}>
              {showLocalKey ? "Hide" : "Show"}
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">Leave empty to disable local auth.</p>
        </div>
        <div className="grid gap-2">
          <Label htmlFor="proxy-log">Log File</Label>
          <Input
            id="proxy-log"
            value={form.logPath}
            onChange={(event) => onChange({ logPath: event.target.value })}
            placeholder="proxy.log"
          />
        </div>
      </CardContent>
    </Card>
  );
}

type StrategyCardProps = {
  strategy: ConfigForm["upstreamStrategy"];
  onChange: (value: ConfigForm["upstreamStrategy"]) => void;
};

export function StrategyCard({ strategy, onChange }: StrategyCardProps) {
  return (
    <Card data-slot="strategy-card">
      <CardHeader>
        <CardTitle>Upstream Strategy</CardTitle>
        <CardDescription>Choose how upstreams are selected globally.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid gap-2">
          <Label htmlFor="upstream-strategy">Strategy</Label>
          <Select
            value={strategy}
            onValueChange={(value) => {
              const nextStrategy = toUpstreamStrategy(value);
              if (nextStrategy) {
                onChange(nextStrategy);
              }
            }}
          >
            <SelectTrigger id="upstream-strategy">
              <SelectValue placeholder="Select strategy" />
            </SelectTrigger>
            <SelectContent>
              {UPSTREAM_STRATEGIES.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <p className="text-xs text-muted-foreground">
          Priority round robin rotates within the highest priority group. Priority fill first uses
          the top entry until it fails.
        </p>
      </CardContent>
    </Card>
  );
}

type UpstreamsCardProps = {
  upstreams: UpstreamForm[];
  showApiKeys: boolean;
  providerOptions: string[];
  onToggleApiKeys: () => void;
  onAdd: () => void;
  onRemove: (index: number) => void;
  onChange: (index: number, patch: Partial<UpstreamForm>) => void;
};

export function UpstreamsCard({
  upstreams,
  showApiKeys,
  providerOptions,
  onToggleApiKeys,
  onAdd,
  onRemove,
  onChange,
}: UpstreamsCardProps) {
  const providerListId = "upstream-provider-options";
  return (
    <Card data-slot="upstreams-card">
      <CardHeader>
        <CardTitle>Upstreams</CardTitle>
        <CardDescription>
          Define provider pools and credentials. Use provider names <code>openai</code> and/or{" "}
          <code>openai-response</code>.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {upstreams.length ? (
          <div className="space-y-4">
            {upstreams.map((upstream, index) => (
              <div
                key={upstream.id || `upstream-${index}`}
                className="space-y-3 rounded-md border border-border/60 bg-background/60 p-4"
              >
                <div className="grid gap-3 sm:grid-cols-2">
                  <div className="grid gap-2">
                    <Label>Id</Label>
                    <Input
                      value={upstream.id}
                      onChange={(event) => onChange(index, { id: event.target.value })}
                      placeholder="openai-default"
                    />
                  </div>
                  <div className="grid gap-2">
                    <Label>Provider</Label>
                    <Input
                      list={providerListId}
                      value={upstream.provider}
                      onChange={(event) => onChange(index, { provider: event.target.value })}
                      placeholder="openai"
                    />
                  </div>
                </div>
                <div className="grid gap-3 sm:grid-cols-2">
                  <div className="grid gap-2">
                    <Label>Base URL</Label>
                    <Input
                      value={upstream.baseUrl}
                      onChange={(event) => onChange(index, { baseUrl: event.target.value })}
                      placeholder="https://api.openai.com"
                    />
                  </div>
                  <div className="grid gap-2">
                    <Label>API Key</Label>
                    <Input
                      type={showApiKeys ? "text" : "password"}
                      value={upstream.apiKey}
                      onChange={(event) => onChange(index, { apiKey: event.target.value })}
                      placeholder="Optional"
                    />
                  </div>
                </div>
                <div className="grid gap-3 sm:grid-cols-[1fr_1fr_auto]">
                  <div className="grid gap-2">
                    <Label>Priority</Label>
                    <Input
                      value={upstream.priority}
                      onChange={(event) => onChange(index, { priority: event.target.value })}
                      placeholder="0"
                      inputMode="numeric"
                    />
                  </div>
                  <div className="grid gap-2">
                    <Label>Index</Label>
                    <Input
                      value={upstream.index}
                      onChange={(event) => onChange(index, { index: event.target.value })}
                      placeholder="Optional"
                      inputMode="numeric"
                    />
                  </div>
                  <div className="flex items-end">
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => onRemove(index)}
                      disabled={upstreams.length <= 1}
                    >
                      Remove
                    </Button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">No upstreams defined yet.</p>
        )}
        <datalist id={providerListId}>
          {providerOptions.map((provider) => (
            <option key={provider} value={provider} />
          ))}
        </datalist>
        <div className="flex flex-wrap items-center gap-2">
          <Button type="button" variant="outline" onClick={onAdd}>
            Add Upstream
          </Button>
          <Button type="button" variant="ghost" size="sm" onClick={onToggleApiKeys}>
            {showApiKeys ? "Hide Keys" : "Show Keys"}
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">
          Priority sorts upstreams in descending order. Index orders entries inside the same
          priority group; empty index values are auto-assigned globally.
        </p>
      </CardContent>
    </Card>
  );
}

type ConfigFileCardProps = {
  configPath: string;
  savedAt: string;
  status: "idle" | "loading" | "saving" | "saved" | "error";
  isDirty: boolean;
  onReset: () => void;
  onReload: () => void;
};

export function ConfigFileCard({
  configPath,
  savedAt,
  status,
  isDirty,
  onReset,
  onReload,
}: ConfigFileCardProps) {
  return (
    <Card data-slot="config-file-card">
      <CardHeader>
        <CardTitle>Config File</CardTitle>
        <CardDescription>Disk location and maintenance actions.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-1">
          <p className="text-xs uppercase tracking-[0.2em] text-muted-foreground">Location</p>
          <p className="font-mono text-xs text-foreground/80">{configPath || "--"}</p>
        </div>
        <Separator />
        <div className="space-y-1 text-sm text-muted-foreground">
          <p>Use the toolbar to save changes back to the JSONC file.</p>
          <p>Restart the proxy after saving to apply updates.</p>
        </div>
        {savedAt ? (
          <div className="text-xs text-muted-foreground">
            Last saved at <span className="text-foreground">{savedAt}</span>
          </div>
        ) : null}
        {isDirty ? (
          <div className="rounded-md border border-border/60 bg-background/60 p-3 text-xs text-muted-foreground">
            You have unsaved changes. Reload is disabled to avoid overwriting your edits.
          </div>
        ) : null}
      </CardContent>
      <CardFooter className="flex flex-wrap gap-2">
        <AlertDialog>
          <AlertDialogTrigger asChild>
            <Button type="button" variant="outline" disabled={!isDirty}>
              Discard changes
            </Button>
          </AlertDialogTrigger>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Discard unsaved changes?</AlertDialogTitle>
              <AlertDialogDescription>
                This will restore the form to the last loaded config file values.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction type="button" onClick={onReset}>
                Discard
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
        <Button
          type="button"
          variant="ghost"
          onClick={onReload}
          disabled={status === "loading" || isDirty}
        >
          Reload
        </Button>
      </CardFooter>
    </Card>
  );
}

type ValidationCardProps = {
  form: ConfigForm;
  validation: { valid: boolean; message: string };
};

export function ValidationCard({ form, validation }: ValidationCardProps) {
  const hasUpstreams = form.upstreams.length > 0;
  return (
    <Card data-slot="validation-card">
      <CardHeader>
        <CardTitle>Validation</CardTitle>
        <CardDescription>Keep fields consistent before saving.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3 text-sm">
        <div className="flex items-center justify-between">
          <span>Host</span>
          <Badge variant={form.host.trim() ? "default" : "destructive"}>
            {form.host.trim() ? "Ready" : "Missing"}
          </Badge>
        </div>
        <div className="flex items-center justify-between">
          <span>Port</span>
          <Badge variant={validation.valid ? "default" : "secondary"}>
            {validation.valid ? "OK" : "Check"}
          </Badge>
        </div>
        <div className="flex items-center justify-between">
          <span>Upstreams</span>
          <Badge variant={hasUpstreams ? "default" : "destructive"}>
            {hasUpstreams ? "Ready" : "Missing"}
          </Badge>
        </div>
      </CardContent>
    </Card>
  );
}
