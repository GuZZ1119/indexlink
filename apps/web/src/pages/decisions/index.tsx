import { ListChecks } from 'lucide-react'
import { useTranslation } from 'react-i18next'

export default function DecisionsPage() {
  const { t } = useTranslation()
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 p-6 text-muted-foreground">
      <ListChecks className="size-8" />
      <p className="text-sm">{t('common.comingSoon')}</p>
    </div>
  )
}
