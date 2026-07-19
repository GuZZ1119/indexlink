# 自适应定投与普通定投：30 组一年期回测实验报告

# Adaptive DCA vs Plain DCA: 30 One-Year Backtest Experiments

> 生成时间 / Generated: 2026-07-19 AEST  
> 数据窗口 / Data window: 2025-07-19 to 2026-07-17  
> 数据来源 / Data source: 本机 Futu/Moomoo OpenD 日线收盘价（只读）/ local Futu/Moomoo OpenD daily closing prices (read-only)

## 1. 结论摘要 / Executive summary

本轮 **30 组参数对比中，自适应策略没有取得平均收益率增长**。以资金加权年化收益率（MWR / XIRR）衡量，普通定投平均为 **18.83%**，自适应平均为 **16.97%**，自适应相对普通定投平均为 **-1.86 个百分点**。这是一个上涨为主的样本期：SPY、QQQ、IEF 的价格涨幅分别为 19.52%、23.84%、2.95%。本实现会在价格高于 MA200 时少投并保留现金，因此在单边上涨阶段错过了一部分上涨。

Across all **30 parameter comparisons, adaptive DCA did not produce an average return uplift**. Measured with money-weighted annual return (MWR / XIRR), plain DCA averaged **18.83%**, adaptive DCA averaged **16.97%**, and adaptive DCA lagged by **1.86 percentage points** on average. The sample was broadly bullish: SPY, QQQ, and IEF gained 19.52%, 23.84%, and 2.95% respectively. This implementation invests less and retains cash when prices are above MA200, which sacrifices upside in a persistent rally.

这不是“策略无效”的证明，也不是完整 70/20/10 的结论：一年、一个市场路径、三个资产组合和十组参数都不足以统计显著；更重要的是，历史月度 CAPE/ERP/VIX 与 Qwen 情绪快照尚未被保存，故本实验只测试可审计的价格-MA200 自适应规则。

This is neither proof that the strategy is ineffective nor a full 70/20/10 conclusion: one year, one market path, three asset universes, and ten parameter settings are not statistically sufficient. More importantly, historical monthly CAPE/ERP/VIX and Qwen sentiment snapshots were not stored, so this experiment tests only an auditable price/MA200 adaptation rule.

## 2. 资产与原始数据 / Assets and raw data

| 标的 / Instrument | 角色 / Role | 首个交易日收盘 / First close | 最后收盘 / Last close | 价格变化 / Price change | 样本数 / Observations |
|---|---|---:|---:|---:|---:|
| SPY | 标普 500 ETF / S&P 500 ETF | $621.8794（2025-07-21） | $743.2900（2026-07-17） | +19.52% | 250 |
| QQQ | 纳斯达克 100 ETF / Nasdaq-100 ETF | $561.4656（2025-07-21） | $695.3300（2026-07-17） | +23.84% | 250 |
| IEF | 美国 7–10 年期国债 ETF / US 7–10Y Treasury ETF | $91.1504（2025-07-21） | $93.8400（2026-07-17） | +2.95% | 250 |

组合定义 / Portfolio definitions:

1. **单资产 / Single asset**: SPY。
2. **双资产 / Two assets**: SPY + QQQ，所有初始资金与后续买入均 50/50 分配。
3. **三资产 / Three assets**: SPY + QQQ + IEF，所有初始资金与后续买入均等权分配。

## 3. 公平性与控制变量 / Fairness and controlled variables

