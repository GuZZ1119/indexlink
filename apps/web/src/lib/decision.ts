import type { DecisionAction } from '@/api/types'

/** 动作 → Badge 配色，保持全站一致的语义色。 */
export const actionBadgeClass: Record<DecisionAction, string> = {
  overweight: 'border-transparent bg-action-overweight-bg text-action-overweight-fg',
  standard: 'border-transparent bg-action-standard-bg text-action-standard-fg',
  delay: 'border-transparent bg-action-delay-bg text-action-delay-fg',
  underweight: 'border-transparent bg-action-underweight-bg text-action-underweight-fg',
  skip: 'border-transparent bg-action-skip-bg text-action-skip-fg',
}

export function formatMultiplier(value: number): string {
  return `${value.toFixed(2).replace(/0$/, '')}x`
}

export function formatCurrency(value: number, currency: string): string {
  return new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency,
    maximumFractionDigits: 0,
  }).format(value)
}

export function formatPrice(value: number, currency: string): string {
  return new Intl.NumberFormat(undefined, {
    style: 'currency',
    currency,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value)
}

export function formatSignedPct(value: number): string {
  return `${value >= 0 ? '+' : ''}${value.toFixed(1)}%`
}
