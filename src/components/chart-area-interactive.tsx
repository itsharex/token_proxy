"use client"

import * as React from "react"
import { Area, AreaChart, CartesianGrid, XAxis } from "recharts"
import { RefreshCcw } from "lucide-react"

import { useIsMobile } from "@/hooks/use-mobile"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  ToggleGroup,
  ToggleGroupItem,
} from "@/components/ui/toggle-group"
import { formatInteger } from "@/features/dashboard/format"
import type { DashboardSeriesPoint } from "@/features/dashboard/types"
import { cn } from "@/lib/utils"
import { m } from "@/paraglide/messages.js"

export type DashboardTimeRange = "90d" | "30d" | "7d"

type RangeOption = {
  value: DashboardTimeRange
  label: string
  shortLabel: string
}

type ChartPoint = {
  tsMs: number
  inputTokens: number
  outputTokens: number
}

const RANGE_OPTIONS: RangeOption[] = [
  { value: "90d", label: "Last 3 months", shortLabel: "Last 3 months" },
  { value: "30d", label: "Last 30 days", shortLabel: "Last 30 days" },
  { value: "7d", label: "Last 7 days", shortLabel: "Last 7 days" },
]

const RANGE_VALUES = new Set(RANGE_OPTIONS.map((option) => option.value))

const chartConfig = {
  inputTokens: {
    label: "Input",
    color: "var(--chart-1)",
  },
  outputTokens: {
    label: "Output",
    color: "var(--chart-2)",
  },
} satisfies ChartConfig

type ChartAreaInteractiveProps = {
  series: DashboardSeriesPoint[]
  range: DashboardTimeRange
  loading: boolean
  onRangeChange: (range: DashboardTimeRange) => void
  onRefresh: () => void
}

type ChartHeaderProps = {
  range: DashboardTimeRange
  loading: boolean
  onRangeChange: (range: DashboardTimeRange) => void
  onRefresh: () => void
}

type ChartBodyProps = {
  data: ChartPoint[]
  range: DashboardTimeRange
}

function toRangeValue(value: string) {
  return RANGE_VALUES.has(value as DashboardTimeRange)
    ? (value as DashboardTimeRange)
    : null
}

function formatTick(value: number, range: DashboardTimeRange) {
  const date = new Date(value)
  if (range === "7d") {
    return date.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })
  }
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" })
}

function resolveRangeLabel(range: DashboardTimeRange) {
  return RANGE_OPTIONS.find((option) => option.value === range)
}

type RangeSelectorProps = {
  range: DashboardTimeRange
  onRangeChange: (range: DashboardTimeRange) => void
}

