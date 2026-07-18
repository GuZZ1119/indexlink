import { ArrowRight, History } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { useLatestDecision } from '@/api/queries'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { Separator } from '@/components/ui/separator'
import { Skeleton } from '@/components/ui/skeleton'
import { appLanguage } from '@/i18n'
import {
  actionBadgeClass,
  formatCurrency,
  formatMultiplier,
  formatPrice,
} from '@/lib/decision'
import { cn } from '@/lib/utils'

export function LatestDecisionCard() {
  const { t, i18n } = useTranslation()
  const { data, isPending } = useLatestDecision()
  const lang = appLanguage(i18n.language)

  return (
    <Card className="flex flex-col">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <History className="size-4 text-muted-foreground" />
          {t('dashboard.latest.title')}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1">
        {isPending || !data ? (
          <div className="space-y-3">
            <Skeleton className="h-8 w-2/3" />
            <Skeleton className="h-24 w-full" />
          </div>
        ) : (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="font-mono text-sm font-semibold">{data.symbol}</span>
                <Badge className={cn(actionBadgeClass[data.action])}>
                  {t(`action.${data.action}`)}
                </Badge>
              </div>
              <span className="text-xs text-muted-foreground">
                {new Date(data.decidedAt).toLocaleDateString(i18n.language, {
                  year: 'numeric',
                  month: 'short',
                  day: 'numeric',
                })}
              </span>
            </div>

            <div className="grid grid-cols-2 gap-2 text-center sm:grid-cols-4 lg:grid-cols-2 xl:grid-cols-4">
              <div className="rounded-lg bg-muted/60 p-2">
                <div className="text-xs text-muted-foreground">
                  {t('dashboard.latest.baseAmount')}
                </div>
                <div className="mt-0.5 text-sm font-semibold tabular-nums">
                  {formatCurrency(data.baseAmount, data.currency)}
                </div>
              </div>
              <div className="rounded-lg bg-muted/60 p-2">
                <div className="text-xs text-muted-foreground">
                  {t('dashboard.latest.multiplier')}
                </div>
                <div className="mt-0.5 text-sm font-semibold tabular-nums">
                  {formatMultiplier(data.multiplier)}
                </div>
              </div>
              <div className="rounded-lg bg-muted/60 p-2">
                <div className="text-xs text-muted-foreground">
                  {t('dashboard.latest.executionPrice')}
                </div>
                <div className="mt-0.5 text-sm font-semibold tabular-nums">
                  {formatPrice(data.executionPrice, data.currency)}
                </div>
              </div>
              <div className="rounded-lg bg-muted/60 p-2">
                <div className="text-xs text-muted-foreground">
                  {t('dashboard.latest.amount')}
                </div>
                <div className="mt-0.5 text-sm font-semibold tabular-nums">
                  {formatCurrency(data.executedAmount, data.currency)}
                </div>
              </div>
            </div>

            <Separator />
            <p className="text-sm leading-relaxed text-muted-foreground">
              {data.summary[lang]}
            </p>
          </div>
        )}
      </CardContent>
      <CardFooter>
        <Button asChild variant="outline" size="sm" className="w-full">
          <Link to={data ? `/decisions/${data.id}` : '/decisions'}>
            {t('dashboard.latest.viewDetail')}
            <ArrowRight className="size-4" />
          </Link>
        </Button>
      </CardFooter>
    </Card>
  )
}
