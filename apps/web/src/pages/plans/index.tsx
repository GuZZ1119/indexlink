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
      const patch: Partial<CreateInvestmentPlanRequest> = { ...normalizedInput }
      delete patch.symbol
      delete patch.currency
      delete patch.schedule_kind
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
                <div className="mt-2 text-sm text-muted-foreground">{plan.currency} {plan.base_contribution} · {plan.schedule_kind === 'weekly' ? t('plansV11.weekly') : t('plansV11.monthly')} {plan.schedule_days.join('、')} · {t('plansV11.core')} {plan.execution_configuration.bucket_allocation.core_ratio} / {t('plansV11.opportunity')} {plan.execution_configuration.bucket_allocation.opportunity_ratio} · {plan.execution_configuration.risk_mode} · {plan.policy.id}@{plan.policy.version}</div>
              </button>
              <Button type="button" variant="outline" size="sm" className="mt-2" onClick={() => edit(plan)}>{t('plansV11.edit')}</Button>
              <Button type="button" variant="outline" size="sm" className="mt-2" disabled={updatePlan.isPending} onClick={() => updatePlan.mutate({ planId: plan.id, input: { is_active: !plan.is_active } })}>{plan.is_active ? t('plansV11.pause') : t('plansV11.resume')}</Button>
              <Button type="button" variant="ghost" size="icon" className="mt-1 shrink-0 text-muted-foreground hover:text-destructive" aria-label={t('plansV11.remove', { name: plan.name })} disabled={remove.isPending} onClick={() => { if (globalThis.confirm(t('plansV11.removeConfirm', { name: plan.name }))) { if (selectedPlanId === plan.id) setSelectedPlanId(null); remove.mutate(plan.id) } }}><Trash2 className="size-4" /></Button>
            </div>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Plus className="size-4 text-muted-foreground" />
            {editingPlanId ? t('plansV11.editTitle') : t('live.plans.create')}
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
            <label className="grid gap-1.5 text-sm font-medium">{t('plansV11.period')}
              <select disabled={editingPlanId !== null} className="h-9 rounded-md border border-input bg-transparent px-3 text-sm disabled:cursor-not-allowed disabled:opacity-60" value={input.schedule_kind} onChange={(event) => {
                const schedule_kind = event.target.value as CreateInvestmentPlanRequest['schedule_kind']
                const schedule_day = schedule_kind === 'weekly' ? 1 : 15
                setInput((current) => ({ ...current, schedule_kind, schedule_day, schedule_days: [schedule_day] }))
              }}><option value="monthly">{t('plansV11.monthlyFixed')}</option><option value="weekly">{t('plansV11.weeklyFixed')}</option></select>
              {editingPlanId && <span className="text-xs font-normal text-muted-foreground">{t('plansV11.immutableSchedule')}</span>}
            </label>
            <PlanField label={input.schedule_kind === 'weekly' ? t('plansV11.weekdays') : t('plansV11.days')} value={(input.schedule_days ?? [input.schedule_day]).join(',')} onChange={(value) => {
              const schedule_days = value.split(',').map((item) => Number(item.trim())).filter(Number.isFinite).sort((left, right) => left - right)
              setInput((current) => ({ ...current, schedule_days, schedule_day: schedule_days[0] ?? current.schedule_day }))
            }} />
            <div className="grid gap-3 sm:grid-cols-2">
              <PlanField label={t('plansV11.coreRatio')} value={input.bucket_allocation?.core_ratio ?? '1.00'} onChange={(value) => updateBucket('core_ratio', value)} />
              <PlanField label={t('plansV11.opportunityRatio')} value={input.bucket_allocation?.opportunity_ratio ?? '0.00'} onChange={(value) => updateBucket('opportunity_ratio', value)} />
            </div>
            <label className="grid gap-1.5 text-sm font-medium">{t('plansV11.opportunityMode')}
              <select className="h-9 rounded-md border border-input bg-transparent px-3 text-sm" value={input.risk_mode ?? 'fixed'} onChange={(event) => setInput((current) => ({ ...current, risk_mode: event.target.value as NonNullable<CreateInvestmentPlanRequest['risk_mode']> }))}><option value="fixed">{t('plansV11.fixedCore')}</option><option value="autopilot">{t('plansV11.autopilotOpportunity')}</option><option value="approval">{t('plansV11.approvalOpportunity')}</option></select>
            </label>
            <label className="grid gap-1.5 text-sm font-medium">{t('plansV11.cashPolicy')}
              <select className="h-9 rounded-md border border-input bg-transparent px-3 text-sm" value={input.opportunity_cash_policy ?? 'expire_each_period'} onChange={(event) => setInput((current) => ({ ...current, opportunity_cash_policy: event.target.value as NonNullable<CreateInvestmentPlanRequest['opportunity_cash_policy']> }))}><option value="expire_each_period">{t('plansV11.expire')}</option><option value="carry_forward">{t('plansV11.carry')}</option><option value="carry_with_cap">{t('plansV11.carryCap')}</option></select>
            </label>
            {input.opportunity_cash_policy === 'carry_with_cap' && <PlanField label={t('plansV11.cashCap')} value={input.opportunity_cash_cap ?? ''} onChange={(value) => update('opportunity_cash_cap', value)} />}
            <PlanField label={t('plansV11.periodLimit')} value={input.period_execution_limit ?? ''} onChange={(value) => update('period_execution_limit', value)} />
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
              {create.isPending || updatePlan.isPending ? t('plansV11.saving') : editingPlanId ? t('plansV11.save') : t('live.plans.create')}
            </Button>
            {editingPlanId && <Button className="w-full" type="button" variant="outline" onClick={() => { setEditingPlanId(null); setInput(initialPlan) }}>{t('plansV11.cancel')}</Button>}
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
