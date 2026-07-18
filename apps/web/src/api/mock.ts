import type {
  ComparisonPoint,
  LatestDecision,
  MarketOverview,
  NewsItem,
  PortfolioReturns,
  RiskNotice,
} from './types'

export const mockMarketOverview: MarketOverview = {
  symbol: 'SPY',
  name: { zh: '标普 500 指数 ETF', en: 'S&P 500 Index ETF' },
  currency: 'USD',
  compositePercentile: 78,
  suggestedAction: 'underweight',
  suggestedMultiplier: 0.75,
  baseDcaAmount: 2000,
  nextDcaTime: '2026-08-01T09:30:00Z',
  metrics: [
    { key: 'cape', percentile: 86 },
    { key: 'erp', percentile: 72 },
    { key: 'ma200', percentile: 81 },
    { key: 'rsi', percentile: 64 },
    { key: 'vix', percentile: 23 },
  ],
  scores: {
    fundamental: 32,
    trend: 58,
    sentiment: 47,
    composite: 39,
  },
  weights: { fundamental: 70, trend: 20, sentiment: 10 },
}

export const mockLatestDecision: LatestDecision = {
  id: 'dec-20260701-spy',
  symbol: 'SPY',
  action: 'underweight',
  multiplier: 0.75,
  baseAmount: 2000,
  executedAmount: 1500,
  executionPrice: 637.42,
  executionTime: '2026-07-01T10:05:00Z',
  currency: 'USD',
  decidedAt: '2026-07-01T09:30:00Z',
  summary: {
    zh: 'CAPE 与 MA200 距离均处于历史高分位，基本面得分偏低；趋势中性、AI 情绪略偏谨慎，本期按 0.75x 减量执行。',
    en: 'CAPE and MA200 distance sit in high historical percentiles with a weak fundamental score; trend is neutral and AI sentiment slightly cautious, so this cycle executes at 0.75x.',
  },
}

export const mockReturns: PortfolioReturns = {
  currency: 'USD',
  totalReturn: 12840,
  totalReturnPct: 18.6,
  positionReturn: 9420,
  positionReturnPct: 13.7,
  realizedReturn: 3420,
  invested: 69000,
  annualizedPct: 9.8,
  vsDcaPct: 2.4,
}

export const mockRiskNotices: RiskNotice[] = [
  {
    id: 'risk-percentile',
    level: 'warning',
    text: {
      zh: '综合估值分位处于历史高位区间（78%），系统已自动降低本期投入，请勿手动追加超出计划的金额。',
      en: 'Composite valuation percentile is in a historically high range (78%). The system has reduced this cycle; avoid adding beyond plan.',
    },
  },
  {
    id: 'risk-model',
    level: 'info',
    text: {
      zh: '分位只测量价格在历史分布中的位置，不代表对未来价值的判断；极端行情下历史分布可能失效。',
      en: 'Percentiles only measure where price sits in its historical distribution; they are not value judgements and may fail in extreme regimes.',
    },
  },
  {
    id: 'risk-ai',
    level: 'info',
    text: {
      zh: 'AI 情绪信号权重为 10%，当情绪数据不可用时将自动降级为 90/10/0 权重。',
      en: 'AI sentiment carries a 10% weight; when unavailable the engine degrades to 90/10/0 weights automatically.',
    },
  },
]

export const mockNews: NewsItem[] = [
  {
    id: 'n1',
    symbol: 'SPY',
    text: {
      zh: '美联储会议纪要显示官员对下半年降息路径存在分歧',
      en: 'Fed minutes show officials split over H2 rate-cut path',
    },
  },
  {
    id: 'n2',
    symbol: 'SPY',
    text: {
      zh: '标普 500 席勒市盈率升至 34.2，接近 2021 年高点',
      en: 'S&P 500 Shiller P/E climbs to 34.2, nearing 2021 highs',
    },
  },
  {
    id: 'n3',
    symbol: 'VIX',
    text: {
      zh: 'VIX 回落至 13 下方，市场波动率处于年内低位',
      en: 'VIX slips below 13 as market volatility hits YTD lows',
    },
  },
  {
    id: 'n4',
    symbol: 'SPY',
    text: {
      zh: '二季度财报季开启，市场关注科技巨头资本开支指引',
      en: 'Q2 earnings season kicks off with focus on big-tech capex guidance',
    },
  },
]

/** 以确定性伪随机生成 5 年月度对比曲线，保证两次渲染结果一致。 */
function buildComparisonSeries(): ComparisonPoint[] {
  const points: ComparisonPoint[] = []
  const months = 60
  const start = new Date(Date.UTC(2021, 6, 1))
  let dca = 0
  let adaptive = 0
  let seed = 42

  const next = () => {
    seed = (seed * 1103515245 + 12345) % 2147483648
    return seed / 2147483648
  }

  for (let i = 0; i < months; i += 1) {
    const drift = 0.55
    const shock = (next() - 0.45) * 4
    dca += drift + shock
    // 自适应定投在下跌段加码、高位段减量，长期略跑赢
    adaptive += drift + shock * 0.9 + (shock < 0 ? 0.18 : 0.06)

    const d = new Date(start)
    d.setUTCMonth(start.getUTCMonth() + i)
    points.push({
      date: `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, '0')}`,
      dca: Number(dca.toFixed(2)),
      adaptive: Number(adaptive.toFixed(2)),
    })
  }
  return points
}

export const mockComparisonSeries: ComparisonPoint[] = buildComparisonSeries()
