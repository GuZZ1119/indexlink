import { AlertTriangle, ArrowLeft, RefreshCw } from 'lucide-react'
import { isRouteErrorResponse, Link, useRouteError } from 'react-router'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/ui/button'

/** Replace React Router's development exception page with a recoverable bilingual error boundary. */
export default function RouteErrorPage() {
  const { t } = useTranslation()
  const error = useRouteError()
  const detail = isRouteErrorResponse(error)
    ? `${error.status} ${error.statusText}`
    : error instanceof Error ? error.message : t('routeError.unknown')
  return (
    <main className="grid min-h-svh place-items-center bg-muted/20 p-6">
      <section className="w-full max-w-lg rounded-xl border bg-background p-6 shadow-sm">
        <AlertTriangle className="mb-3 size-7 text-destructive" />
        <h1 className="text-lg font-semibold">{t('routeError.title')}</h1>
        <p className="mt-2 text-sm text-muted-foreground">{t('routeError.description')}</p>
        <p className="mt-3 break-words rounded-md bg-muted p-3 font-mono text-xs text-muted-foreground">{detail}</p>
        <div className="mt-5 flex flex-wrap gap-2">
          <Button asChild variant="outline"><Link to="/"><ArrowLeft />{t('routeError.dashboard')}</Link></Button>
          <Button onClick={() => globalThis.location.reload()}><RefreshCw />{t('routeError.retry')}</Button>
        </div>
      </section>
    </main>
  )
}
