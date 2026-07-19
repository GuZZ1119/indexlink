import { useEffect, useMemo, useRef, useState, type FormEvent, type ReactNode } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { BarChart3, Bot, ClipboardCheck, FileJson, Plus, RefreshCw, Send, Upload } from 'lucide-react'
import { useSnapshot } from 'valtio'

import {
  ApiRequestError,
  fetchMarketSignalInput,
  previewDecision,
  previewFundamental,
  previewTrend,
  useCreatePlan,
  usePlans,
} from '@/api/queries'
import type { CreateInvestmentPlanRequest, DecisionPreviewResponse, MarketSignalInput } from '@/api/types'
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

const initialPlan: CreateInvestmentPlanRequest = {
  name: '',
  symbol: '',
  base_contribution: '1000.00',
  currency: 'USD',
  schedule_kind: 'monthly',
  schedule_day: 15,
  max_single_execution: '1500.00',
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

/** Convert one imported JSON document into editable signal fields without creating market data. */
function signalFieldsFromImport(value: unknown): SignalFields {
  if (!isRecord(value)) {
    throw new ApiRequestError('signal import must be a JSON object')
  }
  const fundamental = isRecord(value.fundamental) ? value.fundamental : value
  const trend = isRecord(value.trend) ? value.trend : value
  return {
    capeHistory: importedHistory(fundamental, 'cape_history'),
    capeCurrent: importedCurrent(fundamental, 'cape_current'),
    erpHistory: importedHistory(fundamental, 'erp_history'),
    erpCurrent: importedCurrent(fundamental, 'erp_current'),
    maHistory: importedHistory(trend, 'ma_distance_history'),
    maCurrent: importedCurrent(trend, 'ma_distance_current'),
    rsiHistory: importedHistory(trend, 'rsi_history'),
    rsiCurrent: importedCurrent(trend, 'rsi_current'),
    vixHistory: importedHistory(trend, 'vix_history'),
    vixCurrent: importedCurrent(trend, 'vix_current'),
  }
}

/** Convert one server-refreshed market snapshot into editable dashboard signal fields. */
function signalFieldsFromMarketInput(input: MarketSignalInput): SignalFields {
  return {
    capeHistory: input.fundamental.cape_history.join(', '),
    capeCurrent: String(input.fundamental.cape_current),
    erpHistory: input.fundamental.erp_history.join(', '),
    erpCurrent: String(input.fundamental.erp_current),
    maHistory: input.trend.ma_distance_history.join(', '),
    maCurrent: String(input.trend.ma_distance_current),
    rsiHistory: input.trend.rsi_history.join(', '),
    rsiCurrent: String(input.trend.rsi_current),
    vixHistory: input.trend.vix_history.join(', '),
    vixCurrent: String(input.trend.vix_current),
  }
}

/** Check a JSON value is a non-null object before reading its signal fields. */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/** Read one finite historical numeric series from an imported JSON object. */
function importedHistory(value: Record<string, unknown>, field: string): string {
  const values = value[field]
  if (!Array.isArray(values) || values.some((item) => typeof item !== 'number' || !Number.isFinite(item))) {
    throw new ApiRequestError(`${field} must be an array of finite numbers`)
  }
  return values.join(', ')
}

/** Read one finite current numeric value from an imported JSON object. */
function importedCurrent(value: Record<string, unknown>, field: string): string {
  const current = value[field]
  if (typeof current !== 'number' || !Number.isFinite(current)) {
    throw new ApiRequestError(`${field} must be a finite number`)
  }
  return String(current)
}

/** Render the live Decision Preview workflow backed by Rust HTTP APIs. */
export default function DashboardPage() {
  const { t } = useTranslation()
  const { data: plans = [], isPending: plansPending, error: plansError } = usePlans()
  const { selectedPlanId } = useSnapshot(uiStore)
  const queryClient = useQueryClient()
  const createPlan = useCreatePlan()
  const importInput = useRef<HTMLInputElement>(null)
  const [signals, setSignals] = useState<SignalFields>(emptySignals)
  const [planInput, setPlanInput] = useState(initialPlan)
  const [dayOfMonth, setDayOfMonth] = useState(String(new Date().getDate()))
  const [coreRatio, setCoreRatio] = useState('0.80')
  const [opportunityRatio, setOpportunityRatio] = useState('0.20')
  const [submitPaperOrder, setSubmitPaperOrder] = useState(false)
  const [quantity, setQuantity] = useState('1.00')
  const [result, setResult] = useState<DecisionPreviewResponse | null>(null)
  const [importName, setImportName] = useState<string | null>(null)
  const [importError, setImportError] = useState<string | null>(null)
  const [marketRefresh, setMarketRefresh] = useState<MarketSignalInput | null>(null)
  const [planFormOpen, setPlanFormOpen] = useState(false)

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
  const marketRefreshMutation = useMutation({
    mutationFn: async () => {
      if (!selectedPlan) {
        throw new ApiRequestError('select a plan before refreshing market signals')
      }
      return fetchMarketSignalInput(selectedPlan.symbol)
    },
    onSuccess: (input) => {
      setSignals(signalFieldsFromMarketInput(input))
      setMarketRefresh(input)
      setImportError(null)
      setResult(null)
    },
  })

  const updateSignal = (key: keyof SignalFields, value: string) => {
    setSignals((current) => ({ ...current, [key]: value }))
  }
  const updatePlan = <K extends keyof CreateInvestmentPlanRequest>(key: K, value: string | number) => {
    setPlanInput((current) => ({ ...current, [key]: value }) as CreateInvestmentPlanRequest)
  }
  const createDemoPlan = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const created = await createPlan.mutateAsync(planInput)
    setSelectedPlanId(created.id)
    setPlanInput(initialPlan)
    setPlanFormOpen(false)
  }
  const importSignals = async (file: File) => {
    try {
      const next = signalFieldsFromImport(JSON.parse(await file.text()) as unknown)
      setSignals(next)
      setImportName(file.name)
      setImportError(null)
      setResult(null)
    } catch (error) {
      setImportName(null)
      setImportError(error instanceof Error ? error.message : 'signal import failed')
    }
  }
  const error = marketRefreshMutation.error ?? decisionMutation.error ?? createPlan.error ?? plansError
  const hasSignalInput = Object.values(signals).every((value) => value.trim().length > 0)

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
          <DemoSteps
            planReady={selectedPlan !== null}
            signalReady={hasSignalInput}
            result={result}
            paperOrderRequested={submitPaperOrder}
          />
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

          <details
            className="rounded-lg border border-dashed p-3"
            open={planFormOpen || plans.length === 0}
            onToggle={(event) => setPlanFormOpen(event.currentTarget.open)}
          >
            <summary className="cursor-pointer text-sm font-medium">
              {t('live.decision.createPlanHere')}
            </summary>
            <form className="mt-4 space-y-3" onSubmit={(event) => void createDemoPlan(event)}>
              <div className="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
                <DemoPlanField label={t('live.plans.name')} value={planInput.name} onChange={(value) => updatePlan('name', value)} />
                <DemoPlanField label={t('live.plans.symbol')} value={planInput.symbol} onChange={(value) => updatePlan('symbol', value)} />
                <DemoPlanField label={t('live.plans.currency')} value={planInput.currency} onChange={(value) => updatePlan('currency', value)} />
                <DemoPlanField label={t('live.plans.baseContribution')} value={planInput.base_contribution} onChange={(value) => updatePlan('base_contribution', value)} />
                <DemoPlanField label={t('live.plans.scheduleDay')} value={String(planInput.schedule_day)} onChange={(value) => updatePlan('schedule_day', Number(value))} />
                <DemoPlanField label={t('live.plans.maxExecution')} value={planInput.max_single_execution} onChange={(value) => updatePlan('max_single_execution', value)} />
              </div>
              <Button type="submit" disabled={createPlan.isPending}>
                <Plus className="size-4" />
                {createPlan.isPending ? t('live.plans.creating') : t('live.plans.create')}
              </Button>
            </form>
          </details>

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

      <Card className="border-primary/60 bg-primary/5 shadow-sm">
        <CardHeader className="gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <CardTitle className="flex items-center gap-2 text-primary"><RefreshCw className="size-5" />{t('live.decision.marketRefreshTitle')}</CardTitle>
            <CardDescription>{t('live.decision.marketRefreshDescription')}</CardDescription>
          </div>
          <Button size="lg" disabled={!selectedPlan || marketRefreshMutation.isPending} onClick={() => marketRefreshMutation.mutate()}>
            <RefreshCw className={cn('size-4', marketRefreshMutation.isPending && 'animate-spin')} />
            {marketRefreshMutation.isPending ? t('live.decision.marketRefreshing') : t('live.decision.marketRefresh')}
          </Button>
        </CardHeader>
        {marketRefresh && (
          <CardContent className="text-sm text-muted-foreground">
            {t('live.decision.marketRefreshed', { symbol: marketRefresh.symbol, date: marketRefresh.as_of })}
          </CardContent>
        )}
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileJson className="size-4 text-muted-foreground" />
            {t('live.decision.importTitle')}
          </CardTitle>
          <CardDescription>{t('live.decision.importDescription')}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap items-center gap-3">
          <input
            ref={importInput}
            className="sr-only"
            type="file"
            accept="application/json,.json"
            onChange={(event) => {
              const [file] = event.target.files ?? []
              if (file) {
                void importSignals(file)
              }
              event.target.value = ''
            }}
          />
          <Button type="button" variant="outline" onClick={() => importInput.current?.click()}>
            <Upload className="size-4" />
            {t('live.decision.importButton')}
          </Button>
          <span className="text-sm text-muted-foreground">
            {importName ? t('live.decision.imported', { name: importName }) : t('live.decision.importFormat')}
          </span>
          {importError && <p className="w-full text-sm text-destructive">{importError}</p>}
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

      {result && <DecisionResultCard result={result} paperOrderRequested={submitPaperOrder} />}
    </div>
  )
}

