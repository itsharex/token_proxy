import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
import { Separator } from "@/components/ui/separator";

import "./App.css";

type OpenAiConfig = {
  base_url: string;
  api_key: string | null;
};

type ProxyConfigFile = {
  host: string;
  port: number;
  local_api_key: string | null;
  log_path: string;
  openai: OpenAiConfig;
};

type ConfigResponse = {
  path: string;
  config: ProxyConfigFile;
};

type ConfigForm = {
  host: string;
  port: string;
  localApiKey: string;
  logPath: string;
  openaiBaseUrl: string;
  openaiApiKey: string;
};

type StatusState = "idle" | "loading" | "saving" | "saved" | "error";

type StatusBadge = {
  label: string;
  variant: "default" | "secondary" | "destructive" | "outline";
};

const EMPTY_FORM: ConfigForm = {
  host: "127.0.0.1",
  port: "9208",
  localApiKey: "",
  logPath: "proxy.log",
  openaiBaseUrl: "https://api.openai.com",
  openaiApiKey: "",
};

function toForm(config: ProxyConfigFile): ConfigForm {
  return {
    host: config.host,
    port: String(config.port),
    localApiKey: config.local_api_key ?? "",
    logPath: config.log_path,
    openaiBaseUrl: config.openai.base_url,
    openaiApiKey: config.openai.api_key ?? "",
  };
}

function toPayload(form: ConfigForm): ProxyConfigFile {
  const port = Number.parseInt(form.port, 10);
  return {
    host: form.host.trim(),
    port,
    local_api_key: form.localApiKey.trim() ? form.localApiKey.trim() : null,
    log_path: form.logPath.trim(),
    openai: {
      base_url: form.openaiBaseUrl.trim(),
      api_key: form.openaiApiKey.trim() ? form.openaiApiKey.trim() : null,
    },
  };
}

function parseError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function validate(form: ConfigForm) {
  if (!form.host.trim()) {
    return { valid: false, message: "Host is required." };
  }
  const port = Number.parseInt(form.port, 10);
  if (!Number.isFinite(port) || port < 1 || port > 65535) {
    return { valid: false, message: "Port must be between 1 and 65535." };
  }
  if (!form.openaiBaseUrl.trim()) {
    return { valid: false, message: "OpenAI Base URL is required." };
  }
  if (!form.logPath.trim()) {
    return { valid: false, message: "Log path is required." };
  }
  return { valid: true, message: "" };
}

function createStatusBadge(
  status: StatusState,
  isDirty: boolean,
  lastConfig: ProxyConfigFile | null
): StatusBadge {
  if (status === "loading" || status === "saving") {
    return { label: "Working", variant: "secondary" };
  }
  if (status === "error") {
    return { label: "Error", variant: "destructive" };
  }
  if (isDirty) {
    return { label: "Unsaved", variant: "secondary" };
  }
  if (lastConfig) {
    return { label: "Saved", variant: "default" };
  }
  return { label: "Idle", variant: "outline" };
}

function useConfigState() {
  const [form, setForm] = useState<ConfigForm>(EMPTY_FORM);
  const [configPath, setConfigPath] = useState("");
  const [lastConfig, setLastConfig] = useState<ProxyConfigFile | null>(null);
  const [status, setStatus] = useState<StatusState>("idle");
  const [statusMessage, setStatusMessage] = useState("");
  const [savedAt, setSavedAt] = useState("");
  const [showLocalKey, setShowLocalKey] = useState(false);
  const [showOpenAiKey, setShowOpenAiKey] = useState(false);

  const updateForm = useCallback((patch: Partial<ConfigForm>) => {
    setForm((prev) => ({ ...prev, ...patch }));
  }, []);

  return {
    form,
    configPath,
    lastConfig,
    savedAt,
    showLocalKey,
    showOpenAiKey,
    status,
    statusMessage,
    setConfigPath,
    setForm,
    setLastConfig,
    setSavedAt,
    setShowLocalKey,
    setShowOpenAiKey,
    setStatus,
    setStatusMessage,
    updateForm,
  };
}