function RangeSelector({ range, onRangeChange }: RangeSelectorProps) {
  return (
    <>
      <ToggleGroup
        type="single"
        value={range}
        onValueChange={(value) => {
          const next = toRangeValue(value)
          if (next) {
            onRangeChange(next)
          }
        }}
        variant="outline"
        className="hidden *:data-[slot=toggle-group-item]:!px-4 @[767px]/card:flex"
      >
        {RANGE_OPTIONS.map((option) => (
          <ToggleGroupItem key={option.value} value={option.value}>
            {option.label}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
      <Select
        value={range}
        onValueChange={(value) => {
          const next = toRangeValue(value)
          if (next) {
            onRangeChange(next)
          }
        }}
      >
        <SelectTrigger
          className="flex w-40 **:data-[slot=select-value]:block **:data-[slot=select-value]:truncate @[767px]/card:hidden"
          size="sm"
          aria-label="Select a value"
        >
          <SelectValue placeholder="Last 3 months" />
        </SelectTrigger>
        <SelectContent className="rounded-xl">
          {RANGE_OPTIONS.map((option) => (
            <SelectItem key={option.value} value={option.value} className="rounded-lg">
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </>
  )
}

function ChartHeader({ range, loading, onRangeChange, onRefresh }: ChartHeaderProps) {
  const label = resolveRangeLabel(range)

  return (
    <CardHeader>
      <CardTitle>{m.dashboard_stat_total_tokens()}</CardTitle>
      <CardDescription>
        <span className="hidden @[540px]/card:block">{label?.label}</span>
        <span className="@[540px]/card:hidden">{label?.shortLabel}</span>
      </CardDescription>
      <CardAction className="flex items-center gap-2">
        <RangeSelector range={range} onRangeChange={onRangeChange} />
        <Button type="button" variant="outline" size="icon" onClick={onRefresh} disabled={loading}>
          <RefreshCcw className={cn("size-4", loading && "animate-spin")} />
          <span className="sr-only">{m.common_refresh()}</span>
        </Button>
      </CardAction>
    </CardHeader>
  )
}

function ChartDefs() {
  return (
    <defs>
      <linearGradient id="fillInput" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor="var(--color-inputTokens)" stopOpacity={0.3} />
        <stop offset="95%" stopColor="var(--color-inputTokens)" stopOpacity={0.02} />
      </linearGradient>
      <linearGradient id="fillOutput" x1="0" y1="0" x2="0" y2="1">
        <stop offset="5%" stopColor="var(--color-outputTokens)" stopOpacity={0.28} />
        <stop offset="95%" stopColor="var(--color-outputTokens)" stopOpacity={0.02} />
      </linearGradient>
    </defs>
  )
}

function ChartCanvas({ data, range }: ChartBodyProps) {
  return (
    <ChartContainer config={chartConfig} className="aspect-auto h-[250px] w-full">
      <AreaChart data={data}>
        <ChartDefs />
        <CartesianGrid vertical={false} />
        <XAxis
          dataKey="tsMs"
          tickLine={false}
          axisLine={false}
          tickMargin={8}
          minTickGap={24}
          tickFormatter={(value) => formatTick(Number(value), range)}
        />
        <ChartTooltip
          cursor={false}
          content={
            <ChartTooltipContent
              labelFormatter={(value) => formatTick(Number(value), range)}
              formatter={(value, name) => {
                const label = chartConfig[name as keyof typeof chartConfig]?.label ?? name
                return (
                  <div className="flex min-w-0 items-center gap-2">
                    <span className="text-muted-foreground">{label}</span>
                    <span className="ml-auto font-medium">{formatInteger(Number(value))}</span>
                  </div>
                )
              }}
              indicator="dot"
            />
          }
        />
        <Area
          dataKey="inputTokens"
          type="monotone"
          stroke="var(--color-inputTokens)"
          fill="url(#fillInput)"
          strokeWidth={2}
          dot={false}
        />
        <Area
          dataKey="outputTokens"
          type="monotone"
          stroke="var(--color-outputTokens)"
          fill="url(#fillOutput)"
          strokeWidth={2}
          dot={false}
        />
      </AreaChart>
    </ChartContainer>
  )
}

function ChartBody({ data, range }: ChartBodyProps) {
  return (
    <CardContent className="px-2 pt-4 sm:px-6 sm:pt-6">
      {data.length ? (
        <ChartCanvas data={data} range={range} />
      ) : (
        <div className="flex h-[250px] items-center justify-center text-sm text-muted-foreground">
          {m.dashboard_no_data()}
        </div>
      )}
    </CardContent>
  )
}

export function ChartAreaInteractive({
  series,
  range,
  loading,
  onRangeChange,
  onRefresh,
}: ChartAreaInteractiveProps) {
  const isMobile = useIsMobile()

  React.useEffect(() => {
    if (isMobile && range !== "7d") {
      onRangeChange("7d")
    }
  }, [isMobile, onRangeChange, range])

  const chartData = React.useMemo(
    () =>
      series.map((item) => ({
        tsMs: item.tsMs,
        inputTokens: item.inputTokens,
        outputTokens: item.outputTokens,
      })),
    [series]
  )

  return (
    <Card className="@container/card">
      <ChartHeader
        range={range}
        loading={loading}
        onRangeChange={onRangeChange}
        onRefresh={onRefresh}
      />
      <ChartBody data={chartData} range={range} />
    </Card>
  )
}
