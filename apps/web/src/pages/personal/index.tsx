import { ArrowRight, CalendarDays, CheckCircle2, CircleDollarSign, Sparkles } from 'lucide-react'
import { useState } from 'react'
import { Link } from 'react-router'
import { useSnapshot } from 'valtio'

import { PageHeading } from '@/components/v2_1/page-heading'
import { StrategyCard } from '@/components/v2_1/strategy-card'
import { findConsumerStrategy } from '@/features/v2_1/model'
import { uiStore } from '@/stores/ui'

const updates = [
  { title: '本月建议已准备好', text: '市场位置相对历史偏低，本月建议在原计划基础上多投入一点。', when: '今天' },
  { title: '策略回顾已更新', text: '自适应长期计划新增了最近一期的回测说明与风险边界。', when: '昨天' },
  { title: '你已连续执行 8 个月', text: '长期计划的价值，来自每次按规则完成。', when: '6 月 1 日' },
]

export default function PersonalPage() {
  const { activeStrategyId } = useSnapshot(uiStore)
  const [completedThisMonth, setCompletedThisMonth] = useState(false)
  const strategy = findConsumerStrategy(activeStrategyId)

  return (
    <div className="mx-auto w-full max-w-7xl space-y-8 px-5 py-8 md:px-8 lg:px-10 lg:py-10">
      <PageHeading eyebrow="个人中心" title="把长期计划，过成一件轻松的事" description="这里不显示需要你盯住的市场噪音，只保留计划、下一步与值得回顾的变化。" />

      <section className="overflow-hidden rounded-[1.6rem] bg-[#102028] text-white">
        <div className="grid gap-8 p-6 sm:p-8 lg:grid-cols-[1.35fr_.65fr] lg:p-10">
          <div>
            <p className="inline-flex items-center gap-2 text-sm text-[#b8d5c6]"><Sparkles className="size-4" />本月航线</p>
            <h2 className="mt-5 max-w-xl text-3xl font-semibold tracking-[-0.045em] sm:text-4xl">按计划投入 ¥2,200</h2>
            <p className="mt-4 max-w-lg text-[0.95rem] leading-7 text-slate-300">你的基础计划是 ¥2,000。现在处于相对历史中低位，因此策略建议额外增加 10%。这不是对未来的预测，只是在兑现你已经选好的规则。</p>
            <div className="mt-7 flex flex-wrap gap-3">
              <button type="button" onClick={() => setCompletedThisMonth(true)} disabled={completedThisMonth} className="rounded-full bg-white px-4 py-2.5 text-sm font-medium text-[#102028] transition-colors hover:bg-[#dcece4] disabled:cursor-default disabled:bg-[#dcece4]">{completedThisMonth ? '已记录为完成' : '我已完成这次投入'}</button>
              <Link to="/strategy-center" className="inline-flex items-center gap-1 rounded-full border border-white/20 px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-white/10">查看依据 <ArrowRight className="size-3.5" /></Link>
            </div>
          </div>
          <div className="grid content-end gap-3 rounded-[1.2rem] border border-white/10 bg-white/5 p-5">
            <div><p className="text-sm text-slate-300">下一次计划日</p><p className="mt-1 text-2xl font-medium">6 月 15 日</p></div>
            <div className="h-px bg-white/10" />
            <div><p className="text-sm text-slate-300">正在坚持</p><p className="mt-1 font-medium">{strategy.shortName}</p></div>
            <div><p className="text-sm text-slate-300">已连续完成</p><p className="mt-1 font-medium">8 个月</p></div>
          </div>
        </div>
      </section>

      <section className="grid gap-7 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <div className="space-y-5">
          <div className="flex items-center justify-between"><div><h2 className="text-xl font-semibold tracking-[-0.03em] text-[#102028]">你正在坚持的计划</h2><p className="mt-1 text-sm text-slate-500">可以随时回到策略中心，先理解再换用。</p></div><Link to="/strategy-center" className="text-sm font-medium text-[#2d6a57]">策略中心</Link></div>
          {completedThisMonth && <p role="status" className="rounded-xl border border-[#b8d5c6] bg-[#f1f7f4] px-4 py-3 text-sm leading-6 text-[#245a49]">已在当前浏览器会话中记录本月完成。同步到真实计划记录将在后端接入后开放。</p>}
          <StrategyCard strategy={strategy} selected />
        </div>
        <aside className="rounded-[1.35rem] border border-slate-200 bg-white p-5">
          <h2 className="text-lg font-semibold tracking-[-0.025em] text-[#102028]">最近变化</h2>
          <ol className="mt-5 space-y-5">
            {updates.map((update, index) => <li key={update.title} className="relative pl-6"><span className="absolute left-0 top-1.5 size-2.5 rounded-full bg-[#2d6a57]" />{index < updates.length - 1 && <span className="absolute left-[4px] top-5 h-[calc(100%+8px)] w-px bg-slate-200" />}<p className="text-sm font-medium text-[#102028]">{update.title}</p><p className="mt-1 text-sm leading-6 text-slate-600">{update.text}</p><p className="mt-1.5 text-xs text-slate-400">{update.when}</p></li>)}
          </ol>
        </aside>
      </section>

      <section className="grid gap-4 sm:grid-cols-3">
        <MiniStat icon={<CalendarDays />} label="今年完成" value={completedThisMonth ? '6 / 6 次' : '5 / 6 次'} detail={completedThisMonth ? '本月已在当前会话记录完成' : '下一次在 6 月 15 日'} />
        <MiniStat icon={<CircleDollarSign />} label="累计投入" value="¥12,000" detail="全部为本地演示数据" />
        <MiniStat icon={<CheckCircle2 />} label="计划匹配度" value="92%" detail="当前持仓与目标计划的接近程度" />
      </section>
    </div>
  )
}

function MiniStat({ icon, label, value, detail }: { icon: React.ReactNode; label: string; value: string; detail: string }) {
  return <div className="rounded-[1.2rem] border border-slate-200 bg-white p-5"><div className="flex items-center gap-2 text-sm text-slate-500">{icon}{label}</div><p className="mt-4 text-2xl font-semibold tracking-[-0.035em] text-[#102028]">{value}</p><p className="mt-2 text-xs leading-5 text-slate-500">{detail}</p></div>
}
