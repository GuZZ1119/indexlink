# IndexLink 全项目最小 MVP 清单

本文档描述整个 IndexLink 项目的“演示可用”最小 MVP，而不是只描述投资计划或双桶部分。项目完整主线是：

```text
投资计划
  + 70% 基本面分位
  + 20% 趋势节奏
  + 10% 阿里云 Qwen 情绪
  -> 70/20/10 综合决策
  -> 执行预览
  -> 双桶拆分
  -> 最终总结 / 决策存证
```

当前 PR 只补齐了“双桶执行预览 API”这块演示缺口；它不是整个 MVP 的终点。真正的最小 MVP 需要能把投资计划、量化信号、AI 情绪、双桶拆分和最终解释串成一个可演示闭环。

## MVP 边界

最小 MVP 只要求后端可演示，不要求完整生产交易系统。

必须包含：

- 单用户投资计划管理。
- 70/20/10 决策模型的可演示输出。
- 阿里云 DashScope/Qwen 的真实 sentiment 调用路径。
- 执行日预览与双桶金额拆分。
- 一份最终 summary，说明为什么今天建议执行、跳过、标准执行或加/减码。

暂不包含：

- 真实券商下单。
- 自动 Scheduler。
- 订单状态机与成交回报。
- 多用户权限。
- 完整前端。
- 生产级行情源和缓存策略。

## 当前已完成内容

### 后端基础设施

- Axum API server。
- PostgreSQL storage adapter。
- 健康检查与 readiness 检查。
- 统一 JSON error envelope。
- 本地 Docker Compose 开发环境。

### 投资计划

- 创建 investment plan。
- 列出 investment plans。
- 按 ID 查询 investment plan。
- 更新 investment plan。
- 领域层规范化 `name`、`symbol`、`currency`、金额与执行日。
- 入站 DTO 与领域类型隔离，避免 serde 直接构造带不变量的领域模型。

### 执行预览与双桶

- 判断计划在指定月内日期是否 `due`、`waiting` 或 `inactive`。
- due 时返回受 `max_single_execution` 限制后的计划投入金额。
- 支持 `core` 与 `opportunity` 两个投入桶。
- 校验双桶比例必须在 `0..=1`，且两个比例合计必须为 `1`。
- API 暴露执行预览入口：`POST /investment-plans/:id/execution-preview`。
- due 且请求提供双桶配置时，返回本次计划投入金额的 core/opportunity 拆分。

### 70% 基本面量化

- `quant-engine` 已有 fundamental 方向的实现和测试。
- 已覆盖 CAPE、ERP、历史分位、权重、错误边界和审计字段。
- 这部分可以作为 70% 主信号进入后续 decision preview。

### 10% AI 感知库层

- 已有 DashScope/OpenAI-compatible Qwen client。
- 已有 CNBC RSS 新闻源。
- 已有新闻格式化为 sentiment prompt 的逻辑。
- 已有“拉取新闻 -> 调用 AI provider -> 返回 sentiment”的库层 pipeline。
- 但还没有接入 server config、API state 和 HTTP route。

## 当前明确缺口

### 20% 趋势节奏尚未实现完整行为

README 设计里，20% trend 应由 200 日均线距离、RSI、VIX 等技术指标组成，用于控制“不要接飞刀 / 不要追顶”的节奏。

当前状态：

- trend config、stub 和大量 TDD 边界测试已经存在。
- `evaluate_trend` 行为仍未完整实现。
- 多数 trend 行为测试仍是 ignored。

MVP 需要补到：

- `evaluate_trend` 返回真实 `TrendSignal`，不再只依赖 neutral stub。
- 支持 overheated、falling knife、neutral 等基础 regime。
- 输出可进入 70/20/10 合成的趋势分数。
- 最终 summary 能解释趋势项如何影响加码或降码。

### 阿里云 Qwen 还没接到 API

当前 AI 只是 library-ready，不能直接从 API 演示。

MVP 需要补到：

- server config 读取 `DASHSCOPE_API_KEY`。
- 支持配置 Qwen model、base URL、timeout、temperature 和 max tokens。
- API state 注入 Qwen provider 与 news source。
- 新增 market sentiment API。
- 错误响应不暴露 token、URL credential、serde 细节或 provider 内部错误。
- 至少一次使用真实阿里云 key 的 smoke test。

### 70/20/10 决策合成尚未实现

当前还没有完整 decision engine，把 70% fundamental、20% trend 和 10% sentiment 合成最终倍率。

MVP 需要补到：

- 新增 decision 领域类型或 crate/module。
- 定义输入快照：
  - investment plan snapshot
  - fundamental signal
  - trend signal
  - sentiment signal
  - 权重配置
  - 降级状态
- 合成综合得分：

