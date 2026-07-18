import { createBrowserRouter, RouterProvider } from 'react-router'

import { AppLayout } from '@/components/layout/app-layout'
import DashboardPage from '@/pages/dashboard'
import DecisionsPage from '@/pages/decisions'
import PlansPage from '@/pages/plans'

const router = createBrowserRouter([
  {
    element: <AppLayout />,
    children: [
      { path: '/', element: <DashboardPage /> },
      { path: '/decisions/:id?', element: <DecisionsPage /> },
      { path: '/plans/:id?', element: <PlansPage /> },
    ],
  },
])

export default function App() {
  return <RouterProvider router={router} />
}
