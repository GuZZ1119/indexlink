import { useState, type FormEvent } from 'react'
import { CalendarClock, Plus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSnapshot } from 'valtio'

import { useCreatePlan, usePlans } from '@/api/queries'
import type { CreateInvestmentPlanRequest } from '@/api/types'
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
  max_single_execution: '1500.00',
}

/** Create, list, and select live investment plans from the Rust API. */
export default function PlansPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { selectedPlanId } = useSnapshot(uiStore)
  const { data: plans = [], isPending, error } = usePlans()
  const create = useCreatePlan()
  const [input, setInput] = useState(initialPlan)
  const requestError = create.error ?? error

  const update = <K extends keyof CreateInvestmentPlanRequest>(key: K, value: string | number) => {
    setInput((current) => ({ ...current, [key]: value }) as CreateInvestmentPlanRequest)
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
            <button
              type="button"
              key={plan.id}
              onClick={() => setSelectedPlanId(plan.id)}
              className={`w-full rounded-lg border p-4 text-left transition-colors hover:bg-muted/50 ${
                selectedPlanId === plan.id ? 'border-primary bg-primary/5' : 'border-border'
              }`}
            >
              <div className="flex items-center justify-between gap-3">
                <span className="font-semibold">{plan.name}</span>
                <span className="font-mono text-sm">{plan.symbol}</span>
              </div>
              <div className="mt-2 text-sm text-muted-foreground">
                {plan.currency} {plan.base_contribution} · {t('live.plans.scheduleDay')} {plan.schedule_day} · {t('live.plans.maxExecution')}{' '}
                {plan.max_single_execution}
              </div>
            </button>
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
