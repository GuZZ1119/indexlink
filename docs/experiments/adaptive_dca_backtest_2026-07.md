# 50 组自适应定投与普通定投情景回测报告

# 50-Scenario Adaptive DCA vs Plain DCA Backtest Report

> 生成时间 / Generated: 2026-07-19 AEST  
> 数据窗口 / Data window: 2025-07-19 to 2026-07-17  
> 数据来源 / Data source: 本机 Futu/Moomoo OpenD 日线收盘价（只读）/ local Futu/Moomoo OpenD daily closing prices (read-only)

## 1. 结论摘要 / Executive summary

本报告替换此前的 30 组实验记录，扩展为 **50 组有约束的投资者情景实验**。每一组普通定投与自适应定投使用相同的初始资金、相同日期和金额的后续入金、相同组合权重与相同期末估值日；自适应未投资现金按 0% 计入期末净值。比较指标为资金加权年化收益率（XIRR / MWR）。

This report replaces the prior 30-experiment record and expands it to **50 controlled investor scenarios**. Each plain/adaptive pair uses the same initial capital, later dated cash flows, portfolio weights, and ending valuation date. Adaptive uninvested cash remains in ending NAV at 0%. The comparison metric is money-weighted annual return (XIRR / MWR).

**总体结果 / Overall result:** 普通定投平均 XIRR 为 **16.01%**，自适应策略平均为 **15.10%**，自适应相对普通定投平均为 **-0.91 个百分点**。这不是收益增长，而是在此样本路径上的小幅落后；结论如实保留。

**Overall result:** Plain DCA averages **16.01%** XIRR; adaptive DCA averages **15.10%**; adaptive DCA trails by **0.91 percentage points** on average. This is not a return uplift but a modest lag on this historical path, and the negative result is retained without optimisation after the fact.

## 2. 产品、原始数据与市场走势 / Products, raw data, and market trend

| 产品 / Product | 类型 / Type | 首个收盘 / First close | 最后收盘 / Last close | 价格走势 / Price trend |
|---|---|---:|---:|---:|
| SPY | 标普 500 ETF / S&P 500 ETF | $621.8794（2025-07-21） | $743.2900（2026-07-17） | +19.52% |
| QQQ | 纳斯达克 100 ETF / Nasdaq-100 ETF | $561.4656 | $695.3300 | +23.84% |
| IEF | 美国 7–10 年期国债 ETF / US 7–10Y Treasury ETF | $91.1504 | $93.8400 | +2.95% |
| GLD | 黄金 ETF / Gold ETF | $313.1300 | $368.4100 | +17.65% |
| VT | 全球股票 ETF / Global equity ETF | $127.9442 | $154.7800 | +20.97% |

每只产品在窗口内均有 250 个 OpenD 日线观测。使用五种产品是为了覆盖美国核心权益、成长权益、国债、黄金与全球权益；不含分红、国债利息、税费、佣金、滑点、现金利息、再平衡或汇率。

Each product has 250 OpenD daily observations in the window. The five products cover US core equity, growth equity, Treasuries, gold, and global equity. Dividends, Treasury interest, taxes, commissions, slippage, cash interest, rebalancing, and FX are excluded.

## 3. 组合与资金情景 / Portfolios and investor funding scenarios

### 组合 / Portfolio universes

| 代码 | 持仓 / Holdings | 资产数 |
|---|---|---:|
| P1 | SPY 100% | 1 |
| P2 | SPY 60% + QQQ 40% | 2 |
| P3 | SPY 60% + IEF 40% | 2 |
| P4 | VT 50% + IEF 30% + GLD 20% | 3 |
| P5 | SPY 35% + QQQ 25% + IEF 25% + GLD 15% | 4 |

### 资金情景 / Funding scenarios

