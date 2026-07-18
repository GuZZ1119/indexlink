/** 决策最终动作，与后端 decision-engine 的 Action 对齐。 */
export type DecisionAction =
  | 'overweight'
  | 'standard'
  | 'delay'
  | 'underweight'
  | 'skip'

export interface ValuationMetric {
  /** i18n key 后缀：cape / erp / ma200 / rsi / vix */
  key: 'cape' | 'erp' | 'ma200' | 'rsi' | 'vix'
  /** 历史分位，0-100 */
  percentile: number
}

export interface ScoreBreakdown {
  /** 0-100 */
  fundamental: number
  trend: number
  sentiment: number
  composite: number
}

export interface LatestDecision {
  id: string
  symbol: string
  action: DecisionAction
  multiplier: number
  baseAmount: number
  executedAmount: number
  executionPrice: number
  executionTime: string
  currency: string
  decidedAt: string
  summary: { zh: string; en: string }
}

export interface MarketOverview {
  symbol: string
  name: { zh: string; en: string }
  currency: string
  compositePercentile: number
  suggestedAction: DecisionAction
  suggestedMultiplier: number
  baseDcaAmount: number
  nextDcaTime: string
  metrics: ValuationMetric[]
  scores: ScoreBreakdown
  /** 70/20/10 权重 */
  weights: { fundamental: number; trend: number; sentiment: number }
}

export interface PortfolioReturns {
  currency: string
  totalReturn: number
  totalReturnPct: number
  positionReturn: number
  positionReturnPct: number
  realizedReturn: number
  invested: number
  annualizedPct: number
  vsDcaPct: number
}

export interface ComparisonPoint {
  /** ISO 月份，例如 2024-07 */
  date: string
  /** 累计收益率，% */
  dca: number
  adaptive: number
}

export interface RiskNotice {
  id: string
  level: 'info' | 'warning'
  text: { zh: string; en: string }
}

export interface NewsItem {
  id: string
  symbol: string
  text: { zh: string; en: string }
}