- 每个测试的初始资金均为 **$1,200**，在首个可交易日按组合权重买入。
- Each test starts with **$1,200**, invested at the portfolio weights on the first available trading day.
- 每个普通定投与对应自适应定投接收**完全相同日期、完全相同金额**的外部入金；实验期总入金均为 **$2,400**（含初始资金）。
- Each plain/adaptive pair receives **identical dated external cash flows**; every test receives **$2,400** in total, including initial capital.
- 普通定投收到入金即按组合权重买入。自适应策略收到相同入金后可先保留现金，月末按规则决定买入；未买入现金以 0% 现金余额计入期末价值。
- Plain DCA invests each cash flow immediately at portfolio weights. Adaptive DCA receives the same flow, may retain cash, and decides its purchase at month end; uninvested cash is included at 0% in ending value.
- 因此，比较采用 **XIRR / MWR**，而不是“期末价值 ÷ 累计投入”的简单比率；这能处理不同买入时点，并把保留现金的机会成本保留下来。
- Therefore comparisons use **XIRR / MWR**, rather than a simple ending-value-to-contributions ratio; it handles different investment dates and retains the opportunity cost of cash.
- 不含佣金、税费、滑点、股息、利息、再平衡和汇率；价格为 OpenD 返回的收盘价，未在报告中额外假设总回报调整。
- No commissions, taxes, slippage, dividends, interest, rebalancing, or FX are included. Prices are OpenD closing prices; no additional total-return adjustment is assumed.

## 4. 策略规则 / Strategy rules

### 普通定投 / Plain DCA

按照测试编号，以每月一次 $100、每月两次各 $50，或每 15 个自然日 $50 的方式立即买入。若目标日不是交易日，则使用下一交易日收盘价。

The test schedule buys immediately as either $100 once monthly, $50 twice monthly, or $50 every 15 calendar days. If the scheduled date is not a trading day, the next trading-day close is used.

### 自适应定投 / Adaptive DCA

每个月最后一个交易日，计算组合各资产的 200 日均线距离并取平均值 `d`：

```text
multiplier = clamp(1 - 2.5 × d, 0.5, 2.0)
desired_purchase = min(monthly_cap, 100 × multiplier, available_cash)
```

随后将 `desired_purchase` 按组合权重买入。`monthly_cap` 是每月上限；自适应策略不能借款，故不会超过可用现金。这一规则是价格趋势/估值位置代理，不是完整的 IndexLink 70/20/10 决策引擎。

On the last trading day of each month, the strategy calculates the mean 200-day moving-average distance `d` across portfolio assets, then applies the formula above. `desired_purchase` is invested at portfolio weights. `monthly_cap` is a hard cap and the strategy cannot borrow, so it never exceeds available cash. This is a price trend/valuation-position proxy, not the complete IndexLink 70/20/10 decision engine.

## 5. 十组参数 / Ten parameter settings

| 测试 / Test | 普通定投频率 / Plain schedule | 自适应月上限 / Adaptive monthly cap |
|---|---|---:|
| T01 | 每月 1 次 $100 / Once monthly $100 | $100 |
| T02 | 每月 1 次 $100 / Once monthly $100 | $125 |
| T03 | 每月 1 次 $100 / Once monthly $100 | $150 |
| T04 | 每月 1 次 $100 / Once monthly $100 | $200 |
| T05 | 每月 2 次各 $50 / Twice monthly $50 | $100 |
| T06 | 每月 2 次各 $50 / Twice monthly $50 | $125 |
| T07 | 每月 2 次各 $50 / Twice monthly $50 | $150 |
| T08 | 每月 2 次各 $50 / Twice monthly $50 | $200 |
| T09 | 每 15 天 $50 / Every 15 days $50 | $150 |
| T10 | 每 15 天 $50 / Every 15 days $50 | $200 |

## 6. 30 组结果 / Results for all 30 comparisons

`Δ` = 自适应 XIRR − 普通 XIRR / adaptive XIRR minus plain XIRR. 期末自适应现金为已纳入自适应期末价值、但尚未买入的本机模拟现金。

### 单资产：SPY / Single asset: SPY

