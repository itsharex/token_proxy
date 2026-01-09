import { useMemo, useState } from "react";

import type { LucideIcon } from "lucide-react";
import {
  AlertCircle,
  CircleCheck,
  FileJson,
  LayoutDashboard,
  Loader2,
  PanelLeftClose,
  PanelLeftOpen,
  RefreshCw,
  Server,
  Settings2,
  Shuffle,
  SlidersHorizontal,
} from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { LanguageSwitcher } from "@/components/LanguageSwitcher";
import {
  ConfigFileCard,
  ProxyCoreCard,
  StrategyCard,
  UpstreamsCard,
  ValidationCard,
  type StatusBadge,
} from "@/features/config/cards";
import { DashboardScreen } from "@/features/dashboard/DashboardScreen";
import type { ProxyServiceViewProps } from "@/features/config/cards/proxy-service-card";
import type { ConfigForm, ProxyServiceRequestState, ProxyServiceStatus } from "@/features/config/types";
import { cn } from "@/lib/utils";
import { m } from "@/paraglide/messages.js";

type ConfigSectionId = "dashboard" | "core" | "strategy" | "upstreams" | "file" | "validation";

type ConfigSection = {
  id: ConfigSectionId;
  label: () => string;
  description: () => string;
  icon: LucideIcon;
};

const CONFIG_SECTIONS: readonly ConfigSection[] = [
  {
    id: "dashboard",
    label: () => m.config_section_dashboard_label(),
    description: () => m.config_section_dashboard_desc(),
    icon: LayoutDashboard,
  },
  {
    id: "core",
    label: () => m.config_section_core_label(),
    description: () => m.config_section_core_desc(),
    icon: SlidersHorizontal,
  },
  {
    id: "strategy",
    label: () => m.config_section_strategy_label(),
    description: () => m.config_section_strategy_desc(),
    icon: Shuffle,
  },
  {
    id: "upstreams",
    label: () => m.config_section_upstreams_label(),
    description: () => m.config_section_upstreams_desc(),
    icon: Server,
  },
  {
    id: "file",
    label: () => m.config_section_file_label(),
    description: () => m.config_section_file_desc(),
    icon: FileJson,
  },
  {
    id: "validation",
    label: () => m.config_section_validation_label(),
    description: () => m.config_section_validation_desc(),
    icon: CircleCheck,
  },
] as const;

const CONFIG_SECTION_IDS: ReadonlySet<string> = new Set(CONFIG_SECTIONS.map((section) => section.id));

function findSection(sectionId: ConfigSectionId) {
  return CONFIG_SECTIONS.find((section) => section.id === sectionId) ?? CONFIG_SECTIONS[0];
}

function toConfigSectionId(value: string): ConfigSectionId | null {
  return CONFIG_SECTION_IDS.has(value) ? (value as ConfigSectionId) : null;
}

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
        "flex min-h-0 flex-col overflow-hidden bg-background/60 backdrop-blur transition-[width] duration-200",
        sidebarOpen ? "border-r border-border/60" : "w-0 border-r-0"
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

type DashboardTabProps = {
  sidebarOpen: boolean;
  onToggleSidebar: () => void;
};

function DashboardTab({ sidebarOpen, onToggleSidebar }: DashboardTabProps) {
  return (
    <TabsContent value="dashboard" className="mt-0">
      <DashboardScreen
        variant="embedded"
        headerLeading={<SidebarToggleButton open={sidebarOpen} onToggle={onToggleSidebar} />}
      />
    </TabsContent>
  );
}

type CoreTabProps = {
  form: ConfigForm;
  showLocalKey: boolean;
  onToggleLocalKey: () => void;
  onFormChange: (patch: Partial<ConfigForm>) => void;
  proxyService: ProxyServiceViewProps;
};

function CoreTab({
  form,
  showLocalKey,
  onToggleLocalKey,
  onFormChange,
  proxyService,
}: CoreTabProps) {
  return (
    <TabsContent value="core" className="mt-0">
      <ProxyCoreCard
        form={form}
        showLocalKey={showLocalKey}
        onToggleLocalKey={onToggleLocalKey}
        onChange={onFormChange}
        proxyService={proxyService}
      />
    </TabsContent>
  );
}

type StrategyTabProps = {
  strategy: ConfigForm["upstreamStrategy"];
  onChange: (value: ConfigForm["upstreamStrategy"]) => void;
};

function StrategyTab({ strategy, onChange }: StrategyTabProps) {
  return (
    <TabsContent value="strategy" className="mt-0">
      <StrategyCard strategy={strategy} onChange={onChange} />
    </TabsContent>
  );
}

type UpstreamsTabProps = {
  upstreams: ConfigForm["upstreams"];
  showApiKeys: boolean;
  providerOptions: string[];
  onToggleApiKeys: () => void;
  onAdd: (upstream: ConfigForm["upstreams"][number]) => void;
  onRemove: (index: number) => void;
  onChange: (index: number, patch: Partial<ConfigForm["upstreams"][number]>) => void;
};

