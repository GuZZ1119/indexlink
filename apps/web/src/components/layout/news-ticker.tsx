import { Radio } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { useNews } from '@/api/queries'
import { appLanguage } from '@/i18n'

export function NewsTicker() {
  const { i18n } = useTranslation()
  const { data: news } = useNews()
  const lang = appLanguage(i18n.language)

  const items = news ?? []

  return (
    <div className="flex h-9 shrink-0 items-center overflow-hidden border-b bg-muted/40">
      <div className="flex h-full shrink-0 items-center gap-1.5 border-r bg-background px-4 text-xs font-medium text-muted-foreground">
        <Radio className="size-3.5 text-status-live" />
      </div>
      <div className="relative flex-1 overflow-hidden">
        {items.length > 0 && (
          <div className="flex w-max animate-marquee items-center gap-10 whitespace-nowrap px-4 hover:[animation-play-state:paused]">
            {/* 渲染两份以实现无缝循环滚动 */}
            {[...items, ...items].map((item, idx) => (
              <span
                key={`${item.id}-${idx}`}
                className="flex items-center gap-2 text-xs text-muted-foreground"
              >
                <span className="font-mono font-semibold text-foreground">
                  {item.symbol}
                </span>
                {item.text[lang]}
              </span>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
