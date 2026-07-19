import { proxy } from 'valtio'

/** Browser-only UI state; server data remains in React Query. */
export const uiStore = proxy<{ selectedPlanId: string | null }>({
  selectedPlanId: null,
})

/** Select the plan used by the Dashboard and decision-history pages. */
export function setSelectedPlanId(planId: string | null) {
  uiStore.selectedPlanId = planId
}
