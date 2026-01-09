import { useMemo, useState } from "react";

import {
  AlertCircle,
  Loader2,
  PanelLeftClose,
  PanelLeftOpen,
  RefreshCw,
  Settings2,
} from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { LanguageSwitcher } from "@/components/LanguageSwitcher";
import {
  ConfigFileCard,
  ProxyCoreCard,
  StrategyCard,
  UpstreamsCard,
  ValidationCard,
  type StatusBadge,
} from "@/features/config/cards";
import type { ConfigSection, ConfigSectionId } from "@/features/config/sections";
import { CONFIG_SECTIONS, findSection, toConfigSectionId } from "@/features/config/sections";
import { DashboardScreen } from "@/features/dashboard/DashboardScreen";
import type { ProxyServiceViewProps } from "@/features/config/cards/proxy-service-card";
import type { ConfigForm, ProxyServiceRequestState, ProxyServiceStatus } from "@/features/config/types";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages.js";

type ConfigSidebarProps = {
  statusBadge: StatusBadge;
  activeSection: ConfigSectionId;
  sidebarOpen: boolean;
};

type ConfigStatusIndicatorProps = {
  statusBadge: StatusBadge;
};

function ConfigStatusIndicator({ statusBadge }: ConfigStatusIndicatorProps) {
  const dotClassName = cn(
    "size-1.5 rounded-full",
    statusBadge.variant === "default"
      ? "bg-primary"
      : statusBadge.variant === "secondary"
        ? "bg-muted-foreground/35"
        : statusBadge.variant === "destructive"
          ? "bg-destructive"
          : "bg-border",
    statusBadge.id === "working" && "animate-pulse"
  );

  const textClassName = cn(
    "truncate text-[11px] font-medium leading-none",
    statusBadge.variant === "destructive" ? "text-destructive" : "text-muted-foreground"
  );

  return (
    <div data-slot="config-sidebar-status" className="flex min-w-0 items-center gap-1.5">
      <span aria-hidden="true" className={dotClassName} />
      <span className={textClassName}>{statusBadge.label}</span>
    </div>
  );
}

function ConfigSidebar({ statusBadge, activeSection, sidebarOpen }: ConfigSidebarProps) {
  return (
    <aside
      data-slot="config-sidebar"
      aria-hidden={!sidebarOpen}
      className={cn(
        "flex min-h-0 flex-col overflow-hidden bg-background transition-[width] duration-200",
        sidebarOpen ? "" : "w-0"
      )}
    >
      <div className={cn("flex min-h-0 flex-1 flex-col", !sidebarOpen && "hidden")}>
        <div className="flex items-center gap-3 px-4 py-3">
          <div className="grid size-8 place-items-center rounded-lg border border-border/60 bg-background shadow-sm">
            <Settings2 className="size-4 text-foreground" aria-hidden="true" />
          </div>
          <div className="min-w-0 flex-1">
            <p className="title-font truncate text-sm font-semibold text-foreground">Token Proxy</p>
            <div className="mt-1 min-w-0">
              <ConfigStatusIndicator statusBadge={statusBadge} />
            </div>
          </div>
        </div>
        <ScrollArea data-slot="config-sidebar-scroll" className="min-h-0 flex-1 px-2 pb-4">
          <TabsList
            data-slot="config-sidebar-tabs"
            className="h-auto w-full flex-col items-stretch justify-start gap-1 bg-transparent p-0"
          >
            {CONFIG_SECTIONS.map((section) => {
              const Icon = section.icon;
              const isActive = section.id === activeSection;
              return (
                <TabsTrigger
                  key={section.id}
                  value={section.id}
                  aria-current={isActive ? "page" : undefined}
                  className={cn(
                    "group flex h-auto w-full items-center justify-start gap-2.5 rounded-md px-2.5 py-2 text-left transition-colors",
                    "hover:bg-accent/60",
                    "data-[state=active]:bg-accent data-[state=active]:text-accent-foreground data-[state=active]:shadow-none"
                  )}
                >
                  <Icon
                    className={cn(
                      "size-4 shrink-0 text-muted-foreground transition-colors",
                      isActive ? "text-foreground" : "group-hover:text-foreground"
                    )}
                    aria-hidden="true"
                  />
                  <span className="min-w-0 truncate text-sm font-medium">{section.label()}</span>
                </TabsTrigger>
              );
            })}
          </TabsList>
        </ScrollArea>
      </div>
    </aside>
  );
}

