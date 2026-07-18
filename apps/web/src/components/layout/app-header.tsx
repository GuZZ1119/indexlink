import { Languages } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'

import { Avatar, AvatarFallback } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { SidebarTrigger } from '@/components/ui/sidebar'

const mockAccount = {
  name: 'James',
  email: 'james@indexlink.dev',
}

export function AppHeader() {
  const { t, i18n } = useTranslation()

  const toggleLanguage = () => {
    void i18n.changeLanguage(i18n.language.startsWith('zh') ? 'en' : 'zh')
  }

  return (
    <header className="flex h-14 shrink-0 items-center gap-2 border-b bg-background px-4">
      <SidebarTrigger aria-label={t('header.toggleSidebar')} />
      <Link to="/" className="flex items-center gap-2">
        <img
          src="/logo.png"
          alt=""
          aria-hidden="true"
          className="size-40 object-contain"
        />
      </Link>

      <div className="ml-auto flex items-center gap-1">
        <Button
          variant="ghost"
          size="sm"
          onClick={toggleLanguage}
          aria-label={t('header.switchLanguage')}
        >
          <Languages className="size-4" />
          <span className="text-xs font-medium uppercase">
            {i18n.language.startsWith('zh') ? '中' : 'EN'}
          </span>
        </Button>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" className="gap-2 px-2">
              <Avatar className="size-7">
                <AvatarFallback className="text-xs">J</AvatarFallback>
              </Avatar>
              <span className="hidden text-sm sm:inline">{mockAccount.name}</span>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-52">
            <DropdownMenuLabel>
              <div className="flex flex-col">
                <span>{mockAccount.name}</span>
                <span className="text-xs font-normal text-muted-foreground">
                  {mockAccount.email}
                </span>
              </div>
            </DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem>{t('header.profile')}</DropdownMenuItem>
            <DropdownMenuItem>{t('header.settings')}</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive">
              {t('header.signOut')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </header>
  )
}