| 测试 | 上限 | 普通期末值 | 自适应期末值 | 普通 XIRR | 自适应 XIRR | Δ | 自适应现金 |
|---|---:|---:|---:|---:|---:|---:|---:|
| T01 | $100 | $2,745.24 | $2,707.25 | 19.55% | 17.38% | -2.17pp | $255.54 |
| T02 | $125 | $2,745.24 | $2,707.70 | 19.55% | 17.40% | -2.14pp | $252.48 |
| T03 | $150 | $2,745.24 | $2,707.70 | 19.55% | 17.40% | -2.14pp | $252.48 |
| T04 | $200 | $2,745.24 | $2,707.70 | 19.55% | 17.40% | -2.14pp | $252.48 |
| T05 | $100 | $2,739.46 | $2,707.25 | 19.47% | 17.60% | -1.86pp | $255.54 |
| T06 | $125 | $2,739.46 | $2,707.70 | 19.47% | 17.63% | -1.84pp | $252.48 |
| T07 | $150 | $2,739.46 | $2,707.70 | 19.47% | 17.63% | -1.84pp | $252.48 |
| T08 | $200 | $2,739.46 | $2,707.70 | 19.47% | 17.63% | -1.84pp | $252.48 |
| T09 | $150 | $2,740.33 | $2,707.70 | 19.52% | 17.63% | -1.89pp | $252.48 |
| T10 | $200 | $2,740.33 | $2,707.70 | 19.52% | 17.63% | -1.89pp | $252.48 |

**平均 / Mean:** 普通 19.51%，自适应 17.53%，Δ **-1.98pp**。SPY 同期买入并持有价格走势为 +19.52%。

**Interpretation:** Plain DCA closely matched the SPY price trend; the adaptive rule retained about $252–$256 cash in a rising market and lagged.

### 双资产：SPY + QQQ / Two assets: SPY + QQQ

| 测试 | 上限 | 普通期末值 | 自适应期末值 | 普通 XIRR | 自适应 XIRR | Δ | 自适应现金 |
|---|---:|---:|---:|---:|---:|---:|---:|
| T01 | $100 | $2,785.89 | $2,742.42 | 21.87% | 19.38% | -2.49pp | $295.36 |
| T02 | $125 | $2,785.89 | $2,743.27 | 21.87% | 19.43% | -2.44pp | $290.50 |
| T03 | $150 | $2,785.89 | $2,743.27 | 21.87% | 19.43% | -2.44pp | $290.50 |
| T04 | $200 | $2,785.89 | $2,743.27 | 21.87% | 19.43% | -2.44pp | $290.50 |
| T05 | $100 | $2,779.60 | $2,742.42 | 21.80% | 19.64% | -2.16pp | $295.36 |
| T06 | $125 | $2,779.60 | $2,743.27 | 21.80% | 19.69% | -2.11pp | $290.50 |
| T07 | $150 | $2,779.60 | $2,743.27 | 21.80% | 19.69% | -2.11pp | $290.50 |
| T08 | $200 | $2,779.60 | $2,743.27 | 21.80% | 19.69% | -2.11pp | $290.50 |
| T09 | $150 | $2,780.02 | $2,743.27 | 21.83% | 19.69% | -2.13pp | $290.50 |
| T10 | $200 | $2,780.02 | $2,743.27 | 21.83% | 19.69% | -2.13pp | $290.50 |

**平均 / Mean:** 普通 21.83%，自适应 19.58%，Δ **-2.25pp**。50/50 初始买入并持有的价格走势为 +21.68%。

**Interpretation:** QQQ 的强势上涨抬高了组合表现；同样也放大了自适应留存现金的机会成本。

### 三资产：SPY + QQQ + IEF / Three assets: SPY + QQQ + IEF

| 测试 | 上限 | 普通期末值 | 自适应期末值 | 普通 XIRR | 自适应 XIRR | Δ | 自适应现金 |
|---|---:|---:|---:|---:|---:|---:|---:|
| T01 | $100 | $2,668.66 | $2,642.00 | 15.18% | 13.66% | -1.52pp | $214.15 |
| T02 | $125 | $2,668.66 | $2,642.32 | 15.18% | 13.68% | -1.50pp | $211.42 |
| T03 | $150 | $2,668.66 | $2,642.32 | 15.18% | 13.68% | -1.50pp | $211.42 |
| T04 | $200 | $2,668.66 | $2,642.32 | 15.18% | 13.68% | -1.50pp | $211.42 |
| T05 | $100 | $2,664.22 | $2,642.00 | 15.12% | 13.84% | -1.28pp | $214.15 |
| T06 | $125 | $2,664.22 | $2,642.32 | 15.12% | 13.86% | -1.26pp | $211.42 |
| T07 | $150 | $2,664.22 | $2,642.32 | 15.12% | 13.86% | -1.26pp | $211.42 |
| T08 | $200 | $2,664.22 | $2,642.32 | 15.12% | 13.86% | -1.26pp | $211.42 |
| T09 | $150 | $2,664.28 | $2,642.32 | 15.13% | 13.86% | -1.27pp | $211.42 |
| T10 | $200 | $2,664.28 | $2,642.32 | 15.13% | 13.86% | -1.27pp | $211.42 |

