import type { CSSProperties } from 'react'
import { Outlet } from 'react-router'

import { AppHeader } from './app-header'
import { AppSidebar } from './app-sidebar'
import { NewsTicker } from './news-ticker'
import { SidebarInset, SidebarProvider } from '@/components/ui/sidebar'

export function AppLayout() {
  return (
    <SidebarProvider
      className="h-svh min-h-0 flex-col"
      style={{ '--app-chrome-height': '5.75rem' } as CSSProperties}
    >
      <NewsTicker />
      <AppHeader />
      <div className="flex min-h-0 flex-1">
        <AppSidebar />
        <SidebarInset className="min-h-0 overflow-y-auto">
          <Outlet />
        </SidebarInset>
      </div>
    </SidebarProvider>
  )
}
