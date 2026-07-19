import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import type {
  CreateInvestmentPlanRequest,
  DecisionPreviewRequest,
  DecisionPreviewResponse,
  DecisionRecord,
  FundamentalPreviewRequest,
  FundamentalSignal,
  InvestmentPlan,
  MarketSignalInput,
  PaperPortfolioSnapshot,
  PaperPerformance,
  ActualPerformance,
  HistoricalBacktest,
  HoldingPriceHistory,
  TrendPreviewRequest,
  TrendSignal,
} from './types'

const apiBaseUrl = (import.meta.env.VITE_API_BASE_URL ?? '').replace(/\/$/, '')

/** Error returned to the UI without exposing transport or provider internals. */
export class ApiRequestError extends Error {}

interface ErrorEnvelope {
  error?: { message?: string }
}

/** Call one same-origin or configured Rust HTTP endpoint. */
async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${apiBaseUrl}${path}`, {
    ...init,
    headers: {
      Accept: 'application/json',
      ...(init?.body ? { 'Content-Type': 'application/json' } : {}),
      ...init?.headers,
    },
  })
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as ErrorEnvelope | null
    throw new ApiRequestError(body?.error?.message ?? 'request failed')
  }
  if (response.status === 204) {
    return undefined as T
  }
  return (await response.json()) as T
}

/** List normalized investment plans from the Rust API. */
export function fetchPlans(): Promise<InvestmentPlan[]> {
  return request('/investment-plans')
}

/** Create one normalized investment plan through the Rust API. */
export function createPlan(input: CreateInvestmentPlanRequest): Promise<InvestmentPlan> {
  return request('/investment-plans', { method: 'POST', body: JSON.stringify(input) })
}

/** Delete one recurring holding and its local-only dependent records. */
export async function deletePlan(planId: string): Promise<void> {
  await request(`/investment-plans/${encodeURIComponent(planId)}`, { method: 'DELETE' })
}

/** Calculate a 70% fundamental signal from caller-provided historical data. */
export function previewFundamental(input: FundamentalPreviewRequest): Promise<FundamentalSignal> {
  return request('/signals/fundamental/preview', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

/** Calculate a 20% trend signal from caller-provided historical data. */
export function previewTrend(input: TrendPreviewRequest): Promise<TrendSignal> {
  return request('/signals/trend/preview', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

/** Read one automatic, source-labelled signal snapshot from the local Rust API. */
export function fetchMarketSignalInput(symbol: string): Promise<MarketSignalInput> {
  return request(`/signals/market-input/${encodeURIComponent(symbol)}`)
}

/** Read funds, positions, and recent orders from the configured local paper account. */
export function fetchPaperPortfolio(): Promise<PaperPortfolioSnapshot> {
  return request('/paper-portfolio')
}

/** Refresh one plan's local paper ledger from read-only OpenD account data. */
export function fetchPaperPerformance(planId: string): Promise<PaperPerformance> {
  return request(`/investment-plans/${encodeURIComponent(planId)}/paper-performance`)
}

/** Refresh and read every active holding's real local-paper trajectory. */
export function fetchActualPerformance(): Promise<ActualPerformance> {
  return request('/paper-performance/actual')
}

/** Read one transparent year of price-only historical plain-versus-adaptive replay. */
export function fetchHistoricalBacktest(): Promise<HistoricalBacktest> {
  return request('/paper-performance/historical-backtest')
}

/** Read actual OpenD price lines plus local paper buy/sell markers for every active holding. */
export function fetchHoldingPriceHistory(period: '3m' | '6m' | '1y' | '3y'): Promise<HoldingPriceHistory[]> {
  return request(`/market-data/holdings?period=${period}`)
}

/** Store a user-confirmed local opening balance used only for return calculations. */
export function setPaperOpeningBalance(
  planId: string,
  input: { amount: string; occurred_at: string },
): Promise<void> {
  return request(`/investment-plans/${encodeURIComponent(planId)}/paper-performance/opening-balance`, {
    method: 'PUT',
    body: JSON.stringify(input),
  })
}

/** Compose a decision, persist its audit record, and optionally submit a paper order. */
export function previewDecision(
  planId: string,
  input: DecisionPreviewRequest,
): Promise<DecisionPreviewResponse> {
  return request(`/investment-plans/${planId}/decision-preview`, {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

/** List persisted decision records for one selected plan. */
export function fetchDecisionRecords(planId: string): Promise<DecisionRecord[]> {
  return request(`/investment-plans/${planId}/decisions?limit=50`)
}

/** Fetch one decision record for a detail route. */
export function fetchDecisionRecord(id: string): Promise<DecisionRecord> {
  return request(`/decisions/${id}`)
}

/** React Query hook for live plan data. */
export function usePlans() {
  return useQuery({ queryKey: ['plans'], queryFn: fetchPlans })
}

/** React Query mutation that refreshes the plan list after creation. */
export function useCreatePlan() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: createPlan,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['plans'] })
    },
  })
}

/** Delete a recurring holding and invalidate every plan-backed view. */
export function useDeletePlan() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: deletePlan,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['plans'] })
    },
  })
}

/** React Query hook for the selected plan's decision history. */
export function useDecisionRecords(planId: string | null) {
  return useQuery({
    queryKey: ['decision-records', planId],
    queryFn: () => fetchDecisionRecords(planId!),
    enabled: planId !== null,
  })
}

/** React Query hook for a single decision-record detail. */
export function useDecisionRecord(id: string | null) {
  return useQuery({
    queryKey: ['decision-record', id],
    queryFn: () => fetchDecisionRecord(id!),
    enabled: id !== null,
  })
}
