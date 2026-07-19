import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { BarChart3, Bot, ClipboardCheck, Send } from 'lucide-react'
import { useSnapshot } from 'valtio'

import {
  ApiRequestError,
  previewDecision,
  previewFundamental,
  previewTrend,
  usePlans,
} from '@/api/queries'
import type { DecisionPreviewResponse } from '@/api/types'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { actionBadgeClass, formatCurrency, formatMultiplier } from '@/lib/decision'
import { cn } from '@/lib/utils'
import { setSelectedPlanId, uiStore } from '@/stores/ui'

type SignalFields = {
  capeHistory: string
  capeCurrent: string
  erpHistory: string
  erpCurrent: string
  maHistory: string
  maCurrent: string
  rsiHistory: string
  rsiCurrent: string
  vixHistory: string
  vixCurrent: string
}

const emptySignals: SignalFields = {
  capeHistory: '',
  capeCurrent: '',
  erpHistory: '',
  erpCurrent: '',
  maHistory: '',
  maCurrent: '',
  rsiHistory: '',
  rsiCurrent: '',
  vixHistory: '',
  vixCurrent: '',
}

/** Parse a comma- or newline-separated historical series without manufacturing data. */
function parseHistory(value: string, label: string): number[] {
  const values = value
    .split(/[\s,]+/)
    .filter(Boolean)
    .map(Number)
  if (values.length === 0 || values.some((item) => !Number.isFinite(item))) {
    throw new ApiRequestError(`${label} must contain finite numbers`)
  }
  return values
}

/** Parse one finite current indicator value. */
function parseCurrent(value: string, label: string): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) {
    throw new ApiRequestError(`${label} must be a finite number`)
  }
  return parsed
}