| 情景 | 投资者画像 / Investor profile | 初始资金 | 月入金 | 普通定投频率 | 自适应月上限 |
|---|---|---:|---:|---|---:|
| S1 | 学生稳健 / Student steady | $500 | $200 | 每月 2 次 / 2× | $200 |
| S2 | 学生高频 / Student frequent | $800 | $200 | 每月 4 次 / 4× | $200 |
| G1 | 新毕业生核心 / New graduate core | $5,000 | $600 | 每月 2 次 / 2× | $900 |
| G2 | 新毕业生成长 / New graduate growth | $8,000 | $800 | 每月 4 次 / 4× | $1,200 |
| F1 | 家庭起步 / Family starter | $30,000 | $2,000 | 每月 4 次 / 4× | $3,000 |
| F2 | 家庭稳定 / Family established | $50,000 | $3,000 | 每月 2 次 / 2× | $5,000 |
| A1 | 富裕核心 / Affluent core | $100,000 | $5,000 | 每月 1 次 / 1× | $8,000 |
| A2 | 高净值增长 / High-net-worth growth | $250,000 | $10,000 | 每月 4 次 / 4× | 无硬上限 / Unlimited |
| W1 | 财富保全 / Wealth preservation | $500,000 | $6,000 | 每月 1 次 / 1× | $12,000 |
| I1 | 机构长期资金 / Institutional long horizon | $1,000,000 | $50,000 | 每月 4 次 / 4× | 无硬上限 / Unlimited |

每个资金情景与五个组合各组合一次，形成 **10 × 5 = 50** 个完整对比。该设计是分层情景抽样，而非用无意义的随机金额堆砌测试；它覆盖 1、2、3、4 个资产以及低资金、普通家庭、高净值和机构约束。

Each funding scenario is paired with every portfolio, yielding **10 × 5 = 50** complete comparisons. This is stratified scenario sampling rather than meaningless random amounts; it covers 1, 2, 3, and 4 assets plus student, household, high-net-worth, and institutional constraints.

## 4. 公平控制与策略规则 / Fair controls and strategy rules

### 公平控制 / Fair controls

- 同一测试编号下，两种策略收到完全相同的外部入金现金流。普通定投立即按组合权重买入；自适应策略收到同一笔资金后可持有现金。
- Under each test ID, both strategies receive exactly the same external cash flows. Plain DCA buys immediately at portfolio weights; adaptive DCA can hold the same cash.
- 初始资金均在首个交易日买入；其后按 1 日、1/15 日或 1/8/15/22 日等既定日程入金。非交易日顺延至下一交易日收盘价。
- Initial capital is invested on the first trading day. Later funding follows a fixed 1st, 1st/15th, or 1st/8th/15th/22nd schedule; non-trading dates roll to the next trading close.
- 自适应现金没有被丢弃：它留在期末净值中，按 0% 计价。XIRR 因此会反映“少投或晚投”的机会成本。
- Adaptive cash is not discarded: it remains in ending NAV at 0%. XIRR therefore captures the opportunity cost of investing less or later.

### 自适应规则 / Adaptive rule

每个入金日以目标组合权重计算 MA200 距离 `d`，然后：

```text
multiplier = clamp(1 - 2.5 × d, 0.25, 2.0)
desired purchase = scheduled contribution × multiplier
actual purchase = min(desired purchase, available cash, remaining monthly cap)
```

“无硬上限 / Unlimited”表示没有额外的月度金额上限；但策略仍不能借款，且 `2.0x` 是风险规则的倍率边界，不是资金上限。这个价格-MA200 规则不是完整 IndexLink 70/20/10 引擎：历史 CAPE、ERP、VIX 与 Qwen 情绪快照尚未按月持久化，不能被事后伪造。

“Unlimited” means there is no additional monthly amount cap; the strategy still cannot borrow, and `2.0x` is a risk-rule multiplier boundary rather than a capital cap. This price/MA200 rule is not the full IndexLink 70/20/10 engine: historical CAPE, ERP, VIX, and Qwen sentiment were not persisted monthly and must not be fabricated after the fact.

## 5. 完整 50 组结果 / Full results for all 50 comparisons

`Δ` = 自适应 XIRR − 普通 XIRR / adaptive XIRR minus plain XIRR. 期末值含自适应未投资现金。`市场`为同权重初始买入并持有的价格回报，不等同于定投 XIRR。

