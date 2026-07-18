import { useState } from 'react'
import { CircleHelp, Gauge } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Bar, BarChart, CartesianGrid, Cell, LabelList, XAxis, YAxis } from 'recharts'

import { useMarketOverview } from '@/api/queries'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardAction,
  CardContent,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from '@/components/ui/chart'
import { Skeleton } from '@/components/ui/skeleton'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { appLanguage } from '@/i18n'
import { actionBadgeClass, formatCurrency, formatMultiplier } from '@/lib/decision'
import { cn } from '@/lib/utils'

function percentileColor(percentile: number): string {
  if (percentile >= 80) return 'var(--percentile-extreme)'
  if (percentile >= 60) return 'var(--percentile-high)'
  if (percentile >= 40) return 'var(--percentile-neutral)'
  return 'var(--percentile-low)'
}

export function ValuationCard() {
  const { t, i18n } = useTranslation()
  const { data, isPending } = useMarketOverview()
  const [showPercentiles, setShowPercentiles] = useState(false)
  const [renderPercentiles, setRenderPercentiles] = useState(false)
  const lang = appLanguage(i18n.language)
  const chartConfig = {
    percentile: {
      label: t('dashboard.valuation.composite'),
      color: 'var(--percentile-neutral)',
    },
  } satisfies ChartConfig
  const percentileData =
    data?.metrics.map((metric) => ({
      key: metric.key,
      label: t(`dashboard.valuation.metrics.${metric.key}`),
      description: t(`dashboard.valuation.metricDescriptions.${metric.key}`),
      percentile: metric.percentile,
      fill: percentileColor(metric.percentile),
    })) ?? []

  const expandPercentiles = () => {
    setRenderPercentiles(true)
    window.requestAnimationFrame(() => setShowPercentiles(true))
  }

  const collapsePercentiles = () => {
    setShowPercentiles(false)
  }

  return (
    <Card className="self-start lg:col-span-2">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Gauge className="size-4 text-muted-foreground" />
          {t('dashboard.valuation.title')}
        </CardTitle>
        {!renderPercentiles && !isPending && data && (
          <CardAction>
            <button
              type="button"
              onClick={expandPercentiles}
              className="cursor-pointer text-sm text-muted-foreground underline underline-offset-4 transition-colors hover:text-foreground"
            >
              {t('dashboard.valuation.why')}
            </button>
          </CardAction>
        )}
      </CardHeader>
      <CardContent>
        {isPending || !data ? (
          <div className="space-y-3">
            <Skeleton className="h-16 w-full" />
            <Skeleton className="h-40 w-full" />
          </div>
        ) : (
          <div className="space-y-6">
            <div className="flex flex-wrap items-end justify-between gap-4">
              <div>
                <div className="font-mono text-sm text-muted-foreground">
                  {data.symbol} · {data.name[lang]}
                </div>
                <div className="mt-1 flex items-baseline gap-2">
                  <span className="text-4xl font-semibold tabular-nums">
                    {data.compositePercentile}
                    <span className="text-xl text-muted-foreground">%</span>
                  </span>
                  <span className="text-sm text-muted-foreground">
                    {t('dashboard.valuation.composite')}
                  </span>
                </div>
              </div>
              <div className="grid grid-cols-2 gap-12 text-right sm:grid-cols-4">
                <div className="text-right">
                  <div className="text-xs text-muted-foreground">
                    {t('dashboard.valuation.suggestedAction')}
                  </div>
                  <Badge
                    className={cn('mt-1 h-6 px-3 text-sm', actionBadgeClass[data.suggestedAction])}
                  >
                    {t(`action.${data.suggestedAction}`)}
                  </Badge>
                </div>
                <div className="text-right">
                  <div className="text-xs text-muted-foreground">
                    {t('dashboard.valuation.multiplier')}
                  </div>
                  <div className="mt-1 text-xl font-semibold tabular-nums">
                    {formatMultiplier(data.suggestedMultiplier)}
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-xs text-muted-foreground">
                    {t('dashboard.valuation.expectedAmount')}
                  </div>
                  <div className="mt-1 text-xl font-semibold tabular-nums">
                    {formatCurrency(
                      data.baseDcaAmount * data.suggestedMultiplier,
                      data.currency,
                    )}
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-xs text-muted-foreground">
                    {t('dashboard.valuation.nextDcaTime')}
                  </div>
                  <div className="mt-1 text-xl font-semibold tabular-nums">
                    {new Date(data.nextDcaTime).toLocaleString(i18n.language, {
                      month: 'short',
                      day: 'numeric',
                      hour: '2-digit',
                      minute: '2-digit',
                    })}
                  </div>
                </div>
              </div>
            </div>

            {renderPercentiles && (
              <div
                className={cn(
                  'grid transition-[grid-template-rows,opacity] duration-300 ease-out',
                  showPercentiles ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0',
                )}
                onTransitionEnd={(event) => {
                  if (event.propertyName === 'grid-template-rows' && !showPercentiles) {
                    setRenderPercentiles(false)
                  }
                }}
              >
                <div className="min-h-0 overflow-hidden">
                  <div className="rounded-xl bg-muted/20 px-2 pb-3 pt-5">
                    <ChartContainer config={chartConfig} className="h-48 w-full">
                      <BarChart
                        data={percentileData}
                        margin={{ top: 20, right: 8, left: 0, bottom: 0 }}
                      >
                        <CartesianGrid vertical={false} strokeDasharray="3 3" />
                        <XAxis
                          dataKey="label"
                          tick={false}
                          tickLine={false}
                          axisLine={false}
                          height={8}
                        />
                        <YAxis
                          domain={[0, 100]}
                          ticks={[0, 25, 50, 75, 100]}
                          tickLine={false}
                          axisLine={false}
                          width={36}
                          tickFormatter={(value: number) => `${value}%`}
                        />
                        <ChartTooltip
                          cursor={false}
                          content={
                            <ChartTooltipContent
                              hideLabel
                              formatter={(value, _name, _item, _index, payload) => {
                                const label =
                                  typeof payload === 'object' &&
                                  payload !== null &&
                                  'label' in payload &&
                                  typeof payload.label === 'string'
                                    ? payload.label
                                    : t('dashboard.valuation.composite')
                                const formattedValue =
                                  typeof value === 'number' ? `${value}%` : String(value ?? '')

                                return (
                                  <div className="flex min-w-40 flex-1 items-center justify-between gap-4">
                                    <span className="font-medium text-foreground">{label}</span>
                                    <span className="font-mono font-medium text-foreground tabular-nums">
                                      {formattedValue}
                                    </span>
                                  </div>
                                )
                              }}
                            />
                          }
                        />
                        <Bar
                          dataKey="percentile"
                          fill="var(--percentile-neutral)"
                          radius={[6, 6, 0, 0]}
                          maxBarSize={48}
                        >
                          <LabelList
                            dataKey="percentile"
                            position="top"
                            className="fill-foreground font-mono text-xs font-semibold"
                            formatter={(value) => (typeof value === 'number' ? `${value}%` : '')}
                          />
                          {percentileData.map((metric) => (
                            <Cell key={metric.key} fill={metric.fill} />
                          ))}
                        </Bar>
                      </BarChart>
                    </ChartContainer>
                    <div className="grid grid-cols-5 gap-2 pl-10 pr-2 pt-1">
                      {percentileData.map((metric) => (
                        <div
                          key={metric.key}
                          className="flex min-w-0 items-center justify-center gap-1"
                        >
                          <span className="truncate text-xs text-muted-foreground">
                            {metric.label}
                          </span>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <button
                                type="button"
                                className="shrink-0 rounded-full text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
                                aria-label={`${metric.label} description`}
                              >
                                <CircleHelp className="size-3.5" />
                              </button>
                            </TooltipTrigger>
                            <TooltipContent
                              side="top"
                              className="max-w-72 items-start border border-border/50 bg-background text-xs leading-relaxed text-muted-foreground shadow-xl [&>svg]:!bg-background [&>svg]:!fill-background"
                            >
                              {metric.description}
                            </TooltipContent>
                          </Tooltip>
                        </div>
                      ))}
                    </div>
                    <div className="mt-3 flex justify-center">
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={collapsePercentiles}
                      >
                        {t('dashboard.valuation.collapse')}
                      </Button>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
