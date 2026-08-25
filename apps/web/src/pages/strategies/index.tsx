import { useMemo, useState, type FormEvent } from 'react'
import { CheckCircle2, Copy, Play, Plus, Save, ShieldCheck, Trash2 } from 'lucide-react'

import { useActivatePlanPolicy, useCreateStrategy, usePlans, useStrategies, useValidateStrategy } from '@/api/queries'
import type { StrategyComparisonDocument, StrategyConditionDocument, StrategyIndicatorDocument, StrategyRuleDocument, StrategySpecDocument } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'

const indicator = (): StrategyIndicatorDocument => ({ kind: 'relative_strength_index', lookback_days: 14 })
const comparison = (): StrategyComparisonDocument => ({ kind: 'comparison', expression: { kind: 'indicator', indicator: indicator() }, operator: 'less_than', threshold: '35' })
const rule = (): StrategyRuleDocument => ({ condition: comparison(), action: { kind: 'set_opportunity_multiplier', multiplier: 1 } })
const newDocument = (): StrategySpecDocument => ({ policy_id: 'dsl_risk_guard', policy_version: 1, name: 'Opportunity risk guard', rules: [rule()] })

/** Form-only Strategy Studio; it never accepts arbitrary source code. */
export default function StrategiesPage() {
  const strategies = useStrategies()
  const { data: plans = [] } = usePlans()
  const validate = useValidateStrategy()
  const create = useCreateStrategy()
  const activate = useActivatePlanPolicy()
  const [document, setDocument] = useState(newDocument)
  const [selected, setSelected] = useState<string | null>(null)
  const [simulation, setSimulation] = useState<{ as_of: string; matched_rule_index: number | null; action: string; multiplier: number; evidence: Array<{ indicator: string; value: string }> } | null>(null)
  const [simulationError, setSimulationError] = useState<string | null>(null)
  const selectedStrategy = useMemo(() => strategies.data?.find((item) => `${item.policy.id}@${item.policy.version}` === selected) ?? strategies.data?.[0], [selected, strategies.data])
  const error = validate.data?.valid === false ? validate.data.error : create.error ?? activate.error

  const updateRule = (index: number, next: StrategyRuleDocument) => setDocument((current) => ({ ...current, rules: current.rules.map((item, itemIndex) => itemIndex === index ? next : item) }))
  const submit = async (event: FormEvent) => {
    event.preventDefault()
    const result = await validate.mutateAsync(document)
    if (!result.valid || !result.document) return
    const saved = await create.mutateAsync(result.document)
    setSelected(`${saved.policy.id}@${saved.policy.version}`)
  }
  const duplicate = () => {
    if (!selectedStrategy) return
    setDocument({ ...selectedStrategy.document, policy_id: `${selectedStrategy.policy.id}_v${selectedStrategy.policy.version + 1}`, policy_version: selectedStrategy.policy.version + 1, name: `${selectedStrategy.name} copy` })
  }
  const simulate = async (symbol: string) => {
    if (!selectedStrategy) return
    setSimulationError(null)
    try {
      const response = await fetch(`${import.meta.env.VITE_API_BASE_URL ?? ''}/strategies/${selectedStrategy.policy.id}/${selectedStrategy.policy.version}/simulate`, { method: 'POST', headers: { 'Content-Type': 'application/json', Accept: 'application/json' }, body: JSON.stringify({ symbol }) })
      const body = await response.json()
      if (!response.ok) throw new Error(body?.error?.message ?? 'simulation failed')
      setSimulation(body)
    } catch (reason) { setSimulationError(reason instanceof Error ? reason.message : 'simulation failed') }
  }

  return <div className="mx-auto grid w-full max-w-7xl gap-4 p-4 lg:grid-cols-[18rem_minmax(0,1fr)_22rem] lg:p-6">
    <Card><CardHeader><CardTitle>策略版本</CardTitle><CardDescription>不可变版本；复制后创建新版本。</CardDescription></CardHeader><CardContent className="space-y-2">{strategies.data?.map((item) => <button key={`${item.policy.id}@${item.policy.version}`} type="button" onClick={() => setSelected(`${item.policy.id}@${item.policy.version}`)} className="w-full rounded-lg border p-3 text-left hover:bg-muted/50"><strong>{item.name}</strong><span className="mt-1 block font-mono text-xs text-muted-foreground">{item.policy.id}@{item.policy.version}</span></button>)}<Button className="w-full" variant="outline" onClick={duplicate} disabled={!selectedStrategy}><Copy className="mr-2 size-4" />复制选中版本</Button></CardContent></Card>
    <div className="space-y-4"><Card className="border-sky-200 bg-sky-50/40"><CardHeader><CardTitle className="flex gap-2"><ShieldCheck className="size-5 text-sky-700" />受限策略 Studio</CardTitle><CardDescription>白名单指标、条件组与机会桶动作。核心桶始终保留。</CardDescription></CardHeader><CardContent><form className="space-y-4" onSubmit={(event) => void submit(event)}><div className="grid gap-3 sm:grid-cols-3"><Field label="策略 ID" value={document.policy_id} onChange={(value) => setDocument((current) => ({ ...current, policy_id: value }))} /><Field label="版本" type="number" value={String(document.policy_version)} onChange={(value) => setDocument((current) => ({ ...current, policy_version: Number(value) }))} /><Field label="名称" value={document.name} onChange={(value) => setDocument((current) => ({ ...current, name: value }))} /></div>{document.rules.map((item, index) => <RuleEditor key={index} index={index} rule={item} onChange={(next) => updateRule(index, next)} onRemove={() => setDocument((current) => ({ ...current, rules: current.rules.filter((_, itemIndex) => itemIndex !== index) }))} removable={document.rules.length > 1} />)}<Button type="button" variant="outline" onClick={() => setDocument((current) => ({ ...current, rules: [...current.rules, rule()] }))}><Plus className="mr-2 size-4" />增加下一优先规则</Button>{error && <p className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">{error instanceof Error ? error.message : error}</p>}{validate.data?.valid && <p className="flex gap-2 text-sm text-emerald-700"><CheckCircle2 className="size-4" />校验通过，可保存。</p>}<Button className="w-full" type="submit" disabled={validate.isPending || create.isPending}><Save className="mr-2 size-4" />验证并保存不可变版本</Button></form></CardContent></Card>{selectedStrategy && <Card><CardHeader><CardTitle>当前数据模拟</CardTitle><CardDescription>仅解释首条命中规则；不写审计、不下单。</CardDescription></CardHeader><CardContent className="space-y-3">{plans.map((plan) => <Button key={plan.id} variant="outline" className="mr-2" onClick={() => void simulate(plan.symbol)}><Play className="mr-2 size-4" />用 {plan.symbol} 模拟</Button>)}{simulationError && <p className="text-sm text-destructive">{simulationError}</p>}{simulation && <div className="rounded-lg border bg-muted/40 p-3 text-sm"><p>截至 {simulation.as_of}：{simulation.matched_rule_index === null ? '未命中规则，使用默认机会桶行为。' : `命中优先级第 ${simulation.matched_rule_index + 1} 条规则。`}</p><p className="mt-1">动作：{simulation.action}，机会桶倍率：{simulation.multiplier.toFixed(2)}x</p><p className="mt-2 text-xs text-muted-foreground">证据：{simulation.evidence.map((value) => `${value.indicator}=${value.value}`).join('；')}</p></div>}</CardContent></Card>}</div>
    <Card><CardHeader><CardTitle>激活到计划</CardTitle><CardDescription>确认后 Preview、scheduler 与审计都运行该版本；审批模式不自动下单。</CardDescription></CardHeader><CardContent className="space-y-3">{selectedStrategy && plans.map((plan) => <div key={plan.id} className="rounded-lg border p-3"><p className="font-medium">{plan.name} · {plan.symbol}</p><p className="text-xs text-muted-foreground">当前 {plan.policy.id}@{plan.policy.version}</p><Button className="mt-3 w-full" size="sm" onClick={() => { if (globalThis.confirm(`绑定 ${selectedStrategy.name} 到 ${plan.name}？`)) activate.mutate({ planId: plan.id, policy: selectedStrategy.policy }) }}>确认激活</Button></div>)}</CardContent></Card>
  </div>
}

function Field({ label, value, onChange, type }: { label: string; value: string; onChange: (value: string) => void; type?: string }) { return <label className="grid gap-1 text-sm font-medium">{label}<Input type={type} value={value} onChange={(event) => onChange(event.target.value)} /></label> }
function RuleEditor({ index, rule, onChange, onRemove, removable }: { index: number; rule: StrategyRuleDocument; onChange: (value: StrategyRuleDocument) => void; onRemove: () => void; removable: boolean }) {
  const comparisons = rule.condition.kind === 'comparison' ? [rule.condition] : rule.condition.conditions
  const group = rule.condition.kind === 'comparison' ? 'single' : rule.condition.kind
  const setCondition = (conditions: StrategyComparisonDocument[], nextGroup = group) => onChange({ ...rule, condition: nextGroup === 'single' ? conditions[0] : { kind: nextGroup, conditions } as StrategyConditionDocument })
  return <section className="space-y-3 rounded-lg border bg-background p-3"><div className="flex items-center justify-between"><strong className="text-sm">优先规则 {index + 1}</strong>{removable && <Button type="button" variant="ghost" size="icon" onClick={onRemove}><Trash2 className="size-4" /></Button>}</div><label className="grid max-w-48 gap-1 text-sm">条件关系<select className="h-9 rounded-md border px-2" value={group} onChange={(event) => setCondition(comparisons, event.target.value)}><option value="single">单条件</option><option value="all">全部满足</option><option value="any">任一满足</option></select></label>{comparisons.map((condition, conditionIndex) => <ComparisonEditor key={conditionIndex} condition={condition} onChange={(next) => { const updated = comparisons.map((item, itemIndex) => itemIndex === conditionIndex ? next : item); setCondition(updated) }} />)}{group !== 'single' && <Button type="button" size="sm" variant="outline" onClick={() => setCondition([...comparisons, comparison()])}><Plus className="mr-1 size-3" />增加条件</Button>}<label className="grid gap-1 text-sm">机会桶动作<select className="h-9 rounded-md border px-2" value={rule.action.kind} onChange={(event) => onChange({ ...rule, action: event.target.value === 'skip_opportunity' ? { kind: 'skip_opportunity' } : { kind: 'set_opportunity_multiplier', multiplier: 1 } })}><option value="set_opportunity_multiplier">设置倍率</option><option value="skip_opportunity">跳过机会桶</option></select></label>{rule.action.kind === 'set_opportunity_multiplier' && <Field label="倍率（0–1.5）" type="number" value={String(rule.action.multiplier)} onChange={(value) => onChange({ ...rule, action: { kind: 'set_opportunity_multiplier', multiplier: Number(value) } })} />}</section>
}
function ComparisonEditor({ condition, onChange }: { condition: StrategyComparisonDocument; onChange: (value: StrategyComparisonDocument) => void }) { const indicator = condition.expression.indicator; return <div className="grid gap-2 sm:grid-cols-3"><label className="grid gap-1 text-sm">指标<select className="h-9 rounded-md border px-2" value={indicator.kind} onChange={(event) => { const kind = event.target.value as StrategyIndicatorDocument['kind']; const next: StrategyIndicatorDocument = kind === 'vix' || kind === 'close_price' ? { kind } : { kind, lookback_days: kind === 'relative_strength_index' ? 14 : kind === 'drawdown' ? 90 : 200 }; onChange({ ...condition, expression: { kind: 'indicator', indicator: next } }) }}><option value="close_price">收盘价</option><option value="simple_moving_average">SMA</option><option value="exponential_moving_average">EMA</option><option value="relative_strength_index">RSI</option><option value="drawdown">回撤</option><option value="vix">VIX</option></select></label>{'lookback_days' in indicator && <Field label="窗口（交易日）" type="number" value={String(indicator.lookback_days)} onChange={(value) => onChange({ ...condition, expression: { kind: 'indicator', indicator: { ...indicator, lookback_days: Number(value) } } })} />}<label className="grid gap-1 text-sm">比较<select className="h-9 rounded-md border px-2" value={condition.operator} onChange={(event) => onChange({ ...condition, operator: event.target.value as StrategyComparisonDocument['operator'] })}><option value="less_than">小于</option><option value="less_than_or_equal">小于等于</option><option value="greater_than">大于</option><option value="greater_than_or_equal">大于等于</option></select></label><Field label="阈值" value={condition.threshold} onChange={(value) => onChange({ ...condition, threshold: value })} /></div> }