function useConfigDerived(
  form: ConfigForm,
  lastConfig: ProxyConfigFile | null,
  status: StatusState
) {
  const validation = useMemo(() => validate(form), [form]);
  const currentPayload = useMemo(
    () => (validation.valid ? toPayload(form) : null),
    [form, validation.valid]
  );

  const isDirty = useMemo(() => {
    if (!currentPayload || !lastConfig) {
      return false;
    }
    return JSON.stringify(currentPayload) !== JSON.stringify(lastConfig);
  }, [currentPayload, lastConfig]);

  const statusBadge = useMemo(
    () => createStatusBadge(status, isDirty, lastConfig),
    [isDirty, lastConfig, status]
  );

  const canSave = status !== "saving" && validation.valid && isDirty;

  return { validation, currentPayload, isDirty, statusBadge, canSave };
}

type ConfigActionsArgs = {
  currentPayload: ProxyConfigFile | null;
  lastConfig: ProxyConfigFile | null;
  validation: { valid: boolean; message: string };
  setConfigPath: (path: string) => void;
  setForm: (value: ConfigForm) => void;
  setLastConfig: (value: ProxyConfigFile | null) => void;
  setSavedAt: (value: string) => void;
  setStatus: (value: StatusState) => void;
  setStatusMessage: (value: string) => void;
};

function useConfigActions({
  currentPayload,
  lastConfig,
  validation,
  setConfigPath,
  setForm,
  setLastConfig,
  setSavedAt,
  setStatus,
  setStatusMessage,
}: ConfigActionsArgs) {
  const loadConfig = useCallback(async () => {
    setStatus("loading");
    setStatusMessage("");
    try {
      const response = await invoke<ConfigResponse>("read_proxy_config");
      setConfigPath(response.path);
      setForm(toForm(response.config));
      setLastConfig(response.config);
      setStatus("idle");
    } catch (error) {
      setStatus("error");
      setStatusMessage(parseError(error));
    }
  }, [setConfigPath, setForm, setLastConfig, setStatus, setStatusMessage]);

  const saveConfig = useCallback(async () => {
    if (!currentPayload) {
      setStatus("error");
      setStatusMessage(validation.message || "Invalid configuration.");
      return;
    }
    setStatus("saving");
    setStatusMessage("");
    try {
      await invoke("write_proxy_config", { config: currentPayload });
      setLastConfig(currentPayload);
      setSavedAt(new Date().toLocaleString());
      setStatus("saved");
    } catch (error) {
      setStatus("error");
      setStatusMessage(parseError(error));
    }
  }, [currentPayload, setLastConfig, setSavedAt, setStatus, setStatusMessage, validation.message]);

  const resetForm = useCallback(() => {
    if (!lastConfig) {
      return;
    }
    setForm(toForm(lastConfig));
    setStatusMessage("");
  }, [lastConfig, setForm, setStatusMessage]);

  return { loadConfig, saveConfig, resetForm };
}

type HeaderSectionProps = {
  statusBadge: StatusBadge;
};

