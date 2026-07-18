export default {
  common: {
    appName: 'IndexLink',
    comingSoon: '页面规划中，敬请期待。',
  },
  nav: {
    dashboard: '仪表盘',
    decisions: '决策明细',
    plans: '定投计划',
  },
  header: {
    toggleSidebar: '收放侧栏',
    switchLanguage: '切换语言',
    profile: '个人资料',
    settings: '设置',
    signOut: '退出登录',
  },
  action: {
    overweight: '加码',
    standard: '标准',
    delay: '延时',
    underweight: '减量',
    skip: '跳过',
  },
  dashboard: {
    valuation: {
      title: '当前市场估值',
      composite: '综合估值分位',
      suggestedAction: '建议动作',
      multiplier: '执行倍率',
      expectedAmount: '预计执行金额',
      nextDcaTime: '下期定投时间',
      why: 'why?',
      collapse: '收回',
      metrics: {
        cape: 'CAPE 分位',
        erp: 'ERP 分位',
        ma200: 'MA200 距离分位',
        rsi: 'RSI 分位',
        vix: 'VIX 分位',
      },
      metricDescriptions: {
        cape:
          'CAPE 分位表示当前席勒市盈率在自身历史分布中的位置。分位越高，代表估值相对历史越贵，系统通常会降低投入；分位越低，代表估值相对历史越便宜，可能支持加码。',
        erp:
          'ERP 分位表示当前股权风险溢价在自身历史分布中的位置。与 CAPE 相反，ERP 分位越高，代表股票相对无风险资产的补偿越高、估值压力越低；ERP 分位越低，代表风险补偿不足，系统会更谨慎。',
        ma200:
          'MA200 距离分位表示当前价格相对 200 日均线的偏离程度在历史中的位置。分位越高，说明价格越明显高于长期均线，可能存在追高或赶顶风险；分位越低，说明价格接近或低于长期均线，但也可能反映趋势转弱。',
        rsi:
          'RSI 分位表示当前 RSI 在自身历史分布中的位置。分位越高，通常代表市场动能偏强、接近超买区间，系统可能降低追高风险；分位越低，代表动能偏弱或接近超卖，但需要结合 VIX 和趋势确认是否属于急跌。',
        vix:
          'VIX 分位表示当前市场波动率和恐慌程度在历史中的位置。分位越高，代表市场恐慌越强、短期急跌风险越高，系统可能触发“接飞刀”保护；分位越低，代表市场较平静，但不一定等于低风险。',
      },
    },
    scores: {
      fundamental: '基本面',
      trend: '趋势面',
      sentiment: 'AI 情绪',
      composite: '综合得分',
      weight: '权重 {{value}}%',
    },
    latest: {
      title: '最近一次决策',
      amount: '执行金额',
      baseAmount: '基准金额',
      multiplier: '倍率',
      executionPrice: '本期定投价格',
      viewDetail: '查看详情',
    },
    risk: {
      title: '风险提示',
    },
    returns: {
      total: '总收益',
      position: '持仓收益',
      realized: '确定收益',
      invested: '累计投入',
      annualized: '年化收益率 {{value}}',
      vsDca: '相对普通定投 {{value}}',
    },
    chart: {
      title: '收益对比',
      subtitle: '普通定投 vs 自适应定投（累计收益率）',
      dca: '普通定投',
      adaptive: '自适应定投',
      range: {
        y1: '近 1 年',
        y3: '近 3 年',
        all: '全部',
      },
    },
  },
} as const