```text
score = 0.70 * fundamental + 0.20 * trend + 0.10 * sentiment
```

- 当 AI 不可用时，按设计降级为 `90/10/0` 或明确的 neutral sentiment 策略。
- 将综合得分映射为执行倍率或执行建议。

### 最终总结 / 决策存证尚未实现

演示不能只返回几个分数；需要一份人能看懂的最终 summary。

MVP 需要补到：

- 新增 decision preview API，返回最终演示结果。
- 输出至少包含：
  - plan id / symbol / currency
  - execution status
  - planned contribution
  - 70% fundamental 分数与简短解释
  - 20% trend 分数与简短解释
  - 10% Qwen sentiment 与简短解释
  - final score
  - multiplier / action
  - bucket split
  - degradation 信息
  - 一段最终 summary

示例 summary：

```text
今天是计划执行日。基本面处于偏便宜区间，支持加码；趋势信号中性，未触发接飞刀或追顶保护；
Qwen 新闻情绪略偏正面。综合 70/20/10 后建议按 1.10x 执行，本次计划投入 1100 USD，
其中 880 USD 进入 core bucket，220 USD 进入 opportunity bucket。
```

## 最小演示流程

完整项目 MVP 的演示流程应该是：

1. 启动 PostgreSQL 与 API server。
2. 调用 health/ready，确认服务可用。
3. 创建一个 investment plan，例如 VOO 每月 15 日定投。
4. 查询该计划，展示服务端规范化后的 symbol、currency 和字符串金额。
5. 调用 market sentiment API，后端拉取新闻并调用阿里云 Qwen。
6. 调用或提供 fundamental 输入，得到 70% 基本面分数。
7. 调用或提供 trend 输入，得到 20% 趋势节奏分数。
8. 调用 decision preview API，合成 70/20/10。
9. decision preview 内部复用执行预览，判断今天是否 due。
10. due 时按倍率计算本次计划投入金额。
11. 对投入金额做双桶拆分。
12. 返回最终 summary / 决策存证。

## 当前 PR 能演示的子流程

当前 PR 只覆盖第 9-11 步中的一部分：执行预览 + 双桶拆分。

示例请求：

```http
POST /investment-plans/:id/execution-preview
Content-Type: application/json

{
  "day_of_month": 15,
  "bucket_allocation": {
    "core_ratio": "0.80",
    "opportunity_ratio": "0.20"
  }
}
```

示例响应：

```json
{
  "plan_id": "00000000-0000-0000-0000-000000000001",
  "symbol": "VOO",
  "currency": "USD",
  "schedule_kind": "monthly",
  "schedule_day": 15,
  "day_of_month": 15,
  "status": "due",
  "planned_contribution": "1000",
  "bucket_split": {
    "planned_contribution": "1000",
    "core_contribution": "800.00",
    "opportunity_contribution": "200.00"
  }
}
```

这不是最终 decision summary，只是最终 summary 会依赖的执行层输入之一。

## 后续 PR 建议顺序

1. Trend 20% 行为实现
   - 完成 `evaluate_trend`。
   - 打开并通过当前 ignored 的核心 trend 行为测试。
   - 输出 trend score、regime 和解释字段。

2. 阿里云 Qwen API 接入
   - server config 读取 DashScope 配置。
   - API state 注入 Qwen provider。
   - 新增 market sentiment endpoint。
   - 测试用 fake provider，演示用真实阿里云 key。

3. Decision domain / engine
   - 定义 70/20/10 输入快照。
   - 合成 score。
   - 支持 AI 不可用时降级。
   - 输出 multiplier/action。

4. Decision preview API
   - 组合 investment plan、fundamental、trend、sentiment、execution preview 和 bucket split。
   - 返回最终 summary。
   - 这是整个项目演示 MVP 的核心接口。

5. Demo smoke 文档
   - 给出完整 curl 流程。
   - 写明需要的环境变量。
   - 记录真实阿里云 key 的手动验证步骤。

6. 可选：持久化 decision record
   - 如果演示需要“历史决策存证”，再落 `decisions` 表。
   - 最小后端演示可以先不持久化，只返回 preview。

## MVP 完成判定

只有满足下面条件，才算“整个项目最小 MVP 可演示”：

- 能创建并读取投资计划。
- 能真实调用阿里云 Qwen 得到 sentiment。
- 能得到 70% fundamental 信号。
- 能得到 20% trend 信号。
- 能合成 70/20/10 综合决策。
- 能判断执行日。
- 能计算本次计划投入金额。
- 能输出双桶拆分。
- 能返回一段面向用户的最终 summary。

当前状态更准确地说是：投资计划与双桶执行层接近可演示；70% fundamental 可复用；AI 库层可复用；20% trend、阿里云 API 接入、70/20/10 decision engine 和最终 summary 仍是 MVP 缺口。