function HeaderSection({ statusBadge }: HeaderSectionProps) {
  return (
    <header className="mb-10 flex flex-col gap-3">
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
        Configure the local proxy for OpenAI-compatible routing. Changes write directly to the
        JSONC config file and require a restart to take effect.
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

function ProxyCoreCard({ form, showLocalKey, onToggleLocalKey, onChange }: ProxyCoreCardProps) {
  return (
    <Card>
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

type OpenAiCardProps = {
  form: ConfigForm;
  showOpenAiKey: boolean;
  onToggleOpenAiKey: () => void;
  onChange: (patch: Partial<ConfigForm>) => void;
};

function OpenAiCard({ form, showOpenAiKey, onToggleOpenAiKey, onChange }: OpenAiCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>OpenAI Upstream</CardTitle>
        <CardDescription>Base URL and upstream credentials.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        <div className="grid gap-2">
          <Label htmlFor="openai-base-url">Base URL</Label>
          <Input
            id="openai-base-url"
            value={form.openaiBaseUrl}
            onChange={(event) => onChange({ openaiBaseUrl: event.target.value })}
            placeholder="https://api.openai.com"
          />
        </div>
        <div className="grid gap-2">
          <Label htmlFor="openai-key">API Key</Label>
          <div className="flex flex-wrap items-center gap-2">
            <Input
              id="openai-key"
              type={showOpenAiKey ? "text" : "password"}
              value={form.openaiApiKey}
              onChange={(event) => onChange({ openaiApiKey: event.target.value })}
              placeholder="Optional"
            />
            <Button type="button" variant="outline" size="sm" onClick={onToggleOpenAiKey}>
              {showOpenAiKey ? "Hide" : "Show"}
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            Leave empty to rely on client-provided keys.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}

type ConfigFileCardProps = {
  configPath: string;
  savedAt: string;
  status: StatusState;
  statusMessage: string;
  canSave: boolean;
  isDirty: boolean;
  onSave: () => void;
  onReset: () => void;
  onReload: () => void;
};

function ConfigFileCard({
  configPath,
  savedAt,
  status,
  statusMessage,
  canSave,
  isDirty,
  onSave,
  onReset,
  onReload,
}: ConfigFileCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Config File</CardTitle>
        <CardDescription>JSONC source of truth for the proxy.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-1">
          <p className="text-xs uppercase tracking-[0.2em] text-muted-foreground">Location</p>
          <p className="font-mono text-xs text-foreground/80">{configPath || "--"}</p>
        </div>
        <Separator />
        <div className="space-y-1 text-sm text-muted-foreground">
          <p>Click save to write the JSONC config to disk.</p>
          <p>Restart the proxy after saving to apply updates.</p>
        </div>
        {savedAt ? (
          <div className="text-xs text-muted-foreground">
            Last saved at <span className="text-foreground">{savedAt}</span>
          </div>
        ) : null}
        {statusMessage ? (
          <div className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">
            {statusMessage}
          </div>
        ) : null}
      </CardContent>
      <CardFooter className="flex flex-wrap gap-2">
        <Button type="button" onClick={onSave} disabled={!canSave}>
          Save to File
        </Button>
        <Button type="button" variant="outline" onClick={onReset} disabled={!isDirty}>
          Reset
        </Button>
        <Button type="button" variant="ghost" onClick={onReload} disabled={status === "loading"}>
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

function ValidationCard({ form, validation }: ValidationCardProps) {
  return (
    <Card>
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
          <span>OpenAI Base URL</span>
          <Badge variant={form.openaiBaseUrl.trim() ? "default" : "destructive"}>
            {form.openaiBaseUrl.trim() ? "Ready" : "Missing"}
          </Badge>
        </div>
      </CardContent>
    </Card>
  );
}

function App() {
  const state = useConfigState();
  const derived = useConfigDerived(state.form, state.lastConfig, state.status);

  const { loadConfig, saveConfig, resetForm } = useConfigActions({
    currentPayload: derived.currentPayload,
    lastConfig: state.lastConfig,
    validation: derived.validation,
    setConfigPath: state.setConfigPath,
    setForm: state.setForm,
    setLastConfig: state.setLastConfig,
    setSavedAt: state.setSavedAt,
    setStatus: state.setStatus,
    setStatusMessage: state.setStatusMessage,
  });

  useEffect(() => {
    void loadConfig();
  }, [loadConfig]);

  return (
    <main className="app-shell">
      <div className="app-content">
        <HeaderSection statusBadge={derived.statusBadge} />
        <section className="grid gap-6 lg:grid-cols-[1.15fr_0.85fr]">
          <div className="space-y-6">
            <ProxyCoreCard
              form={state.form}
              showLocalKey={state.showLocalKey}
              onToggleLocalKey={() => state.setShowLocalKey((value) => !value)}
              onChange={state.updateForm}
            />
            <OpenAiCard
              form={state.form}
              showOpenAiKey={state.showOpenAiKey}
              onToggleOpenAiKey={() => state.setShowOpenAiKey((value) => !value)}
              onChange={state.updateForm}
            />
          </div>
          <div className="space-y-6">
            <ConfigFileCard
              configPath={state.configPath}
              savedAt={state.savedAt}
              status={state.status}
              statusMessage={state.statusMessage}
              canSave={derived.canSave}
              isDirty={derived.isDirty}
              onSave={saveConfig}
              onReset={resetForm}
              onReload={loadConfig}
            />
            <ValidationCard form={state.form} validation={derived.validation} />
          </div>
        </section>
      </div>
    </main>
  );
}

export default App;
