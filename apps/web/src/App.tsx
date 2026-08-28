import { lazy, Suspense } from 'react'
import type { ComponentType, LazyExoticComponent } from 'react'
import { createBrowserRouter, RouterProvider } from 'react-router'

import { AppLayout } from '@/components/layout/app-layout'
import RouteErrorPage from '@/pages/route-error'

const DashboardPage = lazy(() => import('@/pages/dashboard'))
const DecisionsPage = lazy(() => import('@/pages/decisions'))
const PlansPage = lazy(() => import('@/pages/plans'))
const StrategiesPage = lazy(() => import('@/pages/strategies'))

function PageFallback() {
  return <div className="p-6 text-sm text-muted-foreground">Loading…</div>
}

function LazyPage({ Page }: { Page: LazyExoticComponent<ComponentType> }) {
  return <Suspense fallback={<PageFallback />}><Page /></Suspense>
}

const router = createBrowserRouter([
  {
    element: <AppLayout />,
    errorElement: <RouteErrorPage />,
    children: [
      { path: '/', element: <LazyPage Page={DashboardPage} /> },
      { path: '/decisions/:id?', element: <LazyPage Page={DecisionsPage} /> },
      { path: '/plans/:id?', element: <LazyPage Page={PlansPage} /> },
      { path: '/strategies', element: <LazyPage Page={StrategiesPage} /> },
    ],
  },
])

export default function App() {
  return <RouterProvider router={router} />
}
