import { describe, expect, it } from 'vitest'

import en from './locales/en'
import zh from './locales/zh'

function keys(value: unknown, prefix = ''): string[] {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return [prefix]
  return Object.entries(value as Record<string, unknown>).flatMap(([key, child]) => keys(child, prefix ? `${prefix}.${key}` : key))
}

describe('locale contracts', () => {
  it('keeps the English and Chinese translation key sets identical', () => {
    expect(keys(en).sort()).toEqual(keys(zh).sort())
  })

  it('keeps all visible translations non-empty', () => {
    expect(keys(en).every((key) => key.split('.').reduce<unknown>((current, part) => (current as Record<string, unknown>)[part], en) !== '')).toBe(true)
    expect(keys(zh).every((key) => key.split('.').reduce<unknown>((current, part) => (current as Record<string, unknown>)[part], zh) !== '')).toBe(true)
  })
})