/** Return a unique, non-secret idempotency key for one user-confirmed paper order. */
function paperOrderKey(): string {
  const suffix = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`
  return `web-paper-${suffix}`
}

/** Render the live Decision Preview workflow backed by Rust HTTP APIs. */
export default function DashboardPage() {
  const { t } = useTranslation()
  const { data: plans = [], isPending: plansPending, error: plansError } = usePlans()
  const { selectedPlanId } = useSnapshot(uiStore)
  const queryClient = useQueryClient()
  const [signals, setSignals] = useState<SignalFields>(emptySignals)
  const [dayOfMonth, setDayOfMonth] = useState(String(new Date().getDate()))
  const [coreRatio, setCoreRatio] = useState('0.80')
  const [opportunityRatio, setOpportunityRatio] = useState('0.20')
  const [submitPaperOrder, setSubmitPaperOrder] = useState(false)
  const [quantity, setQuantity] = useState('1.00')
  const [result, setResult] = useState<DecisionPreviewResponse | null>(null)

  useEffect(() => {
    if (selectedPlanId === null && plans[0]) {
      setSelectedPlanId(plans[0].id)
    }
  }, [plans, selectedPlanId])

  const selectedPlan = useMemo(
    () => plans.find((plan) => plan.id === selectedPlanId) ?? null,
    [plans, selectedPlanId],
  )

  const decisionMutation = useMutation({
    mutationFn: async () => {
      if (!selectedPlan) {
        throw new ApiRequestError('select a plan before running a decision')
      }
      const [fundamental, trend] = await Promise.all([
        previewFundamental({
          cape_history: parseHistory(signals.capeHistory, 'CAPE history'),
          cape_current: parseCurrent(signals.capeCurrent, 'CAPE current'),
          erp_history: parseHistory(signals.erpHistory, 'ERP history'),
          erp_current: parseCurrent(signals.erpCurrent, 'ERP current'),
        }),
        previewTrend({
          ma_distance_history: parseHistory(signals.maHistory, 'MA200 history'),
          ma_distance_current: parseCurrent(signals.maCurrent, 'MA200 current'),
          rsi_history: parseHistory(signals.rsiHistory, 'RSI history'),
          rsi_current: parseCurrent(signals.rsiCurrent, 'RSI current'),
          vix_history: parseHistory(signals.vixHistory, 'VIX history'),
          vix_current: parseCurrent(signals.vixCurrent, 'VIX current'),
        }),
      ])
      return previewDecision(selectedPlan.id, {
        day_of_month: Number(dayOfMonth),
        bucket_allocation: {
          core_ratio: coreRatio,
          opportunity_ratio: opportunityRatio,
        },
        fundamental,
        trend,
        ...(submitPaperOrder
          ? {
              paper_order: {
                idempotency_key: paperOrderKey(),
                side: 'buy' as const,
                order_type: 'market' as const,
                quantity,
              },
            }
          : {}),
      })
    },
    onSuccess: async (next) => {
      setResult(next)
      await queryClient.invalidateQueries({ queryKey: ['decision-records', selectedPlanId] })
    },
  })

  const updateSignal = (key: keyof SignalFields, value: string) => {
    setSignals((current) => ({ ...current, [key]: value }))
  }
  const error = decisionMutation.error ?? plansError

  return (
    <div className="mx-auto flex w-full max-w-6xl flex-col gap-4 p-4 lg:p-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ClipboardCheck className="size-4 text-muted-foreground" />
            {t('live.decision.title')}
          </CardTitle>
          <CardDescription>
            {t('live.decision.description')}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          <label className="grid gap-1.5 text-sm font-medium">
            {t('live.decision.plan')}
            <select
              className="h-8 rounded-lg border border-input bg-transparent px-2.5 text-sm"
              value={selectedPlanId ?? ''}
              disabled={plansPending || plans.length === 0}
              onChange={(event) => setSelectedPlanId(event.target.value || null)}
            >
              {plans.length === 0 ? (
                <option value="">{t('live.decision.createPlanFirst')}</option>
              ) : (
                plans.map((plan) => (
                  <option key={plan.id} value={plan.id}>
                    {plan.name} · {plan.symbol} · {plan.currency} {plan.base_contribution}
                  </option>
                ))
              )}
            </select>
          </label>

          <div className="grid gap-4 md:grid-cols-3">
            <label className="grid gap-1.5 text-sm font-medium">
              {t('live.decision.day')}
              <Input value={dayOfMonth} onChange={(event) => setDayOfMonth(event.target.value)} />
            </label>
            <label className="grid gap-1.5 text-sm font-medium">
              {t('live.decision.coreRatio')}
              <Input value={coreRatio} onChange={(event) => setCoreRatio(event.target.value)} />
            </label>
            <label className="grid gap-1.5 text-sm font-medium">
              {t('live.decision.opportunityRatio')}
              <Input
                value={opportunityRatio}
                onChange={(event) => setOpportunityRatio(event.target.value)}
              />
            </label>
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 lg:grid-cols-2">
        <SignalInputCard
          title={t('live.decision.fundamental')}
          icon={<BarChart3 className="size-4 text-muted-foreground" />}
          fields={[
            [t('live.decision.capeHistory'), 'capeHistory'],
            [t('live.decision.capeCurrent'), 'capeCurrent'],
            [t('live.decision.erpHistory'), 'erpHistory'],
            [t('live.decision.erpCurrent'), 'erpCurrent'],
          ]}
          values={signals}
          onChange={updateSignal}
        />
        <SignalInputCard
          title={t('live.decision.trend')}
          icon={<BarChart3 className="size-4 text-muted-foreground" />}
          fields={[
            [t('live.decision.maHistory'), 'maHistory'],
            [t('live.decision.maCurrent'), 'maCurrent'],
            [t('live.decision.rsiHistory'), 'rsiHistory'],
            [t('live.decision.rsiCurrent'), 'rsiCurrent'],
            [t('live.decision.vixHistory'), 'vixHistory'],
            [t('live.decision.vixCurrent'), 'vixCurrent'],
          ]}
          values={signals}
          onChange={updateSignal}
        />
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Send className="size-4 text-muted-foreground" />
            {t('live.decision.paperTitle')}
          </CardTitle>
          <CardDescription>
            {t('live.decision.paperDescription')}
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap items-end gap-4">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={submitPaperOrder}
              onChange={(event) => setSubmitPaperOrder(event.target.checked)}
            />
            {t('live.decision.submitPaper')}
          </label>
          {submitPaperOrder && (
            <label className="grid gap-1.5 text-sm font-medium">
              {t('live.decision.quantity')}
              <Input value={quantity} onChange={(event) => setQuantity(event.target.value)} />
            </label>
          )}
          <Button
            className="ml-auto"
            disabled={!selectedPlan || decisionMutation.isPending}
            onClick={() => decisionMutation.mutate()}
          >
            {decisionMutation.isPending ? t('live.decision.running') : t('live.decision.run')}
          </Button>
        </CardContent>
      </Card>

      {error && (
        <p className="rounded-lg border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
          {error instanceof Error ? error.message : 'request failed'}
        </p>
      )}

      {result && <DecisionResultCard result={result} />}
    </div>
  )
}

/** Render a group of historical and current signal inputs. */
function SignalInputCard({
  title,
  icon,
  fields,
  values,
  onChange,
}: {
  title: string
  icon: ReactNode
  fields: Array<[string, keyof SignalFields]>
  values: SignalFields
  onChange: (key: keyof SignalFields, value: string) => void
}) {
  const { t } = useTranslation()
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">{icon}{title}</CardTitle>
        <CardDescription>{t('live.decision.historyHelp')}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {fields.map(([label, key]) => {
          const history = key.endsWith('History')
          return (
            <label key={key} className="grid gap-1.5 text-sm font-medium">
              {label}
              {history ? (
                <textarea
                  className="min-h-20 rounded-lg border border-input bg-transparent px-2.5 py-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
                  value={values[key]}
                  onChange={(event) => onChange(key, event.target.value)}
                />
              ) : (
                <Input value={values[key]} onChange={(event) => onChange(key, event.target.value)} />
              )}
            </label>
          )
        })}
      </CardContent>
    </Card>
  )
}

/** Render a non-fabricated Decision Preview response and optional broker acknowledgement. */
function DecisionResultCard({ result }: { result: DecisionPreviewResponse }) {
  const { t } = useTranslation()
  const planned = result.execution.planned_contribution
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Bot className="size-4 text-muted-foreground" />
          {t('live.decision.result')}
          <Badge className={cn(actionBadgeClass[result.decision.action])}>
            {result.decision.action}
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <Metric label={t('live.decision.execution')} value={result.execution.status} />
          <Metric label={t('live.decision.finalScore')} value={result.decision.final_score.toFixed(2)} />
          <Metric label={t('live.decision.multiplier')} value={formatMultiplier(result.decision.multiplier)} />
          <Metric label={t('live.decision.weightMode')} value={result.decision.weight_mode} />
          <Metric
            label={t('live.decision.plannedContribution')}
            value={planned ? formatCurrency(Number(planned), result.execution.currency) : '—'}
          />
          <Metric label={t('live.decision.fundamentalScore')} value={result.decision.fundamental_score.toFixed(2)} />
          <Metric label={t('live.decision.trendScore')} value={result.decision.trend_score.toFixed(2)} />
          <Metric
            label={t('live.decision.qwenSentiment')}
            value={result.decision.sentiment_score?.toFixed(2) ?? t('live.decision.fallback')}
          />
        </div>
        {result.execution.bucket_split && (
          <div className="rounded-lg border bg-muted/30 p-3 text-sm">
            {t('live.decision.bucketSplit')}: core {result.execution.bucket_split.core_contribution} {result.execution.currency}
            {' · '}opportunity {result.execution.bucket_split.opportunity_contribution}{' '}
            {result.execution.currency}
          </div>
        )}
        <p className="rounded-lg bg-muted/50 p-3 text-sm leading-relaxed text-muted-foreground">
          {result.summary}
        </p>
        {result.paper_order_ack && (
          <div className="rounded-lg border border-semantic-positive/40 bg-semantic-positive/10 p-3 text-sm">
            {t('live.decision.paperAck')}: {result.paper_order_ack.status} · order{' '}
            {result.paper_order_ack.order_id} · {result.paper_order_ack.environment}
          </div>
        )}
      </CardContent>
    </Card>
  )
}

/** Render one compact response metric. */
function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg bg-muted/60 p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 break-words font-mono text-sm font-semibold">{value}</div>
    </div>
  )
}
