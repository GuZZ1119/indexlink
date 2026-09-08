import { describe, expect, it } from 'vitest'

import type { StrategySpecDocument } from '@/api/types'
import { compareCopilotDraft } from './copilot-review'

const base = (): StrategySpecDocument => ({
  policy_id: 'dsl_guard',
  policy_version: 1,
  name: 'Guard',
  rules: [{
    condition: {
      kind: 'comparison',
      expression: { kind: 'indicator', indicator: { kind: 'relative_strength_index', lookback_days: 14 } },
      operator: 'less_than',
      threshold: '35',
    },
    action: { kind: 'set_opportunity_multiplier', multiplier: 1.1 },
  }],
})

describe('compareCopilotDraft', () => {
  it('reports no change for structurally identical bounded documents', () => {
    expect(compareCopilotDraft(base(), base())).toEqual([])
  })

  it('reports metadata and changed rule differences before a user applies a draft', () => {
    const draft = base()
    draft.name = 'Cautious guard'
    draft.rules[0].action = { kind: 'skip_opportunity' }
    expect(compareCopilotDraft(base(), draft)).toEqual([
      { field: 'name', kind: 'changed' },
      { field: 'rule_1', kind: 'changed' },
    ])
  })

  it('reports added and removed rules independently', () => {
    const additional = base()
    additional.rules.push(base().rules[0])
    expect(compareCopilotDraft(base(), additional)).toEqual([{ field: 'rule_2', kind: 'added' }])
    expect(compareCopilotDraft(additional, base())).toEqual([{ field: 'rule_2', kind: 'removed' }])
  })
})
