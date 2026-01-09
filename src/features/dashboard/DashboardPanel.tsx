import { useCallback, useEffect, useMemo, useState } from "react"
import { AlertCircle } from "lucide-react"

import { ChartAreaInteractive, type DashboardTimeRange } from "@/components/chart-area-interactive"
import { DataTable } from "@/components/data-table"
import { SectionCards } from "@/components/section-cards"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { readDashboardSnapshot } from "@/features/dashboard/api"
import type { DashboardSnapshot } from "@/features/dashboard/types"
import { parseError } from "@/lib/error"
import { m } from "@/paraglide/messages.js"

const RECENT_PAGE_SIZE = 50

type DashboardStatus = "idle" | "loading" | "error"

function resolveRange(preset: DashboardTimeRange) {
  const now = Date.now()
  const days = preset === "7d" ? 7 : preset === "30d" ? 30 : 90
  return { fromTsMs: now - days * 24 * 60 * 60 * 1000, toTsMs: now }
}

function derivePagination(snapshot: DashboardSnapshot | null, page: number) {
  const totalRequests = snapshot?.summary.totalRequests ?? 0
  const totalPages = Math.max(1, Math.ceil(totalRequests / RECENT_PAGE_SIZE))
  const safePage = Math.min(Math.max(1, page), totalPages)
  return { totalRequests, totalPages, page: safePage }
}

function useDashboardSnapshot() {
  const [rangePreset, setRangePreset] = useState<DashboardTimeRange>("30d")
  const [page, setPage] = useState(1)
  const [snapshot, setSnapshot] = useState<DashboardSnapshot | null>(null)
  const [status, setStatus] = useState<DashboardStatus>("idle")
  const [statusMessage, setStatusMessage] = useState("")

  const loadSnapshot = useCallback(async () => {
    setStatus("loading")
    setStatusMessage("")
    try {
      const range = resolveRange(rangePreset)
      const offset = (page - 1) * RECENT_PAGE_SIZE
      const data = await readDashboardSnapshot(range, offset)
      setSnapshot(data)
      setStatus("idle")
    } catch (error) {
      setStatus("error")
      setStatusMessage(parseError(error))
    }
  }, [page, rangePreset])

  useEffect(() => {
    void loadSnapshot()
  }, [loadSnapshot])

  const pagination = useMemo(() => derivePagination(snapshot, page), [snapshot, page])

  useEffect(() => {
    if (pagination.page !== page) {
      setPage(pagination.page)
    }
  }, [page, pagination.page])

  const handleRangeChange = useCallback((next: DashboardTimeRange) => {
    setRangePreset(next)
    setPage(1)
  }, [])

  const handlePrevPage = useCallback(() => {
    setPage((current) => Math.max(1, current - 1))
  }, [])

  const handleNextPage = useCallback(() => {
    setPage((current) => Math.min(pagination.totalPages, current + 1))
  }, [pagination.totalPages])

  return {
    snapshot,
    status,
    statusMessage,
    rangePreset,
    pagination,
    refresh: loadSnapshot,
    onRangeChange: handleRangeChange,
    onPrevPage: handlePrevPage,
    onNextPage: handleNextPage,
  }
}

export function DashboardPanel() {
  const {
    snapshot,
    status,
    statusMessage,
    rangePreset,
    pagination,
    refresh,
    onRangeChange,
    onPrevPage,
    onNextPage,
  } = useDashboardSnapshot()

  const isLoading = status === "loading"

  return (
    <div className="flex flex-col gap-4">
      {status === "error" ? (
        <Alert variant="destructive" className="mx-4 lg:mx-6">
          <AlertCircle className="size-4" aria-hidden="true" />
          <div>
            <AlertTitle>{m.dashboard_load_failed()}</AlertTitle>
            <AlertDescription>{statusMessage}</AlertDescription>
          </div>
        </Alert>
      ) : null}

      <SectionCards summary={snapshot?.summary ?? null} />

      <div className="px-4 lg:px-6">
        <ChartAreaInteractive
          series={snapshot?.series ?? []}
          range={rangePreset}
          loading={isLoading}
          onRangeChange={onRangeChange}
          onRefresh={refresh}
        />
      </div>

      <DataTable
        items={snapshot?.recent ?? []}
        page={pagination.page}
        totalPages={pagination.totalPages}
        totalRequests={pagination.totalRequests}
        pageSize={RECENT_PAGE_SIZE}
        loading={isLoading}
        scrollKey={rangePreset + "-" + pagination.page}
        onPrevPage={onPrevPage}
        onNextPage={onNextPage}
      />
    </div>
  )
}
