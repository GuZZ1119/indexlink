export default {
  common: {
    appName: 'IndexLink',
    comingSoon: 'This page is under construction.',
  },
  nav: {
    dashboard: 'Dashboard',
    decisions: 'Decisions',
    plans: 'Plans',
  },
  header: {
    toggleSidebar: 'Toggle sidebar',
    switchLanguage: 'Switch language',
    profile: 'Profile',
    settings: 'Settings',
    signOut: 'Sign out',
  },
  action: {
    overweight: 'Overweight',
    standard: 'Standard',
    delay: 'Delay',
    underweight: 'Underweight',
    skip: 'Skip',
  },
  dashboard: {
    valuation: {
      title: 'Current Market Valuation',
      composite: 'Composite valuation percentile',
      suggestedAction: 'Suggested action',
      multiplier: 'Execution multiplier',
      expectedAmount: 'Expected amount',
      nextDcaTime: 'Next DCA time',
      why: 'why?',
      collapse: 'Collapse',
      metrics: {
        cape: 'CAPE percentile',
        erp: 'ERP percentile',
        ma200: 'MA200 distance percentile',
        rsi: 'RSI percentile',
        vix: 'VIX percentile',
      },
      metricDescriptions: {
        cape:
          'The CAPE percentile shows where the current Shiller P/E sits within its own historical distribution. A higher percentile means valuation is more expensive relative to history and usually reduces allocation; a lower percentile suggests cheaper valuation and may support overweighting.',
        erp:
          'The ERP percentile shows where the current equity risk premium sits within its own historical distribution. Unlike CAPE, a higher ERP percentile means stocks offer more compensation over risk-free assets and valuation pressure is lower; a lower ERP percentile means compensation is thin and the system becomes more cautious.',
        ma200:
          'The MA200-distance percentile shows how far the current price is from its 200-day moving average relative to history. A higher percentile means price is far above its long-term average and may indicate overheating; a lower percentile means price is near or below the average, but may also reflect weakening trend.',
        rsi:
          'The RSI percentile shows where the current RSI sits within its own historical distribution. A higher percentile usually indicates strong momentum and possible overbought conditions, so the system may reduce chasing risk; a lower percentile indicates weaker momentum or oversold conditions, but should be checked with VIX and trend context.',
        vix:
          'The VIX percentile shows where current market volatility and fear sit within history. A higher percentile means stronger fear and higher short-term drawdown risk, which may trigger falling-knife protection; a lower percentile means calmer markets, but not necessarily low risk.',
      },
    },
    scores: {
      fundamental: 'Fundamental',
      trend: 'Trend',
      sentiment: 'AI Sentiment',
      composite: 'Composite',
      weight: 'Weight {{value}}%',
    },
    latest: {
      title: 'Latest Decision',
      amount: 'Executed amount',
      baseAmount: 'Base amount',
      multiplier: 'Multiplier',
      executionPrice: 'DCA price',
      viewDetail: 'View detail',
    },
    risk: {
      title: 'Risk Notices',
    },
    returns: {
      total: 'Total Return',
      position: 'Position P&L',
      realized: 'Realized P&L',
      invested: 'Total Invested',
      annualized: 'Annualized {{value}}',
      vsDca: '{{value}} vs plain DCA',
    },
    chart: {
      title: 'Performance Comparison',
      subtitle: 'Plain DCA vs Adaptive DCA (cumulative return)',
      dca: 'Plain DCA',
      adaptive: 'ADCA',
      range: {
        y1: '1Y',
        y3: '3Y',
        all: 'All',
      },
    },
  },
} as const
