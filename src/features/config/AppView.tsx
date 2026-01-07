import {
  ConfigFileCard,
  HeaderSection,
  ProxyCoreCard,
  StrategyCard,
  UpstreamsCard,
  ValidationCard,
  type StatusBadge,
} from "@/features/config/cards";
import type { ConfigForm } from "@/features/config/types";

type AppViewProps = {
  form: ConfigForm;
  statusBadge: StatusBadge;
  showLocalKey: boolean;
  showUpstreamKeys: boolean;
  providerOptions: string[];
  configPath: string;
  savedAt: string;
  status: "idle" | "loading" | "saving" | "saved" | "error";
  statusMessage: string;
  canSave: boolean;
  isDirty: boolean;
  validation: { valid: boolean; message: string };
  onToggleLocalKey: () => void;
  onToggleUpstreamKeys: () => void;
  onFormChange: (patch: Partial<ConfigForm>) => void;
  onStrategyChange: (value: ConfigForm["upstreamStrategy"]) => void;
  onAddUpstream: () => void;
  onRemoveUpstream: (index: number) => void;
  onChangeUpstream: (index: number, patch: Partial<ConfigForm["upstreams"][number]>) => void;
  onSave: () => void;
  onReset: () => void;
  onReload: () => void;
};

type LeftColumnProps = {
  form: ConfigForm;
  showLocalKey: boolean;
  showUpstreamKeys: boolean;
  providerOptions: string[];
  onToggleLocalKey: () => void;
  onToggleUpstreamKeys: () => void;
  onFormChange: (patch: Partial<ConfigForm>) => void;
  onStrategyChange: (value: ConfigForm["upstreamStrategy"]) => void;
  onAddUpstream: () => void;
  onRemoveUpstream: (index: number) => void;
  onChangeUpstream: (index: number, patch: Partial<ConfigForm["upstreams"][number]>) => void;
};

function LeftColumn({
  form,
  showLocalKey,
  showUpstreamKeys,
  providerOptions,
  onToggleLocalKey,
  onToggleUpstreamKeys,
  onFormChange,
  onStrategyChange,
  onAddUpstream,
  onRemoveUpstream,
  onChangeUpstream,
}: LeftColumnProps) {
  return (
    <div className="space-y-6">
      <ProxyCoreCard
        form={form}
        showLocalKey={showLocalKey}
        onToggleLocalKey={onToggleLocalKey}
        onChange={onFormChange}
      />
      <StrategyCard strategy={form.upstreamStrategy} onChange={onStrategyChange} />
      <UpstreamsCard
        upstreams={form.upstreams}
        showApiKeys={showUpstreamKeys}
        providerOptions={providerOptions}
        onToggleApiKeys={onToggleUpstreamKeys}
        onAdd={onAddUpstream}
        onRemove={onRemoveUpstream}
        onChange={onChangeUpstream}
      />
    </div>
  );
}

type RightColumnProps = {
  form: ConfigForm;
  configPath: string;
  savedAt: string;
  status: "idle" | "loading" | "saving" | "saved" | "error";
  statusMessage: string;
  canSave: boolean;
  isDirty: boolean;
  validation: { valid: boolean; message: string };
  onSave: () => void;
  onReset: () => void;
  onReload: () => void;
};

function RightColumn({
  form,
  configPath,
  savedAt,
  status,
  statusMessage,
  canSave,
  isDirty,
  validation,
  onSave,
  onReset,
  onReload,
}: RightColumnProps) {
  return (
    <div className="space-y-6">
      <ConfigFileCard
        configPath={configPath}
        savedAt={savedAt}
        status={status}
        statusMessage={statusMessage}
        canSave={canSave}
        isDirty={isDirty}
        onSave={onSave}
        onReset={onReset}
        onReload={onReload}
      />
      <ValidationCard form={form} validation={validation} />
    </div>
  );
}

export function AppView({
  form,
  statusBadge,
  showLocalKey,
  showUpstreamKeys,
  providerOptions,
  configPath,
  savedAt,
  status,
  statusMessage,
  canSave,
  isDirty,
  validation,
  onToggleLocalKey,
  onToggleUpstreamKeys,
  onFormChange,
  onStrategyChange,
  onAddUpstream,
  onRemoveUpstream,
  onChangeUpstream,
  onSave,
  onReset,
  onReload,
}: AppViewProps) {
  return (
    <main className="app-shell">
      <div className="app-content">
        <HeaderSection statusBadge={statusBadge} />
        <section className="grid gap-6 lg:grid-cols-[1.15fr_0.85fr]">
          <LeftColumn
            form={form}
            showLocalKey={showLocalKey}
            showUpstreamKeys={showUpstreamKeys}
            providerOptions={providerOptions}
            onToggleLocalKey={onToggleLocalKey}
            onToggleUpstreamKeys={onToggleUpstreamKeys}
            onFormChange={onFormChange}
            onStrategyChange={onStrategyChange}
            onAddUpstream={onAddUpstream}
            onRemoveUpstream={onRemoveUpstream}
            onChangeUpstream={onChangeUpstream}
          />
          <RightColumn
            form={form}
            configPath={configPath}
            savedAt={savedAt}
            status={status}
            statusMessage={statusMessage}
            canSave={canSave}
            isDirty={isDirty}
            validation={validation}
            onSave={onSave}
            onReset={onReset}
            onReload={onReload}
          />
        </section>
      </div>
    </main>
  );
}
