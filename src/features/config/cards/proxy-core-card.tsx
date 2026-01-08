import { Switch } from "@/components/ui/switch";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PasswordInput } from "@/components/ui/password-input";
import type { ConfigForm } from "@/features/config/types";
import { m } from "@/paraglide/messages.js";

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
        <CardTitle>{m.proxy_core_title()}</CardTitle>
        <CardDescription>{m.proxy_core_desc()}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="grid gap-2">
            <Label htmlFor="proxy-host">{m.proxy_core_host_label()}</Label>
            <Input
              id="proxy-host"
              value={form.host}
              onChange={(event) => onChange({ host: event.target.value })}
              placeholder="127.0.0.1"
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="proxy-port">{m.proxy_core_port_label()}</Label>
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
          <Label htmlFor="proxy-key">{m.proxy_core_local_api_key_label()}</Label>
          <PasswordInput
            id="proxy-key"
            visible={showLocalKey}
            onVisibilityChange={onToggleLocalKey}
            value={form.localApiKey}
            onChange={(event) => onChange({ localApiKey: event.target.value })}
            placeholder={m.common_optional()}
          />
          <p className="text-xs text-muted-foreground">{m.proxy_core_local_api_key_help()}</p>
        </div>
        <div className="grid gap-2">
          <Label htmlFor="proxy-log">{m.proxy_core_log_file_label()}</Label>
          <Input
            id="proxy-log"
            value={form.logPath}
            onChange={(event) => onChange({ logPath: event.target.value })}
            placeholder="proxy.log"
          />
        </div>
        <div className="rounded-md border border-border/60 bg-background/60 p-3">
          <div className="flex items-start justify-between gap-4">
            <div className="space-y-1">
              <p className="text-sm font-medium text-foreground">
                {m.proxy_core_format_conversion_title()}
              </p>
              <p className="text-xs text-muted-foreground">
                {m.proxy_core_format_conversion_desc({
                  chat: "/v1/chat/completions",
                  responses: "/v1/responses",
                })}
              </p>
            </div>
            <Switch
              id="enable-format-conversion"
              checked={form.enableApiFormatConversion}
              onCheckedChange={(checked) => onChange({ enableApiFormatConversion: checked })}
              aria-label={m.proxy_core_format_conversion_aria()}
            />
          </div>
          <p className="mt-2 text-xs text-muted-foreground">
            {m.proxy_core_format_conversion_default_disabled()}
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
