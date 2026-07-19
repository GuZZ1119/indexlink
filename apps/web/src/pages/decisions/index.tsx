import { History } from 'lucide-react'
import { Link, useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { useSnapshot } from 'valtio'

import { useDecisionRecord, useDecisionRecords, usePlans } from '@/api/queries'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { actionBadgeClass } from '@/lib/decision'
import { cn } from '@/lib/utils'
import { setSelectedPlanId, uiStore } from '@/stores/ui'
import type { PersistedMarketSentimentSnapshot } from '@/api/types'

/** Display real persisted decision history and detail snapshots from the Rust API. */
export default function DecisionsPage() {
  const { t } = useTranslation()
  const { id } = useParams()
  const { selectedPlanId } = useSnapshot(uiStore)
  const { data: plans = [] } = usePlans()
  const history = useDecisionRecords(selectedPlanId)
  const record = useDecisionRecord(id ?? null)

  if (id) {
    if (record.isPending) {
      return <PageMessage message={t('live.history.loadRecord')} />
    }
    if (record.error || !record.data) {
      return <PageMessage message={record.error instanceof Error ? record.error.message : 'record unavailable'} />
    }
    const decision = record.data.decision_snapshot
    return (
      <div className="mx-auto w-full max-w-4xl p-4 lg:p-6">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              {record.data.symbol}
              <Badge className={cn(actionBadgeClass[decision.action])}>{decision.action}</Badge>
            </CardTitle>
            <CardDescription>{new Date(record.data.created_at).toLocaleString()}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <p className="rounded-lg bg-muted/50 p-3 text-sm leading-relaxed">{record.data.summary}</p>
            <Snapshot title={t('live.history.fundamental')} value={record.data.fundamental_snapshot} />
            <Snapshot title={t('live.history.trend')} value={record.data.trend_snapshot} />
            <Snapshot title={t('live.history.decision')} value={record.data.decision_snapshot} />
            {record.data.sentiment_snapshot && <SentimentEvidence value={record.data.sentiment_snapshot} />}
            {record.data.broker_order_ack && <Snapshot title={t('live.history.paperAck')} value={record.data.broker_order_ack} />}
          </CardContent>
        </Card>
      </div>
    )
  }

  if (!selectedPlanId) {
    return (
      <div className="mx-auto w-full max-w-4xl p-4 lg:p-6">
        <Card>
          <CardHeader>
            <CardTitle>{t('live.history.title')}</CardTitle>
            <CardDescription>{t('live.history.selectPlan')}</CardDescription>
          </CardHeader>
          <CardContent>
            <select
              className="h-8 w-full rounded-lg border border-input bg-transparent px-2.5 text-sm"
              defaultValue=""
              onChange={(event) => setSelectedPlanId(event.target.value || null)}
            >
              <option value="">{t('live.decision.createPlanFirst')}</option>
              {plans.map((plan) => (
                <option key={plan.id} value={plan.id}>
                  {plan.name} · {plan.symbol}
                </option>
              ))}
            </select>
          </CardContent>
        </Card>
      </div>
    )
  }
  if (history.isPending) {
    return <PageMessage message={t('live.history.loading')} />
  }
  if (history.error) {
    return <PageMessage message={history.error instanceof Error ? history.error.message : 'history unavailable'} />
  }
  if (!history.data?.length) {
    return <PageMessage message={t('live.history.empty')} />
  }
  return (
    <div className="mx-auto w-full max-w-4xl p-4 lg:p-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <History className="size-4 text-muted-foreground" />
            {t('live.history.title')}
          </CardTitle>
          <CardDescription>{t('live.history.description')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {history.data.map((item) => {
            const decision = item.decision_snapshot
            return (
              <Link key={item.id} to={`/decisions/${item.id}`} className="block rounded-lg border p-4 transition-colors hover:bg-muted/50">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="font-mono font-semibold">{item.symbol}</span>
                  <Badge className={cn(actionBadgeClass[decision.action])}>{decision.action}</Badge>
                  <span className="text-xs text-muted-foreground">{new Date(item.created_at).toLocaleString()}</span>
                </div>
                <p className="mt-2 line-clamp-2 text-sm text-muted-foreground">{item.summary}</p>
                {item.broker_order_ack && (
                  <p className="mt-2 text-xs text-semantic-positive">
                    {t('live.history.paperAck')}: {item.broker_order_ack.status} · {item.broker_order_ack.order_id}
                  </p>
                )}
              </Link>
            )
          })}
        </CardContent>
      </Card>
    </div>
  )
}

/** Render saved Qwen reasoning as readable audit evidence instead of a raw JSON blob. */
function SentimentEvidence({ value }: { value: PersistedMarketSentimentSnapshot }) {
  const { t } = useTranslation()
  const evidence = typeof value.rationale === 'string'
    && Array.isArray(value.warnings)
    && Array.isArray(value.headlines)
    ? { rationale: value.rationale, warnings: value.warnings, headlines: value.headlines }
    : null
  return (
    <section className="space-y-3 rounded-lg border bg-muted/20 p-4 text-sm">
      <h2 className="font-semibold">{t('live.history.sentiment')}</h2>
      {!evidence && <p className="text-muted-foreground">{t('dashboard.decisionExplanation.aiLegacySource')}</p>}
      {evidence && <p className="text-muted-foreground">{evidence.rationale}</p>}
      {evidence && evidence.warnings.length > 0 && (
        <div>
          <p className="font-medium">{t('dashboard.decisionExplanation.aiWarnings')}</p>
          <ul className="mt-1 list-disc space-y-1 pl-5 text-muted-foreground">
            {evidence.warnings.map((warning) => <li key={warning}>{warning}</li>)}
          </ul>
        </div>
      )}
      {evidence && <div>
        <p className="font-medium">{t('dashboard.decisionExplanation.aiHeadlines')}</p>
        <ul className="mt-1 space-y-1 text-muted-foreground">
          {evidence.headlines.map((headline) => (
            <li key={`${headline.published_at}-${headline.title}`}>
              {headline.url ? (
                <a className="underline-offset-4 hover:underline" href={headline.url} rel="noreferrer" target="_blank">
                  {headline.title}
                </a>
              ) : headline.title}
              <span className="ml-2 text-xs">{new Date(headline.published_at).toLocaleString()}</span>
            </li>
          ))}
        </ul>
      </div>}
    </section>
  )
}

/** Render a concise empty, loading, or error state without mock content. */
function PageMessage({ message }: { message: string }) {
  return <div className="p-6 text-sm text-muted-foreground">{message}</div>
}

/** Render one trusted JSON snapshot returned by the API. */
function Snapshot({ title, value }: { title: string; value: unknown }) {
  return (
    <section>
      <h2 className="mb-2 text-sm font-semibold">{title}</h2>
      <pre className="overflow-x-auto rounded-lg bg-muted/50 p-3 text-xs leading-relaxed">
        {JSON.stringify(value, null, 2)}
      </pre>
    </section>
  )
}