| # | 情景/组合 | 初始/月入金 | 频率/上限 | 普通期末值 | 自适应期末值 | 普通 XIRR | 自适应 XIRR | Δ | 市场 |
|---:|---|---:|---|---:|---:|---:|---:|---:|---:|
| 01 | S1/P1 | $500 / $200 | 2× / $200 | $3,207.98 | $3,166.26 | 19.12% | 16.49% | -2.64pp | 19.52% |
| 02 | S1/P2 | $500 / $200 | 2× / $200 | $3,239.38 | $3,191.05 | 21.12% | 18.05% | -3.07pp | 21.25% |
| 03 | S1/P3 | $500 / $200 | 2× / $200 | $3,089.15 | $3,070.77 | 11.65% | 10.50% | -1.15pp | 12.89% |
| 04 | S1/P4 | $500 / $200 | 2× / $200 | $3,052.19 | $3,033.33 | 9.35% | 8.18% | -1.17pp | 14.90% |
| 05 | S1/P5 | $500 / $200 | 2× / $200 | $3,101.36 | $3,074.07 | 12.41% | 10.71% | -1.70pp | 16.18% |
| 06 | S2/P1 | $800 / $200 | 4× / $200 | $3,511.98 | $3,470.16 | 19.20% | 16.95% | -2.25pp | 19.52% |
| 07 | S2/P2 | $800 / $200 | 4× / $200 | $3,547.65 | $3,498.99 | 21.13% | 18.50% | -2.63pp | 21.25% |
| 08 | S2/P3 | $800 / $200 | 4× / $200 | $3,374.87 | $3,356.57 | 11.85% | 10.87% | -0.97pp | 12.89% |
| 09 | S2/P4 | $800 / $200 | 4× / $200 | $3,342.09 | $3,324.11 | 10.10% | 9.15% | -0.95pp | 14.90% |
| 10 | S2/P5 | $800 / $200 | 4× / $200 | $3,395.14 | $3,368.39 | 12.93% | 11.50% | -1.43pp | 16.18% |
| 11 | G1/P1 | $5,000 / $600 | 2× / $900 | $13,807.26 | $13,682.09 | 19.39% | 17.86% | -1.53pp | 19.52% |
| 12 | G1/P2 | $5,000 / $600 | 2× / $900 | $13,961.90 | $13,816.94 | 21.28% | 19.51% | -1.77pp | 21.25% |
| 13 | G1/P3 | $5,000 / $600 | 2× / $900 | $13,218.74 | $13,163.59 | 12.23% | 11.57% | -0.67pp | 12.89% |
| 14 | G1/P4 | $5,000 / $600 | 2× / $900 | $13,178.19 | $13,121.60 | 11.74% | 11.06% | -0.68pp | 14.90% |
| 15 | G1/P5 | $5,000 / $600 | 2× / $900 | $13,370.36 | $13,288.48 | 14.07% | 13.08% | -0.99pp | 16.18% |
| 16 | G2/P1 | $8,000 / $800 | 4× / $1,200 | $19,785.05 | $19,617.75 | 19.42% | 18.04% | -1.38pp | 19.52% |
| 17 | G2/P2 | $8,000 / $800 | 4× / $1,200 | $20,010.62 | $19,815.98 | 21.28% | 19.68% | -1.60pp | 21.25% |
| 18 | G2/P3 | $8,000 / $800 | 4× / $1,200 | $18,918.41 | $18,845.19 | 12.31% | 11.71% | -0.60pp | 12.89% |
| 19 | G2/P4 | $8,000 / $800 | 4× / $1,200 | $18,883.73 | $18,811.81 | 12.03% | 11.44% | -0.59pp | 14.90% |
| 20 | G2/P5 | $8,000 / $800 | 4× / $1,200 | $19,157.16 | $19,050.16 | 14.27% | 13.39% | -0.88pp | 16.18% |
| 21 | F1/P1 | $30,000 / $2,000 | 4× / $3,000 | $61,414.94 | $60,996.69 | 19.50% | 18.46% | -1.04pp | 19.52% |
| 22 | F1/P2 | $30,000 / $2,000 | 4× / $3,000 | $62,151.61 | $61,665.01 | 21.34% | 20.13% | -1.21pp | 21.25% |
| 23 | F1/P3 | $30,000 / $2,000 | 4× / $3,000 | $58,585.43 | $58,402.38 | 12.49% | 12.04% | -0.45pp | 12.89% |
| 24 | F1/P4 | $30,000 / $2,000 | 4× / $3,000 | $58,699.65 | $58,519.86 | 12.77% | 12.33% | -0.44pp | 14.90% |
| 25 | F1/P5 | $30,000 / $2,000 | 4× / $3,000 | $59,510.85 | $59,243.34 | 14.78% | 14.12% | -0.66pp | 16.18% |
| 26 | F2/P1 | $50,000 / $3,000 | 2× / $5,000 | $98,917.11 | $98,291.25 | 19.53% | 18.58% | -0.95pp | 19.52% |
| 27 | F2/P2 | $50,000 / $3,000 | 2× / $5,000 | $100,122.19 | $99,397.36 | 21.37% | 20.26% | -1.11pp | 21.25% |
| 28 | F2/P3 | $50,000 / $3,000 | 2× / $5,000 | $94,317.25 | $94,041.52 | 12.54% | 12.12% | -0.42pp | 12.89% |
| 29 | F2/P4 | $50,000 / $3,000 | 2× / $5,000 | $94,616.75 | $94,333.83 | 12.99% | 12.56% | -0.43pp | 14.90% |
| 30 | F2/P5 | $50,000 / $3,000 | 2× / $5,000 | $95,896.63 | $95,487.24 | 14.93% | 14.31% | -0.62pp | 16.18% |
| 31 | A1/P1 | $100,000 / $5,000 | 1× / $8,000 | $185,071.27 | $184,019.28 | 19.61% | 18.78% | -0.83pp | 19.52% |
| 32 | A1/P2 | $100,000 / $5,000 | 1× / $8,000 | $187,388.20 | $186,171.60 | 21.44% | 20.48% | -0.96pp | 21.25% |
| 33 | A1/P3 | $100,000 / $5,000 | 1× / $8,000 | $176,199.00 | $175,733.74 | 12.64% | 12.27% | -0.36pp | 12.89% |
| 34 | A1/P4 | $100,000 / $5,000 | 1× / $8,000 | $177,101.89 | $176,607.63 | 13.35% | 12.96% | -0.39pp | 14.90% |
| 35 | A1/P5 | $100,000 / $5,000 | 1× / $8,000 | $179,444.37 | $178,748.91 | 15.19% | 14.64% | -0.55pp | 16.18% |
| 36 | A2/P1 | $250,000 / $10,000 | 4× / 无上限 | $426,597.87 | $424,506.62 | 19.59% | 18.89% | -0.70pp | 19.52% |
| 37 | A2/P2 | $250,000 / $10,000 | 4× / 无上限 | $432,008.73 | $429,575.74 | 21.39% | 20.58% | -0.81pp | 21.25% |
| 38 | A2/P3 | $250,000 / $10,000 | 4× / 无上限 | $405,821.36 | $404,906.12 | 12.67% | 12.37% | -0.30pp | 12.89% |
| 39 | A2/P4 | $250,000 / $10,000 | 4× / 无上限 | $408,401.56 | $407,502.61 | 13.53% | 13.23% | -0.30pp | 14.90% |
| 40 | A2/P5 | $250,000 / $10,000 | 4× / 无上限 | $413,733.63 | $412,396.09 | 15.30% | 14.86% | -0.44pp | 16.18% |
| 41 | W1/P1 | $500,000 / $6,000 | 1× / $12,000 | $676,273.62 | $675,011.23 | 19.72% | 19.48% | -0.24pp | 19.52% |
| 42 | W1/P2 | $500,000 / $6,000 | 1× / $12,000 | $685,618.46 | $684,158.54 | 21.49% | 21.21% | -0.28pp | 21.25% |
| 43 | W1/P3 | $500,000 / $6,000 | 1× / $12,000 | $640,436.74 | $639,878.42 | 12.93% | 12.82% | -0.11pp | 12.89% |
| 44 | W1/P4 | $500,000 / $6,000 | 1× / $12,000 | $649,154.85 | $648,561.74 | 14.58% | 14.47% | -0.11pp | 14.90% |
| 45 | W1/P5 | $500,000 / $6,000 | 1× / $12,000 | $656,814.89 | $655,980.34 | 16.03% | 15.87% | -0.16pp | 16.18% |
| 46 | I1/P1 | $1,000,000 / $50,000 | 4× / 无上限 | $1,834,181.40 | $1,823,725.17 | 19.55% | 18.72% | -0.83pp | 19.52% |
| 47 | I1/P2 | $1,000,000 / $50,000 | 4× / 无上限 | $1,856,916.94 | $1,844,751.96 | 21.37% | 20.40% | -0.97pp | 21.25% |
| 48 | I1/P3 | $1,000,000 / $50,000 | 4× / 无上限 | $1,746,871.33 | $1,742,295.10 | 12.60% | 12.24% | -0.36pp | 12.89% |
| 49 | I1/P4 | $1,000,000 / $50,000 | 4× / 无上限 | $1,754,749.54 | $1,750,254.76 | 13.23% | 12.87% | -0.36pp | 14.90% |
| 50 | I1/P5 | $1,000,000 / $50,000 | 4× / 无上限 | $1,778,219.69 | $1,771,532.02 | 15.09% | 14.56% | -0.53pp | 16.18% |

