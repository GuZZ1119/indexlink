import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { MemoryRouter } from 'react-router'

import { StrategyCard } from '@/components/v2_1/strategy-card'
import { findConsumerStrategy } from '@/features/v2_1/model'
import LabPage from '@/pages/lab'
import PersonalPage from '@/pages/personal'
import StrategyCenterPage from '@/pages/strategy-center'
import { setActiveStrategyId } from '@/stores/ui'

const renderPage = (page: React.ReactNode) => render(<MemoryRouter>{page}</MemoryRouter>)

describe('V2.1 consumer shell', () => {
  beforeEach(() => setActiveStrategyId('adaptive-70-20-10'))
  afterEach(cleanup)

  it('shows one clear monthly action and makes its local-only result visible', () => {
    renderPage(<PersonalPage />)
    expect(screen.getByRole('heading', { name: '按计划投入 ¥2,200' })).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: '我已完成这次投入' }))
    expect(screen.getByRole('status').textContent).toContain('当前浏览器会话')
    expect(screen.getByText('6 / 6 次')).toBeTruthy()
  })

  it('explains and compares strategies before a user selects one', () => {
    renderPage(<StrategyCenterPage />)
    fireEvent.click(screen.getAllByRole('button', { name: '选用这个策略' })[0])
    expect(screen.getByRole('status').textContent).toContain('个人中心已同步更新')
    fireEvent.click(screen.getByRole('button', { name: '和其他策略对比' }))
    expect(screen.getByText('最该知道的限制')).toBeTruthy()
    fireEvent.change(screen.getByLabelText('选择对比策略'), { target: { value: 'defensive-balance' } })
    expect(screen.getByText('股债平衡')).toBeTruthy()
  })

  it('opens and closes a local configuration preview without claiming to connect anything', () => {
    renderPage(<LabPage />)
    fireEvent.click(screen.getAllByRole('button', { name: '打开配置预览' })[1])
    expect(screen.getByText('配置预览')).toBeTruthy()
    expect(screen.getByText(/不写入任何配置/)).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: '收起预览' }))
    expect(screen.queryByText('配置预览')).toBeNull()
  })

  it('renders a compact strategy card without the rule panel', () => {
    render(<StrategyCard strategy={findConsumerStrategy('steady-dca')} selected={false} variant="compact" onSelect={() => undefined} />)
    expect(screen.getByText('每月稳步投入')).toBeTruthy()
    expect(screen.queryByText('它会怎么做：')).toBeNull()
  })
})