/** Render the five observable stages of the local demonstration workflow. */
function DemoSteps({
  planReady,
  signalReady,
  result,
  paperOrderRequested,
}: {
  planReady: boolean
  signalReady: boolean
  result: DecisionPreviewResponse | null
  paperOrderRequested: boolean
}) {
  const { t } = useTranslation()
  const steps = [
    [t('live.demoSteps.plan'), planReady],
    [t('live.demoSteps.signals'), signalReady],
    [t('live.demoSteps.decision'), result !== null],
    [t('live.demoSteps.buckets'), result?.execution.bucket_split !== undefined],
    [t('live.demoSteps.paper'), paperOrderRequested ? result?.paper_order_ack !== undefined : false],
  ] as const
  return (
    <ol className="grid gap-2 sm:grid-cols-2 lg:grid-cols-5">
      {steps.map(([label, complete], index) => (
        <li
          key={label}
          className={cn(
            'rounded-lg border px-3 py-2 text-xs font-medium',
            complete ? 'border-semantic-positive/40 bg-semantic-positive/10' : 'bg-muted/40 text-muted-foreground',
          )}
        >
          {index + 1}. {label}
        </li>
      ))}
    </ol>
  )
}

/** Render one controlled plan field used by the dashboard's creation step. */
function DemoPlanField({
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

/** Render a non-fabricated Decision Preview response and paper-order outcome. */
function DecisionResultCard({
  result,
  paperOrderRequested,
}: {
  result: DecisionPreviewResponse
  paperOrderRequested: boolean
}) {
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
            value={
              result.decision.sentiment_score === undefined
                ? t('live.decision.fallback')
                : `${result.decision.sentiment_score.toFixed(2)} · ${t('live.decision.qwenAvailable')}`
            }
          />
        </div>
        {result.execution.bucket_split && (
          <div className="rounded-lg border bg-muted/30 p-3 text-sm">
            {t('live.decision.bucketSplit')}: {t('live.decision.coreBucket')} {result.execution.bucket_split.core_contribution} {result.execution.currency}
            {' · '}{t('live.decision.opportunityBucket')} {result.execution.bucket_split.opportunity_contribution}{' '}
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
        {paperOrderRequested && !result.paper_order_ack && (
          <div className="rounded-lg border border-muted-foreground/30 bg-muted/50 p-3 text-sm text-muted-foreground">
            {t('live.decision.paperNotSubmitted')}
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