function UpstreamsTab({
  upstreams,
  showApiKeys,
  providerOptions,
  onToggleApiKeys,
  onAdd,
  onRemove,
  onChange,
}: UpstreamsTabProps) {
  return (
    <TabsContent value="upstreams" className="mt-0">
      <UpstreamsCard
        upstreams={upstreams}
        showApiKeys={showApiKeys}
        providerOptions={providerOptions}
        onToggleApiKeys={onToggleApiKeys}
        onAdd={onAdd}
        onRemove={onRemove}
        onChange={onChange}
      />
    </TabsContent>
  );
}

type FileTabProps = {
  configPath: string;
  savedAt: string;
  isDirty: boolean;
  onReset: () => void;
};

function FileTab({ configPath, savedAt, isDirty, onReset }: FileTabProps) {
  return (
    <TabsContent value="file" className="mt-0">
      <ConfigFileCard
        configPath={configPath}
        savedAt={savedAt}
        isDirty={isDirty}
        onReset={onReset}
      />
    </TabsContent>
  );
}

type ValidationTabProps = {
  form: ConfigForm;
  validation: { valid: boolean; message: string };
};

function ValidationTab({ form, validation }: ValidationTabProps) {
  return (
    <TabsContent value="validation" className="mt-0">
      <ValidationCard form={form} validation={validation} />
    </TabsContent>
  );
}

type ConfigTabsProps = AppViewProps & { proxyService: ProxyServiceViewProps };

function ConfigTabs({ proxyService, ...props }: ConfigTabsProps) {
  return (
    <div className="p-6">
      <StatusAlert statusMessage={props.statusMessage} />
      <CoreTab
        form={props.form}
        showLocalKey={props.showLocalKey}
        onToggleLocalKey={props.onToggleLocalKey}
        onFormChange={props.onFormChange}
        proxyService={proxyService}
      />
      <StrategyTab strategy={props.form.upstreamStrategy} onChange={props.onStrategyChange} />
      <UpstreamsTab
        upstreams={props.form.upstreams}
        showApiKeys={props.showUpstreamKeys}
        providerOptions={props.providerOptions}
        onToggleApiKeys={props.onToggleUpstreamKeys}
        onAdd={props.onAddUpstream}
        onRemove={props.onRemoveUpstream}
        onChange={props.onChangeUpstream}
      />
      <FileTab
        configPath={props.configPath}
        savedAt={props.savedAt}
        isDirty={props.isDirty}
        onReset={props.onReset}
      />
      <ValidationTab form={props.form} validation={props.validation} />
    </div>
  );
}

type ConfigContentProps = AppViewProps & {
  isDashboard: boolean;
  sidebarOpen: boolean;
  onToggleSidebar: () => void;
  proxyService: ProxyServiceViewProps;
};

function ConfigContent({
  isDashboard,
  sidebarOpen,
  onToggleSidebar,
  proxyService,
  ...props
}: ConfigContentProps) {
  return (
    <ScrollArea data-slot="config-main-scroll" className="min-h-0 flex-1">
      {isDashboard ? (
        <DashboardTab sidebarOpen={sidebarOpen} onToggleSidebar={onToggleSidebar} />
      ) : (
        <ConfigTabs proxyService={proxyService} {...props} />
      )}
    </ScrollArea>
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
  const [activeSection, setActiveSection] = useState<ConfigSectionId>("core");
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const sectionMeta = useMemo(() => findSection(activeSection), [activeSection]);
  const isDashboard = activeSection === "dashboard";
  const toggleSidebar = () => setSidebarOpen((value) => !value);
  const proxyService = toProxyServiceViewProps(props);

  return (
    <Tabs
      data-slot="config-desktop-shell"
      value={activeSection}
      onValueChange={(value) => {
        const nextSection = toConfigSectionId(value);
        if (nextSection) {
          setActiveSection(nextSection);
        }
      }}
      orientation="vertical"
      className="relative z-10 grid h-full min-h-0 grid-cols-[max-content_1fr]"
    >
      <ConfigSidebar
        statusBadge={props.statusBadge}
        activeSection={activeSection}
        sidebarOpen={sidebarOpen}
      />
      <section
        data-slot="config-main"
        className="flex min-h-0 flex-col bg-background/30 backdrop-blur-sm"
      >
        {isDashboard ? null : (
          <ConfigToolbar
            section={sectionMeta}
            status={props.status}
            canSave={props.canSave}
            isDirty={props.isDirty}
            sidebarOpen={sidebarOpen}
            onToggleSidebar={toggleSidebar}
            onReload={props.onReload}
            onSave={props.onSave}
          />
        )}
        <ConfigContent
          {...props}
          isDashboard={isDashboard}
          sidebarOpen={sidebarOpen}
          onToggleSidebar={toggleSidebar}
          proxyService={proxyService}
        />
      </section>
    </Tabs>
  );
}