## 6. 按资产数汇总 / Summary by asset count

| 资产数 | 普通平均 XIRR | 自适应平均 XIRR | 平均 Δ | 平均市场走势 |
|---:|---:|---:|---:|---:|
| 1 | 19.46% | 18.23% | **-1.24pp** | +19.52% |
| 2 | 16.86% | 15.87% | **-0.99pp** | +17.07% |
| 3 | 12.37% | 11.82% | **-0.54pp** | +14.90% |
| 4 | 14.50% | 13.70% | **-0.80pp** | +16.18% |
| 全部 50 组 / All 50 | 16.01% | 15.10% | **-0.91pp** | 不适用 / N/A |

在这一年上涨路径中，普通定投总体更好；自适应规则在全部 50 组中均未超过对应普通定投。学生场景的落后更明显（平均约 -1.65 至 -1.94pp），因为有限月度现金被 MA200 高位信号延后投入，而后续上涨使保留现金的机会成本更高。高本金场景的差距较小（例如财富保全平均约 -0.18pp），因为初始一次性投入占总资本比例更高，后续定投时点影响被稀释。

On this one-year rising path, plain DCA performs better overall; the adaptive rule does not exceed its paired plain-DCA result in any of the 50 scenarios. The lag is more visible for student cases (about -1.65 to -1.94pp on average), because limited monthly cash is delayed by high-MA200 signals while the subsequent rise raises the opportunity cost. High-capital cases have smaller gaps (wealth preservation averages about -0.18pp), because the initial lump sum dominates the capital base and dilutes the impact of later timing.

