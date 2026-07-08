import { Banknote, CircleDollarSign, PiggyBank, Wallet } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { usePortfolioReturns } from '@/api/queries'
import { Card, CardContent } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { formatCurrency, formatSignedPct } from '@/lib/decision'
import { cn } from '@/lib/utils'

function pnlTone(value: number): string {
  return value >= 0
    ? 'text-semantic-positive'
    : 'text-semantic-negative'
}

export function ReturnsCards() {
  const { t } = useTranslation()
  const { data, isPending } = usePortfolioReturns()

  if (isPending || !data) {
    return (
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <Skeleton key={i} className="h-28 w-full rounded-xl" />
        ))}
      </div>
    )
  }

  const items = [
    {
      key: 'total',
      icon: CircleDollarSign,
      value: formatCurrency(data.totalReturn, data.currency),
      tone: pnlTone(data.totalReturn),
      sub: `${formatSignedPct(data.totalReturnPct)} · ${t('dashboard.returns.annualized', {
        value: formatSignedPct(data.annualizedPct),
      })}`,
    },
    {
      key: 'position',
      icon: Wallet,
      value: formatCurrency(data.positionReturn, data.currency),
      tone: pnlTone(data.positionReturn),
      sub: formatSignedPct(data.positionReturnPct),
    },
    {
      key: 'realized',
      icon: PiggyBank,
      value: formatCurrency(data.realizedReturn, data.currency),
      tone: pnlTone(data.realizedReturn),
      sub: t('dashboard.returns.vsDca', { value: formatSignedPct(data.vsDcaPct) }),
    },
    {
      key: 'invested',
      icon: Banknote,
      value: formatCurrency(data.invested, data.currency),
      tone: 'text-foreground',
      sub: null,
    },
  ] as const

  return (
    <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
      {items.map((item) => (
        <Card key={item.key} className="py-4">
          <CardContent className="px-4">
            <div className="flex items-center gap-1.5 text-sm text-muted-foreground">
              <item.icon className="size-4" />
              {t(`dashboard.returns.${item.key}`)}
            </div>
            <div className={cn('mt-2 text-2xl font-semibold tabular-nums', item.tone)}>
              {item.value}
            </div>
            {item.sub && (
              <div className="mt-1 text-xs text-muted-foreground">{item.sub}</div>
            )}
          </CardContent>
        </Card>
      ))}
    </div>
  )
}
