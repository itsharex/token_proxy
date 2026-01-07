import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ConfigForm } from "@/features/config/types";

type ValidationCardProps = {
  form: ConfigForm;
  validation: { valid: boolean; message: string };
};

export function ValidationCard({ form, validation }: ValidationCardProps) {
  const hasUpstreams = form.upstreams.length > 0;
  const hasEnabledUpstreams = form.upstreams.some((upstream) => upstream.enabled);
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
          <Badge
            variant={!hasUpstreams ? "destructive" : hasEnabledUpstreams ? "default" : "secondary"}
          >
            {!hasUpstreams ? "Missing" : hasEnabledUpstreams ? "Ready" : "Disabled"}
          </Badge>
        </div>
      </CardContent>
    </Card>
  );
}
