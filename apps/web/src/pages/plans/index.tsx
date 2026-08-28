import { useState, type FormEvent } from 'react'
import { CalendarClock, Plus, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSnapshot } from 'valtio'

import { useCreatePlan, useDeletePlan, usePlans, useUpdatePlan } from '@/api/queries'
import type { CreateInvestmentPlanRequest, InvestmentPlan, PolicyReference, UpdateInvestmentPlanRequest } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { setSelectedPlanId, uiStore } from '@/stores/ui'

const initialPlan: CreateInvestmentPlanRequest = {
  name: '',
  symbol: '',
  base_contribution: '1000.00',
  currency: 'USD',
  schedule_kind: 'monthly',
  schedule_day: 15,
  schedule_days: [15],
  policy: { id: 'fixed_dca', version: 1 },
  bucket_allocation: { core_ratio: '1.00', opportunity_ratio: '0.00' },
  risk_mode: 'fixed',
  opportunity_cash_policy: 'expire_each_period',
  max_single_execution: '1500.00',
}

/** Create, list, and select live investment plans from the Rust API. */
export default function PlansPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { selectedPlanId } = useSnapshot(uiStore)
  const { data: plans = [], isPending, error } = usePlans()
  const create = useCreatePlan()
  const remove = useDeletePlan()
  const updatePlan = useUpdatePlan()
  const [input, setInput] = useState(initialPlan)
  const [editingPlanId, setEditingPlanId] = useState<string | null>(null)
  const requestError = create.error ?? updatePlan.error ?? error

  const update = <K extends keyof CreateInvestmentPlanRequest>(key: K, value: string | number) => {
    setInput((current) => ({ ...current, [key]: value }) as CreateInvestmentPlanRequest)
  }
  const updatePolicy = (id: PolicyReference['id']) => {
    setInput((current) => ({ ...current, policy: { id, version: 1 } }))
  }
  const updateBucket = (key: 'core_ratio' | 'opportunity_ratio', value: string) => {
    setInput((current) => ({
      ...current,
      bucket_allocation: { ...current.bucket_allocation!, [key]: value },
    }))
  }
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const normalizedInput = {
      ...input,
      opportunity_cash_cap: input.opportunity_cash_cap?.trim() || undefined,
      period_execution_limit: input.period_execution_limit?.trim() || undefined,
    }
    if (editingPlanId) {
      const { symbol: _symbol, currency: _currency, schedule_kind: _scheduleKind, ...patch } = normalizedInput
      await updatePlan.mutateAsync({ planId: editingPlanId, input: patch as UpdateInvestmentPlanRequest })
      setEditingPlanId(null)
      setInput(initialPlan)
      return
    }
    const created = await create.mutateAsync(normalizedInput)
    setSelectedPlanId(created.id)
    setInput(initialPlan)
    navigate('/')
  }
  const edit = (plan: InvestmentPlan) => {
    setEditingPlanId(plan.id)
    setInput({
      name: plan.name,
      symbol: plan.symbol,
      base_contribution: plan.base_contribution,
      currency: plan.currency,
      schedule_kind: plan.schedule_kind,
      schedule_day: plan.schedule_day,
      schedule_days: plan.schedule_days,
      policy: plan.policy,
      bucket_allocation: plan.execution_configuration.bucket_allocation,
      risk_mode: plan.execution_configuration.risk_mode,
      opportunity_cash_policy: plan.execution_configuration.opportunity_cash_policy,
      opportunity_cash_cap: plan.execution_configuration.opportunity_cash_cap,
      period_execution_limit: plan.execution_configuration.period_execution_limit,
      max_single_execution: plan.max_single_execution,
    })
  }

  return (
    <div className="mx-auto grid w-full max-w-6xl gap-4 p-4 lg:grid-cols-[minmax(0,1fr)_24rem] lg:p-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <CalendarClock className="size-4 text-muted-foreground" />
            {t('live.plans.title')}
          </CardTitle>
          <CardDescription>{t('live.plans.description')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {isPending && <p className="text-sm text-muted-foreground">{t('common.loading')}</p>}
          {!isPending && plans.length === 0 && (
            <p className="rounded-lg border border-dashed p-4 text-sm text-muted-foreground">
              {t('live.plans.empty')}
            </p>
          )}
          {plans.map((plan) => (
            <div key={plan.id} className={`flex w-full items-start gap-2 rounded-lg border p-1 transition-colors hover:bg-muted/50 ${selectedPlanId === plan.id ? 'border-primary bg-primary/5' : 'border-border'}`}>
              <button type="button" onClick={() => setSelectedPlanId(plan.id)} className="min-w-0 flex-1 rounded-md p-3 text-left">
                <div className="flex items-center justify-between gap-3"><span className="font-semibold">{plan.name}</span><span className="font-mono text-sm">{plan.symbol}</span></div>
                <div className="mt-2 text-sm text-muted-foreground">{plan.currency} {plan.base_contribution} · {plan.schedule_kind === 'weekly' ? '每周' : '每月'} {plan.schedule_days.join('、')} · 核心 {plan.execution_configuration.bucket_allocation.core_ratio} / 机会 {plan.execution_configuration.bucket_allocation.opportunity_ratio} · {plan.execution_configuration.risk_mode} · {plan.policy.id}@{plan.policy.version}</div>
              </button>
              <Button type="button" variant="outline" size="sm" className="mt-2" onClick={() => edit(plan)}>编辑</Button>
              <Button type="button" variant="outline" size="sm" className="mt-2" disabled={updatePlan.isPending} onClick={() => updatePlan.mutate({ planId: plan.id, input: { is_active: !plan.is_active } })}>{plan.is_active ? '暂停' : '启用'}</Button>
              <Button type="button" variant="ghost" size="icon" className="mt-1 shrink-0 text-muted-foreground hover:text-destructive" aria-label={`删除 ${plan.name}`} disabled={remove.isPending} onClick={() => { if (globalThis.confirm(`删除“${plan.name}”及其本地决策、账本和快照记录？此操作不可恢复。`)) { if (selectedPlanId === plan.id) setSelectedPlanId(null); remove.mutate(plan.id) } }}><Trash2 className="size-4" /></Button>
            </div>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Plus className="size-4 text-muted-foreground" />
            {editingPlanId ? '编辑定投计划' : t('live.plans.create')}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form className="space-y-3" onSubmit={(event) => void submit(event)}>
            <PlanField label={t('live.plans.name')} value={input.name} onChange={(value) => update('name', value)} />
            <PlanField label={t('live.plans.symbol')} value={input.symbol} onChange={(value) => update('symbol', value)} />
            <PlanField
              label={t('live.plans.baseContribution')}
              value={input.base_contribution}
              onChange={(value) => update('base_contribution', value)}
            />
            <label className="grid gap-1.5 text-sm font-medium">
              {t('live.plans.policy')}
              <select
                className="h-9 rounded-md border border-input bg-transparent px-3 text-sm shadow-xs outline-none focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/50"
                value={input.policy?.id ?? 'fixed_dca'}
                onChange={(event) => updatePolicy(event.target.value as PolicyReference['id'])}
              >
                <option value="fixed_dca">{t('live.plans.fixedDca')}</option>
                <option value="core_opportunity_v1">{t('live.plans.coreOpportunity')}</option>
              </select>
              <span className="text-xs font-normal text-muted-foreground">
                {input.policy?.id === 'fixed_dca' ? t('live.plans.fixedDcaDescription') : t('live.plans.coreOpportunityDescription')}
              </span>
            </label>
            <PlanField label={t('live.plans.currency')} value={input.currency} onChange={(value) => update('currency', value)} />
            <label className="grid gap-1.5 text-sm font-medium">周期
              <select disabled={editingPlanId !== null} className="h-9 rounded-md border border-input bg-transparent px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60" value={input.schedule_kind} onChange={(event) => {
                const schedule_kind = event.target.value as CreateInvestmentPlanRequest['schedule_kind']
                const schedule_day = schedule_kind === 'weekly' ? 1 : 15
                setInput((current) => ({ ...current, schedule_kind, schedule_day, schedule_days: [schedule_day] }))
              }}><option value="monthly">每月固定日期</option><option value="weekly">每周固定星期</option></select>
              {editingPlanId && <span className="text-xs font-normal text-muted-foreground">周期类型与标的/币种共同定义账本口径；如需切换月度/周度，请创建新计划并暂停旧计划。</span>}
            </label>
            <PlanField label={input.schedule_kind === 'weekly' ? '固定星期（1=周一，7=周日）' : '固定日期（1–28）'} value={(input.schedule_days ?? [input.schedule_day]).join(',')} onChange={(value) => {
              const schedule_days = value.split(',').map((item) => Number(item.trim())).filter(Number.isFinite).sort((left, right) => left - right)
              setInput((current) => ({ ...current, schedule_days, schedule_day: schedule_days[0] ?? current.schedule_day }))
            }} />
            <div className="grid gap-3 sm:grid-cols-2">
              <PlanField label="核心桶比例（0–1）" value={input.bucket_allocation?.core_ratio ?? '1.00'} onChange={(value) => updateBucket('core_ratio', value)} />
              <PlanField label="机会桶比例（0–1）" value={input.bucket_allocation?.opportunity_ratio ?? '0.00'} onChange={(value) => updateBucket('opportunity_ratio', value)} />
            </div>
            <label className="grid gap-1.5 text-sm font-medium">机会桶执行模式
              <select className="h-9 rounded-md border border-input bg-transparent px-3 text-sm" value={input.risk_mode ?? 'fixed'} onChange={(event) => setInput((current) => ({ ...current, risk_mode: event.target.value as NonNullable<CreateInvestmentPlanRequest['risk_mode']> }))}><option value="fixed">固定定投（仅核心桶）</option><option value="autopilot">自动执行机会桶</option><option value="approval">生成建议后人工审批</option></select>
            </label>
            <label className="grid gap-1.5 text-sm font-medium">机会现金策略
              <select className="h-9 rounded-md border border-input bg-transparent px-3 text-sm" value={input.opportunity_cash_policy ?? 'expire_each_period'} onChange={(event) => setInput((current) => ({ ...current, opportunity_cash_policy: event.target.value as NonNullable<CreateInvestmentPlanRequest['opportunity_cash_policy']> }))}><option value="expire_each_period">当期到期</option><option value="carry_forward">滚存</option><option value="carry_with_cap">滚存并设上限</option></select>
            </label>
            {input.opportunity_cash_policy === 'carry_with_cap' && <PlanField label="机会现金上限" value={input.opportunity_cash_cap ?? ''} onChange={(value) => update('opportunity_cash_cap', value)} />}
            <PlanField label="周期累计执行上限（可选）" value={input.period_execution_limit ?? ''} onChange={(value) => update('period_execution_limit', value)} />
            <PlanField
              label={t('live.plans.maxExecution')}
              value={input.max_single_execution}
              onChange={(value) => update('max_single_execution', value)}
            />
            {requestError && (
              <p className="text-sm text-destructive">
                {requestError instanceof Error ? requestError.message : 'request failed'}
              </p>
            )}
            <Button className="w-full" type="submit" disabled={create.isPending || updatePlan.isPending}>
              {create.isPending || updatePlan.isPending ? '正在保存…' : editingPlanId ? '保存计划配置' : t('live.plans.create')}
            </Button>
            {editingPlanId && <Button className="w-full" type="button" variant="outline" onClick={() => { setEditingPlanId(null); setInput(initialPlan) }}>取消编辑</Button>}
          </form>
        </CardContent>
      </Card>
    </div>
  )
}

/** Render one compact controlled plan field. */
function PlanField({
  label,
  value,
  onChange,
}: {
  label: string
  value: string
  onChange: (value: string) => void
}) {
  return (
    <label className="grid gap-1.5 text-sm font-medium">
      {label}
      <Input required value={value} onChange={(event) => onChange(event.target.value)} />
    </label>
  )
}
