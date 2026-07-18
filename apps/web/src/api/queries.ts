import { useQuery } from '@tanstack/react-query'

import {
  mockComparisonSeries,
  mockLatestDecision,
  mockMarketOverview,
  mockNews,
  mockReturns,
  mockRiskNotices,
} from './mock'

/** MVP 阶段以固定延迟模拟 Rust API 响应，后续替换为真实 fetch。 */
function delayed<T>(data: T, ms = 300): () => Promise<T> {
  return () => new Promise((resolve) => setTimeout(() => resolve(data), ms))
}

export function useMarketOverview() {
  return useQuery({
    queryKey: ['market-overview'],
    queryFn: delayed(mockMarketOverview),
  })
}

export function useLatestDecision() {
  return useQuery({
    queryKey: ['latest-decision'],
    queryFn: delayed(mockLatestDecision),
  })
}

export function usePortfolioReturns() {
  return useQuery({
    queryKey: ['portfolio-returns'],
    queryFn: delayed(mockReturns),
  })
}

export function useComparisonSeries() {
  return useQuery({
    queryKey: ['comparison-series'],
    queryFn: delayed(mockComparisonSeries),
  })
}

export function useRiskNotices() {
  return useQuery({
    queryKey: ['risk-notices'],
    queryFn: delayed(mockRiskNotices),
  })
}

export function useNews() {
  return useQuery({
    queryKey: ['news'],
    queryFn: delayed(mockNews, 150),
  })
}
