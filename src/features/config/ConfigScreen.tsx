import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { AppView } from "@/features/config/AppView";
import { type StatusBadge } from "@/features/config/cards";
import { EMPTY_FORM, toForm, toPayload, validate } from "@/features/config/form";
import { useConfigListActions } from "@/features/config/list-actions";
import type { ConfigForm, ConfigResponse, ProxyConfigFile } from "@/features/config/types";
import { parseError } from "@/lib/error";

type StatusState = "idle" | "loading" | "saving" | "saved" | "error";

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
  const [showUpstreamKeys, setShowUpstreamKeys] = useState(false);

  const updateForm = useCallback((patch: Partial<ConfigForm>) => {
    setForm((prev) => ({ ...prev, ...patch }));
  }, []);

  return {
    form,
    configPath,
    lastConfig,
    savedAt,
    showLocalKey,
    showUpstreamKeys,
    status,
    statusMessage,
    setConfigPath,
    setForm,
    setLastConfig,
    setSavedAt,
    setShowLocalKey,
    setShowUpstreamKeys,
    setStatus,
    setStatusMessage,
    updateForm,
  };
}

function useConfigDerived(form: ConfigForm, lastConfig: ProxyConfigFile | null, status: StatusState) {
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

  const providerOptions = useMemo(() => {
    const providers = new Set<string>();
    for (const upstream of form.upstreams) {
      const provider = upstream.provider.trim();
      if (provider) {
        providers.add(provider);
      }
    }
    return Array.from(providers);
  }, [form.upstreams]);

  const canSave = status !== "saving" && validation.valid && isDirty;

  return { validation, currentPayload, isDirty, statusBadge, canSave, providerOptions };
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

export function ConfigScreen() {
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
  const listActions = useConfigListActions(state.setForm);

  useEffect(() => {
    void loadConfig();
  }, [loadConfig]);

  return (
    <AppView
      form={state.form}
      statusBadge={derived.statusBadge}
      showLocalKey={state.showLocalKey}
      showUpstreamKeys={state.showUpstreamKeys}
      providerOptions={derived.providerOptions}
      configPath={state.configPath}
      savedAt={state.savedAt}
      status={state.status}
      statusMessage={state.statusMessage}
      canSave={derived.canSave}
      isDirty={derived.isDirty}
      validation={derived.validation}
      onToggleLocalKey={() => state.setShowLocalKey((value) => !value)}
      onToggleUpstreamKeys={() => state.setShowUpstreamKeys((value) => !value)}
      onFormChange={state.updateForm}
      onStrategyChange={(value) => state.updateForm({ upstreamStrategy: value })}
      onAddUpstream={listActions.addUpstream}
      onRemoveUpstream={listActions.removeUpstream}
      onChangeUpstream={listActions.updateUpstream}
      onSave={saveConfig}
      onReset={resetForm}
      onReload={loadConfig}
    />
  );
}