type AppViewProps = {
  activeSectionId: ConfigSectionId;
  form: ConfigForm;
  statusBadge: StatusBadge;
  showLocalKey: boolean;
  showUpstreamKeys: boolean;
  providerOptions: string[];
  configPath: string;
  savedAt: string;
  proxyServiceStatus: ProxyServiceStatus | null;
  proxyServiceRequestState: ProxyServiceRequestState;
  proxyServiceMessage: string;
  status: "idle" | "loading" | "saving" | "saved" | "error";
  statusMessage: string;
  canSave: boolean;
  isDirty: boolean;
  validation: { valid: boolean; message: string };
  onToggleLocalKey: () => void;
  onToggleUpstreamKeys: () => void;
  onFormChange: (patch: Partial<ConfigForm>) => void;
  onStrategyChange: (value: ConfigForm["upstreamStrategy"]) => void;
  onAddUpstream: (upstream: ConfigForm["upstreams"][number]) => void;
  onRemoveUpstream: (index: number) => void;
  onChangeUpstream: (index: number, patch: Partial<ConfigForm["upstreams"][number]>) => void;
  onSave: () => void;
  onReset: () => void;
  onReload: () => void;
  onProxyServiceRefresh: () => void;
  onProxyServiceStart: () => void;
  onProxyServiceStop: () => void;
  onProxyServiceRestart: () => void;
  onProxyServiceReload: () => void;
  onSectionChange: (next: ConfigSectionId) => void;
};

type ConfigToolbarProps = {
  section: ConfigSection;
  status: AppViewProps["status"];
  canSave: boolean;
  isDirty: boolean;
  sidebarOpen: boolean;
  onToggleSidebar: () => void;
  onReload: () => void;
  onSave: () => void;
};

type SidebarToggleButtonProps = {
  open: boolean;
  onToggle: () => void;
};

function SidebarToggleButton({ open, onToggle }: SidebarToggleButtonProps) {
  const Icon = open ? PanelLeftClose : PanelLeftOpen;
  const label = open ? m.sidebar_hide() : m.sidebar_show();

  return (
    <Button type="button" variant="outline" size="icon" onClick={onToggle}>
      <Icon className="size-4" aria-hidden="true" />
      <span className="sr-only">{label}</span>
    </Button>
  );
}

function ConfigToolbar({
  section,
  status,
  canSave,
  isDirty,
  sidebarOpen,
  onToggleSidebar,
  onReload,
  onSave,
}: ConfigToolbarProps) {
  const isLoading = status === "loading";
  const isSaving = status === "saving";
  const canReload = !isLoading && !isDirty;

  return (
    <header
      data-slot="config-toolbar"
      className="flex items-center justify-between gap-3 border-b border-border/60 bg-background/70 px-6 py-4 backdrop-blur"
    >
      <div className="flex min-w-0 items-center gap-3">
        <SidebarToggleButton open={sidebarOpen} onToggle={onToggleSidebar} />
        <div className="min-w-0">
          <p className="truncate text-sm font-medium text-foreground">{section.label()}</p>
          <p className="truncate text-xs text-muted-foreground">{section.description()}</p>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <LanguageSwitcher triggerClassName="h-10 w-[140px]" />
        <Button type="button" variant="outline" size="icon" onClick={onReload} disabled={!canReload}>
          <RefreshCw className={isLoading ? "animate-spin" : undefined} aria-hidden="true" />
        </Button>
        <Button type="button" onClick={onSave} disabled={!canSave}>
          {isSaving ? <Loader2 className="animate-spin" aria-hidden="true" /> : m.common_save()}
        </Button>
      </div>
    </header>
  );
}

type StatusAlertProps = {
  statusMessage: string;
};

function StatusAlert({ statusMessage }: StatusAlertProps) {
  if (!statusMessage) {
    return null;
  }
  return (
    <Alert variant="destructive" className="mb-6">
      <AlertCircle className="size-4" aria-hidden="true" />
      <div>
        <AlertTitle>{m.config_request_failed_title()}</AlertTitle>
        <AlertDescription>{statusMessage}</AlertDescription>
      </div>
    </Alert>
  );
}

type ConfigSectionContentProps = Omit<AppViewProps, "activeSectionId" | "onSectionChange"> & {
  activeSectionId: ConfigSectionId;
  sidebarOpen: boolean;
  onToggleSidebar: () => void;
  proxyService: ProxyServiceViewProps;
};

