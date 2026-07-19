import { useEffect, useMemo, useRef, useState, type FormEvent, type ReactNode } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { BarChart3, Bot, ClipboardCheck, FileJson, Plus, RefreshCw, Send, Upload } from 'lucide-react'
import { useSnapshot } from 'valtio'

import {
  ApiRequestError,
  fetchMarketSignalInput,
  fetchPaperPortfolio,
  previewDecision,
  previewFundamental,
  previewTrend,
  useCreatePlan,
  useDecisionRecords,
  usePlans,
} from '@/api/queries'
import type {
  CreateInvestmentPlanRequest,
  DecisionRecord,
  DecisionResult,
  DecisionPreviewResponse,
  InvestmentPlan,
  MarketSignalInput,
  PaperPortfolioSnapshot,
} from '@/api/types'
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

type OverviewDecision = {
  createdAt: string
  decision: DecisionResult
  executionStatus: 'due' | 'waiting' | 'inactive'
  plannedContribution?: string
  summary: string
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
  const { data: decisionRecords = [], error: decisionRecordsError } = useDecisionRecords(selectedPlan?.id ?? null)

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
  const paperPortfolioMutation = useMutation({
    mutationFn: fetchPaperPortfolio,
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
  const error = marketRefreshMutation.error
    ?? decisionMutation.error
    ?? paperPortfolioMutation.error
    ?? createPlan.error
    ?? plansError
    ?? decisionRecordsError
  const hasSignalInput = Object.values(signals).every((value) => value.trim().length > 0)
  const overviewDecision = useMemo(
    () => latestOverviewDecision(result, decisionRecords[0]),
    [decisionRecords, result],
  )

  return (
    <div className="mx-auto flex w-full max-w-[1600px] flex-col gap-4 p-4 lg:p-6">
      <DashboardOverview
        plan={selectedPlan}
        decision={overviewDecision}
        marketRefresh={marketRefresh}
        portfolio={paperPortfolioMutation.data ?? null}
        portfolioRefreshing={paperPortfolioMutation.isPending}
        onRefreshPortfolio={() => paperPortfolioMutation.mutate()}
      />

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

/** Prefer the just-returned preview, then fall back to the latest persisted audit record. */
function latestOverviewDecision(
  preview: DecisionPreviewResponse | null,
  record: DecisionRecord | undefined,
): OverviewDecision | null {
  if (preview) {
    return {
      createdAt: new Date().toISOString(),
      decision: preview.decision,
      executionStatus: preview.execution.status,
      plannedContribution: preview.execution.planned_contribution,
      summary: preview.summary,
    }
  }
  if (!record) {
    return null
  }
  return {
    createdAt: record.created_at,
    decision: record.decision_snapshot,
    executionStatus: record.execution_status,
    plannedContribution: record.planned_contribution,
    summary: record.summary,
  }
}

/** Render the restored dashboard layout with only source-backed decision data. */
function DashboardOverview({
  plan,
  decision,
  marketRefresh,
  portfolio,
  portfolioRefreshing,
  onRefreshPortfolio,
}: {
  plan: InvestmentPlan | null
  decision: OverviewDecision | null
  marketRefresh: MarketSignalInput | null
  portfolio: PaperPortfolioSnapshot | null
  portfolioRefreshing: boolean
  onRefreshPortfolio: () => void
}) {
  const { t } = useTranslation()
  const currency = plan?.currency ?? 'USD'
  const signalValues: Array<[string, number]> = marketRefresh
    ? [
        [t('dashboard.valuation.metrics.cape'), marketRefresh.fundamental.cape_current],
        [t('dashboard.valuation.metrics.erp'), marketRefresh.fundamental.erp_current],
        [t('dashboard.valuation.metrics.ma200'), marketRefresh.trend.ma_distance_current],
        [t('dashboard.valuation.metrics.rsi'), marketRefresh.trend.rsi_current],
        [t('dashboard.valuation.metrics.vix'), marketRefresh.trend.vix_current],
      ]
    : []
  const decisionScore = decision ? toScore(decision.decision.final_score) : null
  const action = decision?.decision.action

  return (
    <section className="space-y-4" aria-label={t('dashboard.overview.title')}>
      <Card className="border-primary/30 bg-linear-to-br from-primary/8 via-card to-card shadow-sm">
        <CardHeader className="gap-4 lg:grid-cols-[minmax(0,1fr)_auto]">
          <div>
            <CardTitle className="flex items-center gap-2 text-lg">
              <BarChart3 className="size-5 text-primary" />
              {t('dashboard.overview.title')}
            </CardTitle>
            <CardDescription>{t('dashboard.overview.description')}</CardDescription>
          </div>
          {plan ? (
            <div className="rounded-lg border bg-background/80 px-3 py-2 text-sm">
              <span className="font-semibold">{plan.symbol}</span>
              <span className="ml-2 text-muted-foreground">{plan.name}</span>
            </div>
          ) : (
            <div className="rounded-lg border border-dashed px-3 py-2 text-sm text-muted-foreground">
              {t('dashboard.overview.selectPlan')}
            </div>
          )}
        </CardHeader>
        <CardContent className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
          <div>
            <div className="text-4xl font-semibold tracking-tight">
              {decisionScore === null ? '—' : `${decisionScore} / 100`}
            </div>
            <p className="mt-1 text-sm text-muted-foreground">
              {decision
                ? t('dashboard.overview.latestScore', { date: formatLocalDate(decision.createdAt) })
                : t('dashboard.overview.awaitingDecision')}
            </p>
          </div>
          <div className="grid grid-cols-2 gap-x-8 gap-y-3 text-sm sm:grid-cols-4">
            <OverviewFact
              label={t('dashboard.valuation.suggestedAction')}
              value={action ? <Badge className={actionBadgeClass[action]}>{t(`action.${action}`)}</Badge> : '—'}
            />
            <OverviewFact
              label={t('dashboard.valuation.multiplier')}
              value={decision ? formatMultiplier(decision.decision.multiplier) : '—'}
            />
            <OverviewFact
              label={t('dashboard.valuation.expectedAmount')}
              value={decision?.plannedContribution
                ? formatCurrency(Number(decision.plannedContribution), currency)
                : '—'}
            />
            <OverviewFact
              label={t('dashboard.overview.schedule')}
              value={plan ? t('dashboard.overview.monthlyDay', { day: plan.schedule_day }) : '—'}
            />
          </div>
        </CardContent>
      </Card>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,2fr)_minmax(320px,1fr)]">
        <div className="space-y-4">
          <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <ScoreCard
              title={t('dashboard.scores.fundamental')}
              weight={70}
              value={decision ? toScore(decision.decision.fundamental_score) : null}
            />
            <ScoreCard
              title={t('dashboard.scores.trend')}
              weight={20}
              value={decision ? toScore(decision.decision.trend_score) : null}
            />
            <ScoreCard
              title={t('dashboard.scores.sentiment')}
              weight={10}
              value={decision?.decision.sentiment_score === undefined
                ? null
                : toScore(decision.decision.sentiment_score)}
            />
            <ScoreCard
              title={t('dashboard.scores.composite')}
              weight={null}
              value={decision ? toScore(decision.decision.final_score) : null}
              emphasize
            />
          </div>

          <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <PortfolioMetric
              label={t('dashboard.portfolio.totalAssets')}
              value={portfolio ? formatCurrency(Number(portfolio.total_assets), portfolio.currency) : null}
            />
            <PortfolioMetric
              label={t('dashboard.portfolio.cash')}
              value={portfolio ? formatCurrency(Number(portfolio.cash), portfolio.currency) : null}
            />
            <PortfolioMetric
              label={t('dashboard.portfolio.positionPnl')}
              value={portfolio
                ? formatCurrency(sumPositionPnl(portfolio), portfolio.currency)
                : null}
            />
            <PortfolioMetric
              label={t('dashboard.portfolio.marketValue')}
              value={portfolio ? formatCurrency(Number(portfolio.market_value), portfolio.currency) : null}
            />
          </div>

          <Card className="min-h-80">
            <CardHeader>
              <CardTitle>{t('dashboard.chart.title')}</CardTitle>
              <CardDescription>{t('dashboard.chart.subtitle')}</CardDescription>
            </CardHeader>
            <CardContent className="flex min-h-56 items-center justify-center rounded-lg border border-dashed bg-muted/20">
              <div className="max-w-md space-y-2 px-6 text-center">
                <p className="font-medium">{t('dashboard.emptyPerformance.title')}</p>
                <p className="text-sm leading-relaxed text-muted-foreground">
                  {t('dashboard.emptyPerformance.description')}
                </p>
              </div>
            </CardContent>
          </Card>
        </div>

        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>{t('dashboard.latest.title')}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {decision ? (
                <>
                  <div className="flex items-center justify-between gap-3">
                    <span className="font-semibold">{plan?.symbol ?? '—'}</span>
                    {action && <Badge className={actionBadgeClass[action]}>{t(`action.${action}`)}</Badge>}
                  </div>
                  <div className="grid grid-cols-2 gap-2 text-sm">
                    <OverviewFact label={t('dashboard.latest.amount')} value={decision.plannedContribution
                      ? formatCurrency(Number(decision.plannedContribution), currency)
                      : '—'} />
                    <OverviewFact label={t('dashboard.latest.multiplier')} value={formatMultiplier(decision.decision.multiplier)} />
                  </div>
                  <p className="border-t pt-3 text-sm leading-relaxed text-muted-foreground">{decision.summary}</p>
                </>
              ) : (
                <EmptyState text={t('dashboard.latest.empty')} />
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>{t('dashboard.risk.title')}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {decision ? <RiskNotices decision={decision} /> : <EmptyState text={t('dashboard.risk.empty')} />}
            </CardContent>
          </Card>
        </div>
      </div>

      <Card className="border-dashed bg-muted/20">
        <CardHeader>
          <CardTitle>{t('dashboard.marketSnapshot.title')}</CardTitle>
          <CardDescription>{t('dashboard.marketSnapshot.description')}</CardDescription>
        </CardHeader>
        <CardContent>
          {marketRefresh ? (
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
              {signalValues.map(([label, value]) => (
                <OverviewFact key={label} label={label} value={Number(value).toFixed(2)} />
              ))}
            </div>
          ) : (
            <EmptyState text={t('dashboard.marketSnapshot.empty')} />
          )}
        </CardContent>
      </Card>

      <Card className="border-primary/30">
        <CardHeader className="gap-3 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <CardTitle>{t('dashboard.portfolio.title')}</CardTitle>
            <CardDescription>{t('dashboard.portfolio.description')}</CardDescription>
          </div>
          <Button size="lg" disabled={portfolioRefreshing} onClick={onRefreshPortfolio}>
            <RefreshCw className={cn('size-4', portfolioRefreshing && 'animate-spin')} />
            {portfolioRefreshing ? t('dashboard.portfolio.refreshing') : t('dashboard.portfolio.refresh')}
          </Button>
        </CardHeader>
        <CardContent>
          {portfolio ? <PaperPortfolioDetails portfolio={portfolio} /> : <EmptyState text={t('dashboard.portfolio.empty')} />}
        </CardContent>
      </Card>
    </section>
  )
}

/** Render a small label-value fact without claiming unavailable data is zero. */
function OverviewFact({ label, value }: { label: string; value: ReactNode }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <div className="mt-1 min-h-5 text-sm font-semibold">{value}</div>
    </div>
  )
}

/** Render a 70/20/10 decision score sourced from the latest decision record. */
function ScoreCard({
  title,
  weight,
  value,
  emphasize = false,
}: {
  title: string
  weight: number | null
  value: number | null
  emphasize?: boolean
}) {
  const { t } = useTranslation()
  const progress = value ?? 0
  return (
    <Card className={emphasize ? 'border-primary/40 bg-primary/5' : undefined} size="sm">
      <CardHeader className="flex-row items-center justify-between gap-2">
        <CardTitle>{title}</CardTitle>
        {weight !== null && <CardDescription>{t('dashboard.scores.weight', { value: weight })}</CardDescription>}
      </CardHeader>
      <CardContent>
        <div className="text-3xl font-semibold">{value === null ? '—' : value}<span className="ml-1 text-base font-normal text-muted-foreground">/ 100</span></div>
        <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-muted">
          <div className="h-full rounded-full bg-foreground transition-all" style={{ width: `${progress}%` }} />
        </div>
      </CardContent>
    </Card>
  )
}

/** Render a financial metric that awaits real fills and portfolio accounting. */
function PortfolioMetric({ label, value }: { label: string; value: string | null }) {
  const { t } = useTranslation()
  return (
    <Card size="sm">
      <CardHeader><CardTitle>{label}</CardTitle></CardHeader>
      <CardContent>
        <div className="text-3xl font-semibold">{value ?? '—'}</div>
        {value === null && <p className="mt-2 text-sm text-muted-foreground">{t('dashboard.unavailable.awaitingFill')}</p>}
      </CardContent>
    </Card>
  )
}

/** Render the provider-backed paper positions and recent order states. */
function PaperPortfolioDetails({ portfolio }: { portfolio: PaperPortfolioSnapshot }) {
  const { t } = useTranslation()
  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <div>
        <h3 className="mb-2 text-sm font-semibold">{t('dashboard.portfolio.positions')}</h3>
        {portfolio.positions.length === 0 ? <EmptyState text={t('dashboard.portfolio.noPositions')} /> : (
          <div className="space-y-2">
            {portfolio.positions.map((position) => (
              <div key={position.symbol} className="grid grid-cols-2 gap-2 rounded-lg border p-3 text-sm sm:grid-cols-4">
                <span className="font-semibold">{position.symbol}</span>
                <span>{position.quantity} {t('dashboard.portfolio.shares')}</span>
                <span>{formatCurrency(Number(position.market_value), portfolio.currency)}</span>
                <span className={Number(position.unrealized_pnl) < 0 ? 'text-destructive' : 'text-semantic-positive'}>
                  {formatCurrency(Number(position.unrealized_pnl), portfolio.currency)}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
      <div>
        <h3 className="mb-2 text-sm font-semibold">{t('dashboard.portfolio.orders')}</h3>
        {portfolio.orders.length === 0 ? <EmptyState text={t('dashboard.portfolio.noOrders')} /> : (
          <div className="space-y-2">
            {portfolio.orders.slice(0, 8).map((order) => (
              <div key={order.order_id} className="grid grid-cols-2 gap-2 rounded-lg border p-3 text-sm sm:grid-cols-4">
                <span className="font-semibold">{order.symbol}</span>
                <span>{t(`dashboard.portfolio.side.${order.side}`)}</span>
                <span>{order.filled_quantity} / {order.quantity}</span>
                <span className="font-mono text-xs text-muted-foreground">{t(`dashboard.portfolio.state.${order.state}`)}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

/** Sum OpenD's position-level unrealized P&L without inventing realized P&L. */
function sumPositionPnl(portfolio: PaperPortfolioSnapshot): number {
  return portfolio.positions.reduce((total, position) => total + Number(position.unrealized_pnl), 0)
}

/** Render a clear empty-state instead of silently substituting fictional data. */
function EmptyState({ text }: { text: string }) {
  return <p className="rounded-lg border border-dashed bg-muted/20 p-3 text-sm text-muted-foreground">{text}</p>
}

/** Derive safe, user-facing notices only from the latest server decision. */
function RiskNotices({ decision }: { decision: OverviewDecision }) {
  const { t } = useTranslation()
  const notices = [
    ...(decision.decision.action === 'underweight' || decision.decision.action === 'skip' || decision.decision.action === 'tactical_delay'
      ? [t('dashboard.risk.reduced')]
      : []),
    t('dashboard.risk.percentile'),
    ...(decision.decision.sentiment_score === undefined ? [t('dashboard.risk.sentimentUnavailable')] : []),
  ]
  return (
    <ul className="space-y-2">
      {notices.map((notice) => (
        <li key={notice} className="rounded-lg border bg-muted/20 p-3 text-sm leading-relaxed text-muted-foreground">
          {notice}
        </li>
      ))}
    </ul>
  )
}

/** Convert a bounded engine score into the dashboard's 0–100 presentation. */
function toScore(value: number): number {
  return Math.round(value * 100)
}

/** Format a persisted UTC timestamp for a local overview label. */
function formatLocalDate(value: string): string {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString()
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
