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
import { m } from "@/paraglide/messages.js";

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
        <CardTitle>{m.strategy_title()}</CardTitle>
        <CardDescription>{m.strategy_desc()}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid gap-2">
          <Label htmlFor="upstream-strategy">{m.strategy_label()}</Label>
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
              <SelectValue placeholder={m.strategy_placeholder()} />
            </SelectTrigger>
            <SelectContent>
              {options.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label()}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <p className="text-xs text-muted-foreground">
          {m.strategy_help()}
        </p>
      </CardContent>
    </Card>
  );
}