type ConfigSectionBodyProps = Omit<ConfigSectionContentProps, "sidebarOpen" | "onToggleSidebar">;

function ConfigSectionBody({ activeSectionId, proxyService, ...props }: ConfigSectionBodyProps) {
  switch (activeSectionId) {
    case "core":
      return (
        <ProxyCoreCard
          form={props.form}
          showLocalKey={props.showLocalKey}
          onToggleLocalKey={props.onToggleLocalKey}
          onChange={props.onFormChange}
          proxyService={proxyService}
        />
      );
    case "strategy":
      return (
        <StrategyCard strategy={props.form.upstreamStrategy} onChange={props.onStrategyChange} />
      );
    case "upstreams":
      return (
        <UpstreamsCard
          upstreams={props.form.upstreams}
          showApiKeys={props.showUpstreamKeys}
          providerOptions={props.providerOptions}
          onToggleApiKeys={props.onToggleUpstreamKeys}
          onAdd={props.onAddUpstream}
          onRemove={props.onRemoveUpstream}
          onChange={props.onChangeUpstream}
        />
      );
    case "file":
      return (
        <ConfigFileCard
          configPath={props.configPath}
          savedAt={props.savedAt}
          isDirty={props.isDirty}
          onReset={props.onReset}
        />
      );
    case "validation":
      return <ValidationCard form={props.form} validation={props.validation} />;
    default:
      return null;
  }
}

function ConfigSectionContent({
  activeSectionId,
  sidebarOpen,
  onToggleSidebar,
  proxyService,
  ...props
}: ConfigSectionContentProps) {
  if (activeSectionId === "dashboard") {
    return (
      <DashboardScreen
        variant="embedded"
        headerLeading={<SidebarToggleButton open={sidebarOpen} onToggle={onToggleSidebar} />}
      />
    );
  }

  const content = (
    <ConfigSectionBody {...props} activeSectionId={activeSectionId} proxyService={proxyService} />
  );

  return (
    <div className="py-6 pr-6">
      <StatusAlert statusMessage={props.statusMessage} />
      {content}
    </div>
  );
}

function toProxyServiceViewProps(props: AppViewProps) {
  return {
    status: props.proxyServiceStatus,
    requestState: props.proxyServiceRequestState,
    message: props.proxyServiceMessage,
    isDirty: props.isDirty,
    onRefresh: props.onProxyServiceRefresh,
    onStart: props.onProxyServiceStart,
    onStop: props.onProxyServiceStop,
    onRestart: props.onProxyServiceRestart,
    onReload: props.onProxyServiceReload,
  };
}

export function AppView(props: AppViewProps) {
  const { activeSectionId, onSectionChange, ...viewProps } = props;
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const sectionMeta = useMemo(() => findSection(activeSectionId), [activeSectionId]);
  const isDashboard = activeSectionId === "dashboard";
  const toggleSidebar = () => setSidebarOpen((value) => !value);
  const proxyService = toProxyServiceViewProps(props);

  return (
    <Tabs
      data-slot="config-desktop-shell"
      value={activeSectionId}
      onValueChange={(value) => {
        const nextSection = toConfigSectionId(value);
        if (nextSection) {
          onSectionChange(nextSection);
        }
      }}
      orientation="vertical"
      className="relative z-10 grid h-full min-h-0 grid-cols-[max-content_1fr]"
    >
      <ConfigSidebar
        statusBadge={viewProps.statusBadge}
        activeSection={activeSectionId}
        sidebarOpen={sidebarOpen}
      />
      <section
        data-slot="config-main"
        className="flex min-h-0 flex-col bg-background"
      >
        {isDashboard ? null : (
          <ConfigToolbar
            section={sectionMeta}
            status={viewProps.status}
            canSave={viewProps.canSave}
            isDirty={viewProps.isDirty}
            sidebarOpen={sidebarOpen}
            onToggleSidebar={toggleSidebar}
            onReload={viewProps.onReload}
            onSave={viewProps.onSave}
          />
        )}
        <ScrollArea data-slot="config-main-scroll" className="min-h-0 flex-1">
          <ConfigSectionContent
            {...viewProps}
            activeSectionId={activeSectionId}
            sidebarOpen={sidebarOpen}
            onToggleSidebar={toggleSidebar}
            proxyService={proxyService}
          />
        </ScrollArea>
      </section>
    </Tabs>
  );
}
