import type { StrategySpecDocument } from '@/api/types'

/** One display-safe structural change between the editable form and an AI draft. */
export interface CopilotDraftChange {
  /** Stable document field or one-based rule location. */
  field: string
  /** Whether the draft adds, removes, or modifies this part of the document. */
  kind: 'added' | 'removed' | 'changed'
}

/** Compare only the bounded DSL document shape; no model prose is executable. */
export function compareCopilotDraft(
  current: StrategySpecDocument,
  draft: StrategySpecDocument,
): CopilotDraftChange[] {
  const changes: CopilotDraftChange[] = []
  for (const field of ['policy_id', 'policy_version', 'name'] as const) {
    if (current[field] !== draft[field]) changes.push({ field, kind: 'changed' })
  }
  const largestLength = Math.max(current.rules.length, draft.rules.length)
  for (let index = 0; index < largestLength; index += 1) {
    const before = current.rules[index]
    const after = draft.rules[index]
    if (!before && after) changes.push({ field: `rule_${index + 1}`, kind: 'added' })
    else if (before && !after) changes.push({ field: `rule_${index + 1}`, kind: 'removed' })
    else if (JSON.stringify(before) !== JSON.stringify(after)) changes.push({ field: `rule_${index + 1}`, kind: 'changed' })
  }
  return changes
}
