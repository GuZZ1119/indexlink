export type StrategyId = 'steady-dca' | 'adaptive-70-20-10' | 'defensive-balance'

export interface ConsumerStrategy {
  id: StrategyId
  name: string
  shortName: string
  summary: string
  fit: string
  cadence: string
  rule: string
  strength: string
  limitation: string
  risk: '稳健' | '平衡' | '进取'
  annualizedReturn: string
  maxDrawdown: string
  sample: string
}

export const consumerStrategies: readonly ConsumerStrategy[] = [
  {
    id: 'steady-dca',
    name: '每月稳步投入',
    shortName: '固定定投',
    summary: '在固定日期，用固定金额持续买入宽基指数。',
    fit: '想先养成长期投资习惯的人',
    cadence: '每月一次',
    rule: '无论市场涨跌，按计划投入。',
    strength: '规则最简单，不需要判断市场。',
    limitation: '市场极端高估时，仍会按原金额买入。',
    risk: '稳健',
    annualizedReturn: '8.1%',
    maxDrawdown: '-33.4%',
    sample: '2014–2024 · 美股宽基示例',
  },
  {
    id: 'adaptive-70-20-10',
    name: '自适应长期计划',
    shortName: '70 / 20 / 10',
    summary: '以历史估值位置为主，趋势和新闻情绪为辅，微调每月投入。',
    fit: '愿意多看一眼原因，但不想盯盘的人',
    cadence: '每月一次',
    rule: '相对低位多投一点，过热时放慢节奏。',
    strength: '在不改变长期方向的前提下，给执行节奏一点弹性。',
    limitation: '它不预测涨跌；极端行情中历史规律可能失效。',
    risk: '平衡',
    annualizedReturn: '8.6%',
    maxDrawdown: '-31.8%',
    sample: '2014–2024 · 美股宽基示例',
  },
  {
    id: 'defensive-balance',
    name: '稳中有进组合',
    shortName: '股债平衡',
    summary: '用股票指数与短债组合降低波动，并在固定日期恢复目标配比。',
    fit: '更在意波动感受与持续持有的人',
    cadence: '每季度检查',
    rule: '维持股票与短债的目标比例，偏离后再平衡。',
    strength: '回撤通常更温和，更容易长期坚持。',
    limitation: '上行很快的股票市场里，可能明显落后于全股票方案。',
    risk: '稳健',
    annualizedReturn: '6.7%',
    maxDrawdown: '-19.6%',
    sample: '2014–2024 · 美股 ETF 示例',
  },
] as const

export function findConsumerStrategy(id: StrategyId | null): ConsumerStrategy {
  return consumerStrategies.find((strategy) => strategy.id === id) ?? consumerStrategies[1]
}

export interface ComparisonRow {
  label: string
  left: string
  right: string
}

/** Build a deliberately small, comparable view without pretending these demo figures are live data. */
export function compareStrategies(leftId: StrategyId, rightId: StrategyId): ComparisonRow[] {
  const left = findConsumerStrategy(leftId)
  const right = findConsumerStrategy(rightId)
  return [
    { label: '适合的人', left: left.fit, right: right.fit },
    { label: '执行节奏', left: left.cadence, right: right.cadence },
    { label: '历史年化', left: left.annualizedReturn, right: right.annualizedReturn },
    { label: '最大回撤', left: left.maxDrawdown, right: right.maxDrawdown },
    { label: '最该知道的限制', left: left.limitation, right: right.limitation },
  ]
}

export type LabConnectionState = 'not-configured' | 'local-only'

export function connectionLabel(state: LabConnectionState): string {
  return state === 'local-only' ? '仅本机可用' : '尚未配置'
}
