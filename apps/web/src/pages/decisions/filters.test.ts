import { describe, expect, it } from 'vitest'

import { filterDecisionRecords } from './filters'
import type { DecisionRecord } from '@/api/types'

function record(id: string, planId: string, createdAt: string, action: DecisionRecord['decision_snapshot']['action']): DecisionRecord {
  return {
    id,
    plan_id: planId,
    symbol: 'VOO',
    currency: 'USD',
    execution_status: 'due',
    planned_contribution: '100.00',
    execution_snapshot: {},
    fundamental_snapshot: {},
    trend_snapshot: {},
    decision_snapshot: { action, multiplier: 1 },
    summary: 'test',
    created_at: createdAt,
  }
}

describe('filterDecisionRecords', () => {
  const records = [
    record('1', 'plan-a', '2026-08-01T12:00:00Z', 'standard'),
    record('2', 'plan-a', '2026-08-08T12:00:00Z', 'underweight'),
    record('3', 'plan-b', '2026-08-15T12:00:00Z', 'overweight'),
  ]

  it('combines plan, action and inclusive date filters without changing cached records', () => {
    expect(filterDecisionRecords(records, { planId: 'plan-a', action: 'underweight', from: '2026-08-08', to: '2026-08-08' }).map((item) => item.id)).toEqual(['2'])
    expect(records).toHaveLength(3)
  })

  it('returns every record when all filters are empty', () => {
    expect(filterDecisionRecords(records, { planId: '', action: '', from: '', to: '' })).toHaveLength(3)
  })
})
