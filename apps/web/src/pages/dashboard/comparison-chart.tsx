import { ChartSpline } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts'
import { useSnapshot } from 'valtio'

import { useComparisonSeries } from '@/api/queries'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import {
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from '@/components/ui/chart'
import { Skeleton } from '@/components/ui/skeleton'
import { setChartRange, uiStore, type ChartRange } from '@/stores/ui'

const RANGES: { key: ChartRange; months: number | null }[] = [
  { key: 'y1', months: 12 },
  { key: 'y3', months: 36 },
  { key: 'all', months: null },
]

export function ComparisonChart() {
  const { t } = useTranslation()
  const { data, isPending } = useComparisonSeries()
  const { chartRange } = useSnapshot(uiStore)

  const chartConfig = {
    dca: { label: t('dashboard.chart.dca'), color: 'var(--chart-dca)' },
    adaptive: { label: t('dashboard.chart.adaptive'), color: 'var(--chart-adaptive)' },
  } satisfies ChartConfig

  const months = RANGES.find((r) => r.key === chartRange)?.months ?? null
  const series = data ? (months ? data.slice(-months) : data) : []

  return (
    <Card className="lg:col-span-2">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ChartSpline className="size-4 text-muted-foreground" />
          {t('dashboard.chart.title')}
        </CardTitle>
        <CardDescription>{t('dashboard.chart.subtitle')}</CardDescription>
        <CardAction>
          <div className="flex gap-1">
            {RANGES.map((range) => (
              <Button
                key={range.key}
                size="sm"
                variant={chartRange === range.key ? 'secondary' : 'ghost'}
                onClick={() => setChartRange(range.key)}
              >
                {t(`dashboard.chart.range.${range.key}`)}
              </Button>
            ))}
          </div>
        </CardAction>
      </CardHeader>
      <CardContent>
        {isPending || !data ? (
          <Skeleton className="h-72 w-full" />
        ) : (
          <ChartContainer config={chartConfig} className="h-72 w-full">
            <AreaChart data={series} margin={{ left: 0, right: 8, top: 8 }}>
              <defs>
                <linearGradient id="fillAdaptive" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="var(--color-adaptive)" stopOpacity={0.4} />
                  <stop offset="95%" stopColor="var(--color-adaptive)" stopOpacity={0.05} />
                </linearGradient>
                <linearGradient id="fillDca" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="var(--color-dca)" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="var(--color-dca)" stopOpacity={0.05} />
                </linearGradient>
              </defs>
              <CartesianGrid vertical={false} strokeDasharray="3 3" />
              <XAxis
                dataKey="date"
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                minTickGap={40}
              />
              <YAxis
                tickLine={false}
                axisLine={false}
                width={44}
                tickFormatter={(v: number) => `${v}%`}
              />
              <ChartTooltip cursor={false} content={<ChartTooltipContent indicator="line" />} />
              <Area
                dataKey="dca"
                type="monotone"
                stroke="var(--color-dca)"
                fill="url(#fillDca)"
                strokeWidth={2}
              />
              <Area
                dataKey="adaptive"
                type="monotone"
                stroke="var(--color-adaptive)"
                fill="url(#fillAdaptive)"
                strokeWidth={2}
              />
              <ChartLegend content={<ChartLegendContent />} />
            </AreaChart>
          </ChartContainer>
        )}
      </CardContent>
    </Card>
  )
}
