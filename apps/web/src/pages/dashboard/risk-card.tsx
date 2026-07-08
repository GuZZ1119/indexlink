import { Info, ShieldAlert, TriangleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { useRiskNotices } from '@/api/queries'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { appLanguage } from '@/i18n'
import { cn } from '@/lib/utils'

export function RiskCard() {
  const { t, i18n } = useTranslation()
  const { data, isPending } = useRiskNotices()
  const lang = appLanguage(i18n.language)

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ShieldAlert className="size-4 text-muted-foreground" />
          {t('dashboard.risk.title')}
        </CardTitle>
      </CardHeader>
      <CardContent>
        {isPending || !data ? (
          <div className="space-y-2">
            <Skeleton className="h-12 w-full" />
            <Skeleton className="h-12 w-full" />
          </div>
        ) : (
          <ul className="space-y-3">
            {data.map((notice) => (
              <li
                key={notice.id}
                className={cn(
                  'flex items-start gap-2 rounded-lg border p-3 text-sm leading-relaxed',
                  notice.level === 'warning'
                    ? 'border-semantic-warning-border bg-semantic-warning-bg text-semantic-warning'
                    : 'border-border bg-muted/40 text-muted-foreground',
                )}
              >
                {notice.level === 'warning' ? (
                  <TriangleAlert className="mt-0.5 size-4 shrink-0" />
                ) : (
                  <Info className="mt-0.5 size-4 shrink-0" />
                )}
                {notice.text[lang]}
              </li>
            ))}
          </ul>
        )}
      </CardContent>
    </Card>
  )
}