## 7. 局限与下一步 / Limitations and next steps

1. **50 条参数结果不是 50 个独立市场样本 / Fifty parameter results are not fifty independent market samples.** 它们共享同一市场窗口，因此只能观察资金约束和组合敏感性，不能宣称统计显著。
2. **上涨窗口偏向立即投资 / A rising window favours immediate investment.** 若要检验自适应策略是否在高估、回撤和高波动 regime 中降低风险或改善回报，必须加入多个滚动窗口和至少 5–10 年数据。
3. **不是完整 70/20/10 replay / Not a full 70/20/10 replay.** 需要把逐月 CAPE、ERP、VIX、Qwen 情绪、Decision Engine 输出、订单、成交和现金流全部本地持久化，才可无前视地重放正式策略。
4. **总回报仍不完整 / Total return remains incomplete.** 分红、债券利息、税费、交易成本和现金利息会影响真实结果。
5. **产品改进方向 / Product improvement.** 将多执行日/双周频率纳入正式计划模型，并为每个计划周期写入执行去重记录；这样才能把本报告中的多频率实验转为可审计的真实 paper-trading 行为。

## 8. 可复现性 / Reproducibility

- 所有实验只读取本机 OpenD 历史价格，不提交、撤销、修改或模拟任何订单。
- All experiments only read local OpenD historical prices; no order is submitted, cancelled, modified, or simulated.
- 用于读取价格的临时 GLD/VT 计划会在实验后删除；它们不会留下决策、账本或订单记录。
- Temporary GLD/VT plans used only to read prices are deleted after the experiment; they leave no decision, ledger, or order record.
- 报告中的所有数值均可由本文件的资产权重、现金流、策略公式、价格起止点和期末日期重新计算。
- Every reported value can be recomputed from this file's asset weights, cash flows, formula, price endpoints, and ending date.
