import { useMemo } from "react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { type ConfigForm, UPSTREAM_STRATEGIES } from "@/features/config/types";

const UPSTREAM_STRATEGY_VALUES: ReadonlySet<string> = new Set(
  UPSTREAM_STRATEGIES.map((strategy) => strategy.value)
);

function toUpstreamStrategy(value: string): ConfigForm["upstreamStrategy"] | null {
  return UPSTREAM_STRATEGY_VALUES.has(value) ? (value as ConfigForm["upstreamStrategy"]) : null;
}

type StrategyCardProps = {
  strategy: ConfigForm["upstreamStrategy"];
  onChange: (value: ConfigForm["upstreamStrategy"]) => void;
};

export function StrategyCard({ strategy, onChange }: StrategyCardProps) {
  const options = useMemo(() => UPSTREAM_STRATEGIES, []);
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
              {options.map((option) => (
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
