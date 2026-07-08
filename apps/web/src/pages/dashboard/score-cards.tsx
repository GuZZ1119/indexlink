import { Bot, Landmark, Sigma, TrendingUp } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { useMarketOverview } from '@/api/queries'
import { Card, CardContent } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'

function scoreTone(score: number): string {
  if (score >= 60) return 'text-semantic-positive'
  if (score >= 40) return 'text-foreground'
  return 'text-semantic-caution'
}

export function ScoreCards() {
  const { t } = useTranslation()
  const { data, isPending } = useMarketOverview()

  const items = data
    ? ([
        {
          key: 'fundamental',
          icon: Landmark,
          score: data.scores.fundamental,
          weight: data.weights.fundamental,
        },
        {
          key: 'trend',
          icon: TrendingUp,
          score: data.scores.trend,
          weight: data.weights.trend,
        },
        {
          key: 'sentiment',
          icon: Bot,
          score: data.scores.sentiment,
          weight: data.weights.sentiment,
        },
        {
          key: 'composite',
          icon: Sigma,
          score: data.scores.composite,
          weight: null,
        },
      ] as const)
    : []

  return (
    <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
      {isPending || !data
        ? Array.from({ length: 4 }).map((_, i) => (
            <Skeleton key={i} className="h-28 w-full rounded-xl" />
          ))
        : items.map((item) => (
            <Card key={item.key} className="py-4">
              <CardContent className="px-4">
                <div className="flex items-center justify-between text-sm text-muted-foreground">
                  <span className="flex items-center gap-1.5">
                    <item.icon className="size-4" />
                    {t(`dashboard.scores.${item.key}`)}
                  </span>
                  {item.weight !== null && (
                    <span className="text-xs">
                      {t('dashboard.scores.weight', { value: item.weight })}
                    </span>
                  )}
                </div>
                <div className={cn('mt-2 text-3xl font-semibold tabular-nums', scoreTone(item.score))}>
                  {item.score}
                  <span className="text-base font-normal text-muted-foreground"> / 100</span>
                </div>
                <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-muted">
                  <div
                    className="h-full rounded-full bg-primary"
                    style={{ width: `${item.score}%` }}
                  />
                </div>
              </CardContent>
            </Card>
          ))}
    </div>
  )
}
