import { ToggleLeft, ToggleRight } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PasswordInput } from "@/components/ui/password-input";
import type { ConfigForm } from "@/features/config/types";

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
          <PasswordInput
            id="proxy-key"
            visible={showLocalKey}
            onVisibilityChange={onToggleLocalKey}
            value={form.localApiKey}
            onChange={(event) => onChange({ localApiKey: event.target.value })}
            placeholder="Optional"
          />
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
        <div className="rounded-md border border-border/60 bg-background/60 p-3">
          <div className="flex items-start justify-between gap-4">
            <div className="space-y-1">
              <p className="text-sm font-medium text-foreground">OpenAI Format Conversion</p>
              <p className="text-xs text-muted-foreground">
                Translate between <code>/v1/chat/completions</code> and <code>/v1/responses</code>{" "}
                when the preferred provider is missing.
              </p>
            </div>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              onClick={() =>
                onChange({ enableApiFormatConversion: !form.enableApiFormatConversion })
              }
              aria-label={
                form.enableApiFormatConversion
                  ? "Disable OpenAI format conversion"
                  : "Enable OpenAI format conversion"
              }
              aria-pressed={form.enableApiFormatConversion}
            >
              {form.enableApiFormatConversion ? (
                <ToggleRight className="size-5" aria-hidden="true" />
              ) : (
                <ToggleLeft className="size-5" aria-hidden="true" />
              )}
            </Button>
          </div>
          <p className="mt-2 text-xs text-muted-foreground">Default: disabled.</p>
        </div>
      </CardContent>
    </Card>
  );
}
