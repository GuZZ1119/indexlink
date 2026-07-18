import { ComparisonChart } from './comparison-chart'
import { LatestDecisionCard } from './latest-decision-card'
import { ReturnsCards } from './returns-cards'
import { RiskCard } from './risk-card'
import { ScoreCards } from './score-cards'
import { ValuationCard } from './valuation-card'

export default function DashboardPage() {
  return (
    <div className="flex flex-col gap-4 p-4 lg:p-6">
      <div className="grid grid-cols-1">
        <ValuationCard />
      </div>
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-3">
        <div className="flex flex-col gap-4 lg:col-span-2">
          <ScoreCards />
          <ReturnsCards />
          <ComparisonChart />
        </div>
        <div className="flex flex-col gap-4">
          <LatestDecisionCard />
          <RiskCard />
        </div>
      </div>
    </div>
  )
}
