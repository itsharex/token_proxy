import { useMemo, useState } from "react";

import type { LucideIcon } from "lucide-react";
import {
  AlertCircle,
  CircleCheck,
  FileJson,
  Loader2,
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
import {
  ConfigFileCard,
  ProxyCoreCard,
  StrategyCard,
  UpstreamsCard,
  ValidationCard,
  type StatusBadge,
} from "@/features/config/cards";
import type { ConfigForm } from "@/features/config/types";
import { cn } from "@/lib/utils";

type ConfigSectionId = "core" | "strategy" | "upstreams" | "file" | "validation";

type ConfigSection = {
  id: ConfigSectionId;
  label: string;
  description: string;
  icon: LucideIcon;
};

const CONFIG_SECTIONS: readonly ConfigSection[] = [
  {
    id: "core",
    label: "Proxy Core",
    description: "Listening address, auth, and logs",
    icon: SlidersHorizontal,
  },
  {
    id: "strategy",
    label: "Strategy",
    description: "Global upstream selection rules",
    icon: Shuffle,
  },
  {
    id: "upstreams",
    label: "Upstreams",
    description: "Provider pools and API keys",
    icon: Server,
  },
  {
    id: "file",
    label: "Config File",
    description: "JSONC on disk and reset tools",
    icon: FileJson,
  },
  {
    id: "validation",
    label: "Validation",
    description: "Quick readiness checks",
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
    statusBadge.label === "Working" && "animate-pulse"
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

function ConfigSidebar({ statusBadge, activeSection }: ConfigSidebarProps) {
  return (
    <aside
      data-slot="config-sidebar"
      className="flex min-h-0 flex-col border-r border-border/60 bg-background/60 backdrop-blur"
    >
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
                <span className="min-w-0 truncate text-sm font-medium">{section.label}</span>
              </TabsTrigger>
            );
          })}
        </TabsList>
      </ScrollArea>
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
};

type ConfigToolbarProps = {
  section: ConfigSection;
  status: AppViewProps["status"];
  canSave: boolean;
  isDirty: boolean;
  onReload: () => void;
  onSave: () => void;
};

function ConfigToolbar({ section, status, canSave, isDirty, onReload, onSave }: ConfigToolbarProps) {
  const isLoading = status === "loading";
  const isSaving = status === "saving";
  const canReload = !isLoading && !isDirty;

  return (
    <header
      data-slot="config-toolbar"
      className="flex items-center justify-between gap-3 border-b border-border/60 bg-background/70 px-6 py-4 backdrop-blur"
    >
      <div className="min-w-0">
        <p className="truncate text-sm font-medium text-foreground">{section.label}</p>
        <p className="truncate text-xs text-muted-foreground">{section.description}</p>
      </div>
      <div className="flex items-center gap-2">
        <Button type="button" variant="outline" size="icon" onClick={onReload} disabled={!canReload}>
          <RefreshCw className={isLoading ? "animate-spin" : undefined} aria-hidden="true" />
        </Button>
        <Button type="button" onClick={onSave} disabled={!canSave}>
          {isSaving ? <Loader2 className="animate-spin" aria-hidden="true" /> : "Save"}
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
        <AlertTitle>Request failed</AlertTitle>
        <AlertDescription>{statusMessage}</AlertDescription>
      </div>
    </Alert>
  );
}

export function AppView(props: AppViewProps) {
  const [activeSection, setActiveSection] = useState<ConfigSectionId>("core");
  const sectionMeta = useMemo(() => findSection(activeSection), [activeSection]);

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
      <ConfigSidebar statusBadge={props.statusBadge} activeSection={activeSection} />
      <section
        data-slot="config-main"
        className="flex min-h-0 flex-col bg-background/30 backdrop-blur-sm"
      >
        <ConfigToolbar
          section={sectionMeta}
          status={props.status}
          canSave={props.canSave}
          isDirty={props.isDirty}
          onReload={props.onReload}
          onSave={props.onSave}
        />
        <ScrollArea data-slot="config-main-scroll" className="min-h-0 flex-1">
          <div className="p-6">
            <StatusAlert statusMessage={props.statusMessage} />
            <TabsContent value="core" className="mt-0">
              <ProxyCoreCard
                form={props.form}
                showLocalKey={props.showLocalKey}
                onToggleLocalKey={props.onToggleLocalKey}
                onChange={props.onFormChange}
              />
            </TabsContent>
            <TabsContent value="strategy" className="mt-0">
              <StrategyCard strategy={props.form.upstreamStrategy} onChange={props.onStrategyChange} />
            </TabsContent>
            <TabsContent value="upstreams" className="mt-0">
              <UpstreamsCard
                upstreams={props.form.upstreams}
                showApiKeys={props.showUpstreamKeys}
                providerOptions={props.providerOptions}
                onToggleApiKeys={props.onToggleUpstreamKeys}
                onAdd={props.onAddUpstream}
                onRemove={props.onRemoveUpstream}
                onChange={props.onChangeUpstream}
              />
            </TabsContent>
            <TabsContent value="file" className="mt-0">
              <ConfigFileCard
                configPath={props.configPath}
                savedAt={props.savedAt}
                isDirty={props.isDirty}
                onReset={props.onReset}
              />
            </TabsContent>
            <TabsContent value="validation" className="mt-0">
              <ValidationCard form={props.form} validation={props.validation} />
            </TabsContent>
          </div>
        </ScrollArea>
      </section>
    </Tabs>
  );
}
