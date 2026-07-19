/** 决策最终动作，与后端 `decision-engine` 的 JSON 契约对齐。 */
export type DecisionAction =
  | 'overweight'
  | 'standard'
  | 'tactical_delay'
  | 'underweight'
  | 'skip'

/** A server-side investment plan. Decimal values remain JSON strings. */
export interface InvestmentPlan {
  id: string
  name: string
  symbol: string
  base_contribution: string
  currency: string
  schedule_kind: 'monthly'
  schedule_day: number
  max_single_execution: string
  is_active: boolean
  created_at: string
  updated_at: string
}

/** Payload accepted when creating an investment plan. */
export interface CreateInvestmentPlanRequest {
  name: string
  symbol: string
  base_contribution: string
  currency: string
  schedule_kind: 'monthly'
  schedule_day: number
  max_single_execution: string
}

/** Caller-supplied monthly input for the 70% fundamental calculation. */
export interface FundamentalPreviewRequest {
  cape_history: number[]
  cape_current: number
  erp_history: number[]
  erp_current: number
}

/** Auditable 70% fundamental signal returned by the server. */
export interface FundamentalSignal {
  score: number
  cape_percentile: number
  erp_percentile: number
}

/** Caller-supplied monthly input for the 20% trend calculation. */
export interface TrendPreviewRequest {
  ma_distance_history: number[]
  ma_distance_current: number
  rsi_history: number[]
  rsi_current: number
  vix_history: number[]
  vix_current: number
}

/** Auditable 20% trend signal returned by the server. */
export interface TrendSignal {
  score: number
  ma_distance_percentile: number
  rsi_percentile: number
  vix_percentile: number
  regime: 'neutral' | 'overheated' | 'falling_knife'
}

/** Automatically refreshed, source-labelled inputs for the existing signal APIs. */
export interface MarketSignalInput {
  symbol: string
  as_of: string
  fundamental: FundamentalPreviewRequest
  trend: TrendPreviewRequest
  sources: {
    price: string
    fundamental: string
    volatility: string
  }
}

/** Optional paper-only order submitted from a decision preview. */
export interface PaperOrderRequest {
  idempotency_key: string
  side: 'buy' | 'sell'
  order_type: 'market' | 'limit'
  quantity: string
  limit_price?: string
}

/** Locally persisted performance point reconstructed from paper-account observations. */
export interface PaperPerformancePoint {
  observed_at: string
  adaptive_value: string
  plain_dca_value: string
  net_contributions: string
}

/** Local paper-account return summary and chart series for one investment plan. */
export interface PaperPerformance {
  currency: string
  has_opening_balance: boolean
  data_complete: boolean
  net_contributions: string
  adaptive_value: string
  plain_dca_value: string
  realized_pnl: string
  unrealized_pnl: string
  total_return?: string
  points: PaperPerformancePoint[]
}

/** Request accepted by the composed Decision Preview endpoint. */
export interface DecisionPreviewRequest {
  day_of_month: number
  bucket_allocation: {
    core_ratio: string
    opportunity_ratio: string
  }
  fundamental: FundamentalSignal
  trend: TrendSignal
  paper_order?: PaperOrderRequest
}

/** Execution preview returned as part of a decision. */
export interface ExecutionPreview {
  plan_id: string
  symbol: string
  currency: string
  status: 'due' | 'waiting' | 'inactive'
  planned_contribution?: string
  bucket_split?: {
    planned_contribution: string
    core_contribution: string
    opportunity_contribution: string
  }
}

/** Final weighted decision returned by the server. */
export interface DecisionResult {
  final_score: number
  multiplier: number
  action: DecisionAction
  weight_mode: 'normal' | 'sentiment_unavailable'
  fundamental_score: number
  trend_score: number
  sentiment_score?: number
}

/** Paper-order acknowledgement returned only after a broker accepts a request. */
export interface BrokerOrderAck {
  order_id: string
  environment: 'paper' | 'live'
  status: 'accepted' | 'duplicate'
}

/** Read-only snapshot of the configured OpenD paper account. */
export interface PaperPortfolioSnapshot {
  currency: string
  cash: string
  buying_power: string
  total_assets: string
  market_value: string
  positions: PaperPosition[]
  orders: PaperOrder[]
}

/** One current paper-account position returned by OpenD. */
export interface PaperPosition {
  symbol: string
  name?: string
  quantity: string
  price: string
  cost_price: string
  market_value: string
  unrealized_pnl: string
}

/** One recent normalized paper-order state returned by OpenD. */
export interface PaperOrder {
  order_id: string
  symbol: string
  side: 'buy' | 'sell'
  state: 'pending' | 'partially_filled' | 'filled' | 'closed' | 'unknown'
  quantity: string
  filled_quantity: string
  average_fill_price: string
}

/** Composed Decision Preview response. */
export interface DecisionPreviewResponse {
  execution: ExecutionPreview
  decision: DecisionResult
  paper_order_ack?: BrokerOrderAck
  summary: string
}

/** Persisted decision-record history item. */
export interface DecisionRecord {
  id: string
  plan_id: string
  symbol: string
  currency: string
  execution_status: 'due' | 'waiting' | 'inactive'
  planned_contribution?: string
  execution_snapshot: Record<string, unknown>
  fundamental_snapshot: FundamentalSignal
  trend_snapshot: TrendSignal
  sentiment_snapshot?: { source: string; score: number }
  decision_snapshot: DecisionResult
  broker_order_request?: Record<string, unknown>
  broker_order_ack?: BrokerOrderAck
  summary: string
  created_at: string
}
