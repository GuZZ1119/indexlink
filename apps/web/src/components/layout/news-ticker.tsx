import { Radio } from 'lucide-react'
import { useTranslation } from 'react-i18next'

export function NewsTicker() {
  const { t } = useTranslation()

  return (
    <div className="flex h-9 shrink-0 items-center overflow-hidden border-b bg-muted/40">
      <div className="flex h-full shrink-0 items-center gap-1.5 border-r bg-background px-4 text-xs font-medium text-muted-foreground">
        <Radio className="size-3.5 text-status-live" />
      </div>
      <div className="relative flex-1 overflow-hidden">
        <div className="flex h-full items-center px-4 text-xs text-muted-foreground">
          {t('live.ticker')}
        </div>
      </div>
    </div>
  )
}
