import { useState, type FormEvent } from 'react'
import { CalendarClock, Plus, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSnapshot } from 'valtio'

import { useCreatePlan, useDeletePlan, usePlans } from '@/api/queries'
import type { CreateInvestmentPlanRequest, PolicyReference } from '@/api/types'
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
  policy: { id: 'fixed_dca', version: 1 },
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
  const [input, setInput] = useState(initialPlan)
  const requestError = create.error ?? error

  const update = <K extends keyof CreateInvestmentPlanRequest>(key: K, value: string | number) => {
    setInput((current) => ({ ...current, [key]: value }) as CreateInvestmentPlanRequest)
  }
  const updatePolicy = (id: PolicyReference['id']) => {
    setInput((current) => ({ ...current, policy: { id, version: 1 } }))
  }
  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const created = await create.mutateAsync(input)
    setSelectedPlanId(created.id)
    setInput(initialPlan)
    navigate('/')
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
                <div className="mt-2 text-sm text-muted-foreground">{plan.currency} {plan.base_contribution} · {t('live.plans.scheduleDay')} {plan.schedule_day} · {t('live.plans.maxExecution')} {plan.max_single_execution} · {plan.policy.id}@{plan.policy.version}</div>
              </button>
              <Button type="button" variant="ghost" size="icon" className="mt-1 shrink-0 text-muted-foreground hover:text-destructive" aria-label={`删除 ${plan.name}`} disabled={remove.isPending} onClick={() => { if (globalThis.confirm(`删除“${plan.name}”及其本地决策、账本和快照记录？此操作不可恢复。`)) { if (selectedPlanId === plan.id) setSelectedPlanId(null); remove.mutate(plan.id) } }}><Trash2 className="size-4" /></Button>
            </div>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Plus className="size-4 text-muted-foreground" />
            {t('live.plans.create')}
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
            <PlanField
              label={t('live.plans.scheduleDay')}
              value={String(input.schedule_day)}
              onChange={(value) => update('schedule_day', Number(value))}
            />
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
            <Button className="w-full" type="submit" disabled={create.isPending}>
              {create.isPending ? t('live.plans.creating') : t('live.plans.create')}
            </Button>
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
