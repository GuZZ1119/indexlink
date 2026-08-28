import type { DecisionAction, DecisionRecord } from '@/api/types'

/** Filter immutable cross-plan records using only user-selected, display-safe criteria. */
export function filterDecisionRecords(
  records: DecisionRecord[],
  filters: { planId: string; action: '' | DecisionAction; from: string; to: string },
): DecisionRecord[] {
  return records.filter((item) => {
    const date = item.created_at.slice(0, 10)
    return (!filters.planId || item.plan_id === filters.planId)
      && (!filters.action || item.decision_snapshot.action === filters.action)
      && (!filters.from || date >= filters.from)
      && (!filters.to || date <= filters.to)
  })
}
