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
  -> Broker paper trading
  -> 最终总结 / 决策存证
```

当前最小 MVP 的后端底座正在逐步补齐；真正的最小 MVP 需要能把投资计划、量化信号、AI 情绪、双桶拆分、最终解释和演示级前端串成一个可演示闭环。

## MVP 边界

最小 MVP 要求产品演示可用，不要求完整生产交易系统。

必须包含：

- 单用户投资计划管理。
- 70/20/10 决策模型的可演示输出。
- 阿里云 DashScope/Qwen 的真实 sentiment 调用路径。
- 执行日预览与双桶金额拆分。
- Broker paper trading demo，用虚拟账户验证从决策到订单提交的闭环。
- 一份最终 summary，说明为什么今天建议执行、跳过、标准执行或加/减码。
- 演示级最小前端，用于串起计划、信号、决策和双桶结果。

暂不包含：

- 默认开启真实券商下单。
- 自动 Scheduler。
- 订单状态机与成交回报。
- 多用户权限。
- 产品级完整前端。
- 生产级行情源和缓存策略。

## Broker 接入方向：Futu / Moomoo OpenD

Futu 与 Moomoo 的官方 API 都采用 OpenD 网关模式：OpenD 在本地或云端运行，通过 TCP 协议转发到 Futu/Moomoo 服务端；官方 SDK 覆盖 Python、Java、C#、C++、JavaScript，也可以直接按协议接入。官方文档也说明 trading interface 同时用于 live trading 和 paper trading。参考：[Futu API introduction](https://openapi.futunn.com/futu-api-doc/en/intro/intro.html)、[Moomoo API introduction](https://openapi.moomoo.com/moomoo-api-doc/en/intro/intro.html)。

MVP 接入策略：

- 第一阶段只做 provider-neutral broker port 与 mock broker。
- 第二阶段接 Futu/Moomoo OpenD adapter，优先使用 paper trading / virtual account。
- 第三阶段在 demo 中展示“decision preview -> broker paper order -> order ack”。
- 真实账户 live trading 必须显式开启配置，默认关闭。
- live trading 不作为最小 MVP 的默认演示路径；它是 MVP 后的受保护扩展路径。

安全边界：

- 所有下单请求必须带 idempotency key。
- 默认环境必须是 paper trading。
- live trading 需要显式配置开关和人工确认。
- 错误响应与日志不得暴露 broker 登录信息、account id、OpenD 密码或 token。
- Futu/Moomoo adapter 只能实现 broker port，不应污染 decision / quant 的纯函数层。

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

### Broker 边界

- 已新增 provider-neutral broker port。
- 已支持 `Paper` / `Live` 两种执行环境。
- 已支持 market / limit order request 的基础不变量校验。
- 已新增 Futu/Moomoo OpenD 连接配置模型，覆盖 provider、host、port、paper/live 环境和可选 account id。
- OpenD 配置默认不允许 live trading；live orders 需要配置环境匹配且显式开启 live gate。
- OpenD 配置的 account id 可供 adapter 使用，但 debug 输出会脱敏，避免进入日志。
- 已新增 OpenD paper broker adapter，作为 `BrokerClient` 实现接入 Futu/Moomoo paper trading gateway。
- OpenD paper adapter 会在调用 gateway 前拒绝 live config 和 live order，保证真实 transport 前仍有安全闸门。
- 已支持 mock broker，用于虚拟账号 demo 前的本地闭环测试。
- mock broker 默认拒绝 live orders，必须显式开启才接受 live-mode 请求。

### 70% 基本面量化

- `quant-engine` 已有 fundamental 方向的实现和测试。
- 已覆盖 CAPE、ERP、历史分位、权重、错误边界和审计字段。
- 这部分可以作为 70% 主信号进入后续 decision preview。

### 20% 趋势节奏

- `quant-engine` 已实现 `evaluate_trend`。
- 支持 MA200 distance、RSI、VIX 三个技术指标的指数加权分位。
- MA/RSI 原始分位高代表过热，进入 score 时反向计入。
- VIX 原始分位高代表恐慌/急跌，进入 score 时正向计入。
- 支持 `Overheated`、`Neutral`、`FallingKnife` 三种趋势体制。
- `FallingKnife` 与 `Overheated` 同时满足时，优先判定 `FallingKnife`。
- trend 行为测试默认运行，不再依赖 ignored TDD 测试。

### 70/20/10 Decision Engine

- 已新增 `decision-engine` crate，保持纯函数、零 IO。
- 已定义 70/20/10 输入快照、权重配置、权重降级状态和决策输出。
- 已将 fundamental、trend、sentiment 合成为 final score、multiplier 和 action。
- AI sentiment 可用时使用默认 `70/20/10`；不可用时降级为 `90/10/0`。
- trend 非中性体制会触发 `TacticalDelay`，用于避免过热追高或接飞刀。

### 10% AI 感知库层

- 已有 DashScope/OpenAI-compatible Qwen client。
- 已有 CNBC RSS 新闻源。
- 已有新闻格式化为 sentiment prompt 的逻辑。
- 已有“拉取新闻 -> 调用 AI provider -> 返回 sentiment”的库层 pipeline。
- 但还没有接入 server config、API state 和 HTTP route。

## 当前明确缺口

### 阿里云 Qwen 还没接到 API

当前 AI 只是 library-ready，不能直接从 API 演示。

MVP 需要补到：

- server config 读取 `DASHSCOPE_API_KEY`。
- 支持配置 Qwen model、base URL、timeout、temperature 和 max tokens。
- API state 注入 Qwen provider 与 news source。
- 新增 market sentiment API。
- 错误响应不暴露 token、URL credential、serde 细节或 provider 内部错误。
- 至少一次使用真实阿里云 key 的 smoke test。

### Decision Preview API 尚未实现

当前已有 decision engine 纯函数，但还没有 API 将投资计划、执行预览、双桶拆分、broker paper order 和最终 summary 串成一个演示闭环。

MVP 需要补到：

- 新增 decision preview API。
- 从 API DTO 接收或组装 investment plan snapshot、fundamental signal、trend signal、sentiment signal 和双桶配置。
- 调用 `decision-engine` 合成 final score、multiplier 和 action。
- 先用 `MockBroker` 完成 decision-to-paper-order 闭环。
- 返回最终 summary 与 paper order ack。

### Futu/Moomoo OpenD transport 尚未实现

当前 broker crate 已定义 provider-neutral port、mock broker、OpenD 配置模型和 OpenD paper adapter，但还没有真正实现 Futu/Moomoo OpenD 的 TCP/SDK transport。

MVP 需要补到：

- server config 读取 OpenD host、port、目标 broker provider 和 paper/live mode，并映射到已校验的 OpenD 配置模型。
- 新增 Futu/Moomoo OpenD gateway transport，实现 `OpenDOrderGateway`。
- paper trading 下支持提交最小 market/limit order。
- 下单返回 broker order id 与初始状态。
- API 层仅暴露 paper trading demo 所需的安全信息。

### 演示级最小前端尚未实现

完整产品级前端不属于 MVP，但演示级最小前端属于 MVP。它不需要复杂设计系统、登录、多账户或图表编辑能力，只需要能把核心后端链路串起来。

实现分工：演示级前端由 Jame 负责；当前后端工作流只提供 API 契约、配置、安全边界和测试，不修改前端代码。

MVP 需要补到：

- Investment Plans 页面：
  - 创建计划。
  - 列表展示计划。
  - 查看计划详情。
  - 更新金额、执行日和启停状态。
- Execution Preview 页面：
  - 选择计划。
  - 输入 `day_of_month`。
  - 输入 core/opportunity 比例。
  - 展示 due/waiting/inactive、planned contribution 和双桶拆分。
- Market Sentiment 页面：
  - 触发后端调用阿里云 Qwen。
  - 展示 sentiment score、label、简短解释和新闻来源。
- Decision Summary 页面：
  - 展示 70% fundamental、20% trend、10% sentiment。
  - 展示 final score、multiplier/action、bucket split。
  - 展示最终 summary。
- Broker Paper Trading 页面或区块：
  - 展示本次 paper order request。
  - 展示 broker order ack。
  - 明确标注当前是 virtual account / paper trading。
  - live trading 未开启时显示受保护状态。

前端可以先使用后端真实 API；如果某个后端 endpoint 尚未落地，前端对应区域可以先显示“等待后端接口”占位，但最终 MVP 演示前必须替换为真实 API。

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
12. 通过 broker port 提交 paper order。
13. 返回 broker order ack。
14. 返回最终 summary；如需存证则附带 decision record。
15. 前端展示同一条链路：计划、三类信号、最终建议、双桶金额、paper order 和 summary。

## 已具备的执行预览子流程

双桶执行预览 API 覆盖第 9-11 步中的一部分：执行预览 + 双桶拆分。

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

1. Decision preview API + MockBroker 串联
   - 组合 investment plan、fundamental、trend、sentiment、execution preview 和 bucket split。
   - 先用 `MockBroker` 完成 decision-to-paper-order 闭环。
   - 返回最终 summary 与 paper order ack。

2. 阿里云 Qwen API 接入
   - server config 读取 DashScope 配置。
   - API state 注入 Qwen provider。
   - 新增 market sentiment endpoint。
   - 测试用 fake provider，演示用真实阿里云 key。

3. Futu/Moomoo OpenD paper adapter
   - 已完成 broker port adapter 和安全闸门。
   - 下一步补真实 OpenD gateway transport。
   - 读取 server OpenD host/port 与 paper mode 配置。

4. Broker paper execution API
   - decision preview 之后可提交 paper order。
   - 返回 broker order ack。
   - 使用 idempotency key 防止重复提交。

5. 演示级最小前端（Jame 负责）
   - 实现计划列表/创建/详情。
   - 实现 execution preview + bucket split 展示。
   - 实现 market sentiment 与 decision summary 展示。
   - 实现 paper order ack 展示。

6. Demo smoke 文档
   - 给出完整 curl 流程。
   - 给出前端点击演示流程。
   - 写明需要的环境变量。
   - 记录真实阿里云 key 和 Futu/Moomoo paper trading 的手动验证步骤。

7. 可选：持久化 decision record
   - 如果演示需要“历史决策存证”，再落 `decisions` 表。
   - 最小后端演示可以先不持久化，只返回 preview。

8. 可选：受保护 live trading
   - 只在 paper trading demo 稳定后考虑。
   - 需要显式配置、人工确认和更严格审计。

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
- 能通过 Futu/Moomoo paper trading 或 mock broker 提交虚拟订单并返回 ack。
- 能返回一段面向用户的最终 summary。
- 能在演示级前端展示上述完整链路。

当前状态更准确地说是：投资计划与双桶执行层接近可演示；70% fundamental 可复用；20% trend 已可复用；70/20/10 decision engine 已可复用；AI 库层可复用；broker port、mock broker 与 OpenD paper adapter 可复用；Decision Preview API、阿里云 API 接入、Futu/Moomoo OpenD transport、最终 summary 和演示级前端仍是 MVP 缺口。
