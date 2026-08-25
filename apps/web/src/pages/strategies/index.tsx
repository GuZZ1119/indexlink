import { useMemo, useState, type FormEvent } from 'react'
import { CheckCircle2, Code2, Save, ShieldCheck } from 'lucide-react'

import {
  useActivatePlanPolicy,
  useCreateStrategy,
  usePlans,
  useStrategies,
  useValidateStrategy,
} from '@/api/queries'
import type { StrategySpecDocument } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'

const newDocument = (): StrategySpecDocument => ({
  policy_id: 'dsl_rsi_guard',
  policy_version: 1,
  name: 'RSI opportunity guard',
  rules: [{
    condition: {
      kind: 'comparison',
      expression: { kind: 'indicator', indicator: { kind: 'relative_strength_index', lookback_days: 14 } },
      operator: 'less_than',
      threshold: '35',
    },
    action: { kind: 'set_opportunity_multiplier', multiplier: 1.1 },
  }],
})

/** Create, inspect, validate, and explicitly activate restricted opportunity-bucket strategies. */
export default function StrategiesPage() {
  const strategies = useStrategies()
  const { data: plans = [] } = usePlans()
  const validate = useValidateStrategy()
  const create = useCreateStrategy()
  const activate = useActivatePlanPolicy()
  const [document, setDocument] = useState(newDocument)
  const [selected, setSelected] = useState<string | null>(null)
  const selectedStrategy = useMemo(
    () => strategies.data?.find((strategy) => `${strategy.policy.id}@${strategy.policy.version}` === selected) ?? strategies.data?.[0],
    [selected, strategies.data],
  )
  const error = validate.data?.valid === false ? validate.data.error : create.error ?? activate.error

  const updateRule = (patch: Partial<StrategySpecDocument['rules'][number]>) => {
    setDocument((current) => ({ ...current, rules: [{ ...current.rules[0], ...patch }] }))
  }
  const submit = async (event: FormEvent) => {
    event.preventDefault()
    const result = await validate.mutateAsync(document)
    if (!result.valid || !result.document) return
    const saved = await create.mutateAsync(result.document)
    setSelected(`${saved.policy.id}@${saved.policy.version}`)
  }

  return <div className="mx-auto grid w-full max-w-7xl gap-4 p-4 lg:grid-cols-[18rem_minmax(0,1fr)_22rem] lg:p-6">
    <Card>
      <CardHeader><CardTitle className="flex items-center gap-2"><Code2 className="size-4" />策略版本</CardTitle><CardDescription>不可变、只读、无自由代码。</CardDescription></CardHeader>
      <CardContent className="space-y-2">
        {strategies.isPending && <p className="text-sm text-muted-foreground">加载中…</p>}
        {strategies.data?.map((strategy) => <button key={`${strategy.policy.id}@${strategy.policy.version}`} type="button" onClick={() => setSelected(`${strategy.policy.id}@${strategy.policy.version}`)} className={`w-full rounded-lg border p-3 text-left text-sm ${selectedStrategy?.policy.id === strategy.policy.id && selectedStrategy.policy.version === strategy.policy.version ? 'border-primary bg-primary/5' : 'hover:bg-muted/50'}`}><strong>{strategy.name}</strong><span className="mt-1 block font-mono text-xs text-muted-foreground">{strategy.policy.id}@{strategy.policy.version}</span></button>)}
        {!strategies.isPending && !strategies.data?.length && <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">暂无自定义策略。</p>}
      </CardContent>
    </Card>

    <div className="space-y-4">
      <Card className="border-sky-200 bg-sky-50/40 dark:border-sky-950 dark:bg-sky-950/20">
        <CardHeader><CardTitle className="flex items-center gap-2"><ShieldCheck className="size-4 text-sky-700" />受限策略 Studio</CardTitle><CardDescription>仅可配置 RSI(14)、VIX、比较条件和机会桶倍率/跳过动作；核心桶始终保留。</CardDescription></CardHeader>
        <CardContent><form className="grid gap-3" onSubmit={(event) => void submit(event)}>
          <label className="grid gap-1 text-sm font-medium">策略 ID<Input value={document.policy_id} onChange={(event) => setDocument((value) => ({ ...value, policy_id: event.target.value }))} /></label>
          <div className="grid gap-3 sm:grid-cols-2"><label className="grid gap-1 text-sm font-medium">版本<Input type="number" min="1" value={document.policy_version} onChange={(event) => setDocument((value) => ({ ...value, policy_version: Number(event.target.value) }))} /></label><label className="grid gap-1 text-sm font-medium">显示名称<Input value={document.name} onChange={(event) => setDocument((value) => ({ ...value, name: event.target.value }))} /></label></div>
          <div className="grid gap-3 rounded-lg border bg-background p-3 sm:grid-cols-3"><label className="grid gap-1 text-sm font-medium">指标<select className="h-9 rounded-md border bg-background px-2" value={document.rules[0].condition.expression.indicator.kind} onChange={(event) => { const indicator = event.target.value === 'vix' ? { kind: 'vix' as const } : { kind: 'relative_strength_index' as const, lookback_days: 14 }; updateRule({ condition: { ...document.rules[0].condition, expression: { kind: 'indicator', indicator } } }) }}><option value="relative_strength_index">RSI (14)</option><option value="vix">VIX</option></select></label><label className="grid gap-1 text-sm font-medium">条件<select className="h-9 rounded-md border bg-background px-2" value={document.rules[0].condition.operator} onChange={(event) => updateRule({ condition: { ...document.rules[0].condition, operator: event.target.value as typeof document.rules[0]['condition']['operator'] } })}><option value="less_than">小于</option><option value="less_than_or_equal">小于等于</option><option value="greater_than">大于</option><option value="greater_than_or_equal">大于等于</option></select></label><label className="grid gap-1 text-sm font-medium">阈值<Input value={document.rules[0].condition.threshold} onChange={(event) => updateRule({ condition: { ...document.rules[0].condition, threshold: event.target.value } })} /></label></div>
          <div className="grid gap-3 sm:grid-cols-2"><label className="grid gap-1 text-sm font-medium">机会桶动作<select className="h-9 rounded-md border bg-background px-2" value={document.rules[0].action.kind} onChange={(event) => updateRule({ action: event.target.value === 'skip_opportunity' ? { kind: 'skip_opportunity' } : { kind: 'set_opportunity_multiplier', multiplier: 1 } })}><option value="set_opportunity_multiplier">设置倍率</option><option value="skip_opportunity">跳过机会桶</option></select></label>{document.rules[0].action.kind === 'set_opportunity_multiplier' && <label className="grid gap-1 text-sm font-medium">倍率（0–1.5）<Input type="number" min="0" max="1.5" step="0.05" value={document.rules[0].action.multiplier} onChange={(event) => updateRule({ action: { kind: 'set_opportunity_multiplier', multiplier: Number(event.target.value) } })} /></label>}</div>
          {error && <p className="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">{error instanceof Error ? error.message : error}</p>}
          {validate.data?.valid && <p className="flex items-center gap-2 text-sm text-emerald-700"><CheckCircle2 className="size-4" />校验通过，可保存为不可变版本。</p>}
          <Button type="submit" disabled={validate.isPending || create.isPending}><Save className="mr-2 size-4" />验证并保存版本</Button>
        </form></CardContent>
      </Card>
      {selectedStrategy && <Card><CardHeader><CardTitle>只读规则详情</CardTitle><CardDescription>{selectedStrategy.name} · {selectedStrategy.policy.id}@{selectedStrategy.policy.version}</CardDescription></CardHeader><CardContent><pre className="overflow-auto rounded-lg bg-muted p-3 text-xs leading-5">{JSON.stringify(selectedStrategy.document, null, 2)}</pre></CardContent></Card>}
    </div>

    <Card><CardHeader><CardTitle>激活到计划</CardTitle><CardDescription>需用户确认；审批模式只生成建议与审计，不会自动下单。</CardDescription></CardHeader><CardContent className="space-y-3">{!selectedStrategy ? <p className="text-sm text-muted-foreground">先保存或选择一个策略版本。</p> : plans.map((plan) => <div key={plan.id} className="rounded-lg border p-3"><p className="font-medium">{plan.name} · {plan.symbol}</p><p className="mt-1 text-xs text-muted-foreground">当前：{plan.policy.id}@{plan.policy.version}</p><Button className="mt-3 w-full" size="sm" disabled={activate.isPending} onClick={() => { if (globalThis.confirm(`将 ${selectedStrategy.name} 绑定到 ${plan.name}？`)) activate.mutate({ planId: plan.id, policy: selectedStrategy.policy }) }}>确认激活</Button></div>)}</CardContent></Card>
  </div>
}