**平均 / Mean:** 普通 15.14%，自适应 13.78%，Δ **-1.36pp**。等权初始买入并持有价格走势为 +15.44%。

**Interpretation:** IEF 的低波动、低涨幅降低了组合趋势，但未改变本样本中“保留现金落后于上涨资产”的主结论。

## 7. 汇总：收益率增长与大盘走势 / Summary: return uplift and market trend

| 组合 / Portfolio | 普通平均 XIRR | 自适应平均 XIRR | 平均变化 / Mean Δ | 对应买入并持有走势 / Buy-and-hold trend |
|---|---:|---:|---:|---:|
| SPY | 19.51% | 17.53% | **-1.98pp** | +19.52% |
| SPY + QQQ | 21.83% | 19.58% | **-2.25pp** | +21.68% |
| SPY + QQQ + IEF | 15.14% | 13.78% | **-1.36pp** | +15.44% |
| 全部 30 组 / All 30 tests | 18.83% | 16.97% | **-1.86pp** | 不适用 / N/A |

本样本中没有“平均收益率增长”；自适应规则在每一组资产组合都落后普通定投。最主要的可观察原因是期末仍保有约 $211–$295 现金，而同期 SPY/QQQ 上涨明显。月上限从 $125 提高到 $150/$200 在本规则下几乎没有额外效果：受限于可用现金及较温和的 MA200 信号，期望买入额没有触及更高上限。

There is no average return uplift in this sample; adaptive DCA trails plain DCA in every asset universe. The main observable reason is that it ends with roughly $211–$295 in cash while SPY/QQQ rose materially. Raising the cap from $125 to $150/$200 has almost no incremental effect here: available cash and modest MA200 signals prevented the desired purchase from reaching those larger caps.

## 8. 局限与下一步 / Limitations and next steps

1. **不是完整策略回测 / Not a full-strategy backtest.** 目前没有逐月持久化的 CAPE、ERP、VIX 和 Qwen 情绪，不能声称重放了完整 70/20/10 决策。
2. **样本路径单一 / One market path.** 这一年以美股上涨为主；应扩展到至少 5–10 年、包含 2020、2022 等不同 regime，并执行滚动窗口和 out-of-sample 检验。
3. **参数测试不等于独立样本 / Parameter tests are not independent samples.** 30 行共享相同市场路径；平均值用于灵敏度观察，而非统计显著性声明。
4. **缺少总回报成分 / Total-return components are absent.** IEF 利息、ETF 分红、税费、滑点和现金利息均未计入。
5. **可产品化的下一步 / Product next step.** 将规则、参数、日线快照、当月 CAPE/ERP/VIX、Qwen 输出、订单和成交一起持久化；之后以同一 Decision Engine 做无前视偏差的历史 replay，并报告 CAGR、XIRR、最大回撤、波动率、夏普比率及相对普通定投的统计区间。

## 9. 可复现性记录 / Reproducibility record

- 交易标的仅为 SPY、QQQ、IEF；不存在幸存者筛选或事后替换标的。
- Instruments are fixed to SPY, QQQ, and IEF; there is no survivor selection or post-hoc substitution.
- 所有普通/自适应成对测试使用相同初始资金、相同后续外部入金、相同资产权重、同一价格数据和同一期末估值日。
- Every plain/adaptive pair uses the same initial capital, later external flows, asset weights, price data, and ending valuation date.
- 用于抓取价格的临时本地标的将在报告完成后删除；本实验没有提交、撤销、修改或模拟任何订单。
- Temporary local holdings used only to fetch prices will be deleted after the report; this experiment submits, cancels, modifies, and simulates no order.
