import { proxy } from 'valtio'

export type ChartRange = 'y1' | 'y3' | 'all'

/** 浏览器本地 UI 状态：仅存放临时交互状态，不存服务端数据。 */
export const uiStore = proxy<{ chartRange: ChartRange }>({
  chartRange: 'all',
})

export function setChartRange(range: ChartRange) {
  uiStore.chartRange = range
}
