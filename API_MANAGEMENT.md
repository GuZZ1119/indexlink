# IndexLink API 管理清单

本文档用于前后端对接和 MVP 范围管理，记录当前已经可用的 HTTP API、请求/响应约定，以及后续仍需补充的接口。

## 通用约定

- 默认请求和响应均为 JSON。
- 金额、比例、数量等 decimal 字段在 JSON 中使用字符串，例如 `"1000.00"`、`"0.80"`、`"1.00"`。
- UUID 路径参数非法时返回 `400 bad_request`。
- 资源不存在时返回 `404 not_found`。
- 已发送订单但未收到可信回执时返回 `409 order_outcome_unknown`；客户端不得自动重试。
- 服务依赖不可用时返回 `503 service_unavailable`。

统一错误响应：

```json
{
  "error": {
    "code": "bad_request",
    "message": "invalid request"
  }
}
```

## 已有 API

### 健康检查

#### `GET /health`

用于服务存活检查。

响应：

```json
{
  "status": "ok",
  "service": "indexlink-server",
  "version": "0.1.0"
}
```

#### `GET /ready`

用于依赖就绪检查，当前主要检查数据库。

成功响应：

```json
{
  "status": "ready",
  "database": "ok"
}
```

### Investment Plans

#### `POST /investment-plans`

创建投资计划。

请求：

```json
{
  "name": "Core ETF",
  "symbol": "voo",
  "base_contribution": "1000.00",
  "currency": "usd",
  "schedule_kind": "monthly",
  "schedule_day": 15,
  "max_single_execution": "1500.00"
}
```

成功状态码：`201 Created`

响应：创建后的 investment plan。服务端会规范化 `symbol` 与 `currency` 为大写。

#### `GET /investment-plans`

列出所有投资计划。

响应：investment plan 数组。

#### `GET /investment-plans/:id`

按 ID 获取单个投资计划。

#### `PATCH /investment-plans/:id`

更新投资计划。字段均为可选，但不能提交空对象 `{}`。

请求示例：

```json
{
  "name": "Core ETF Plus",
  "base_contribution": "1200.00",
  "schedule_day": 20,
  "max_single_execution": "1800.00",
  "is_active": false
}
```

响应：更新后的 investment plan。

### Execution Preview + 双桶

#### `POST /investment-plans/:id/execution-preview`

预览计划在指定月内日期是否执行，并在 due 时返回可选双桶拆分。

请求：

```json
{
  "day_of_month": 15,
  "bucket_allocation": {
    "core_ratio": "0.80",
    "opportunity_ratio": "0.20"
  }
}
```

响应示例：

```json
{
  "plan_id": "00000000-0000-0000-0000-000000000001",
  "symbol": "VOO",
  "currency": "USD",
  "schedule_kind": "monthly",
  "schedule_day": 15,
  "day_of_month": 15,
  "status": "due",
  "planned_contribution": "1000.00",
  "bucket_split": {
    "planned_contribution": "1000.00",
    "core_contribution": "800.00",
    "opportunity_contribution": "200.00"
  }
}
```

`status` 可选值：

- `due`
- `waiting`
- `inactive`

校验规则：

- `day_of_month` 范围为 `1..=31`。
- `core_ratio` 与 `opportunity_ratio` 都必须在 `0..=1`。
- 两个桶比例合计必须等于 `1`。

### Decision Preview + Paper Broker

#### `POST /investment-plans/:id/decision-preview`

当前最适合前端演示主链路的接口。它会串联：

```text
investment plan
-> execution preview
-> bucket split
-> Qwen market sentiment (with safe fallback)
-> 70/20/10 decision engine
-> optional configured paper order
-> local decision record
-> summary
```

请求：

```json
{
  "day_of_month": 15,
  "bucket_allocation": {
    "core_ratio": "0.80",
    "opportunity_ratio": "0.20"
  },
  "fundamental": {
    "score": 0.10,
    "cape_percentile": 0.10,
    "erp_percentile": 0.90
  },
  "trend": {
    "score": 0.50,
    "ma_distance_percentile": 0.50,
    "rsi_percentile": 0.50,
    "vix_percentile": 0.50,
    "regime": "neutral"
  },
  "paper_order": {
    "idempotency_key": "decision-preview-demo-1",
    "side": "buy",
    "order_type": "market",
    "quantity": "1.00"
  }
}
```

响应包含：

- `execution`：执行预览和双桶拆分。
- `decision`：`final_score`、`multiplier`、`action`、`weight_mode` 和分层 score。
- `paper_order_ack`：只有 due 且 action 可执行时才出现。
- `summary`：演示级摘要。

`sentiment` 不是请求字段。后端会在决策前自动拉取 CNBC RSS 并调用已配置的 Qwen provider；成功时使用 `70/20/10` 并将原始情绪快照写入本地 decision record。未配置 Key 或新闻/AI provider 暂时不可用时，接口仍会安全完成 preview，但 `decision.weight_mode` 为 `sentiment_unavailable`，引擎使用 `90/10/0` 降级权重且不提交任何伪造的情绪快照。手工 `sentiment` 字段会返回 `400 bad_request`，不能绕过该链路。

`decision.action` 可选值：

- `overweight`
- `standard`
- `tactical_delay`
- `underweight`
- `skip`

`decision.weight_mode` 可选值：

- `normal`
- `sentiment_unavailable`

`trend.regime` 请求值：

- `neutral`
- `overheated`
- `falling_knife`

`paper_order` 规则：

- `paper_order` 可省略；省略时只做 preview，不提交订单。
- 只有 `execution.status == "due"` 且 action 不是 `skip` / `tactical_delay` 时才提交配置的 paper order。
- 即使不会提交订单，只要请求中带了非法 `paper_order`，也会返回 `400 bad_request`。
- broker port 调用有 5 秒超时保护。

### Decision Record / History

#### `GET /investment-plans/:id/decisions`

列出一个已存在投资计划的历史 decision record，按 `created_at DESC, id DESC` 返回。

- `limit` 可选，默认 `50`，有效范围为 `1..=200`。
- 非法 plan UUID、非法 query 参数或越界 `limit` 返回 `400 bad_request`。
- 不存在的 investment plan 返回 `404 not_found`。
- Decision Preview 会自动创建本地审计记录；只读 history API 返回这些已持久化快照。可提交的 paper order 会先存订单意图，收到 broker ack 后再补写回执，避免存储故障把已提交订单伪装成可安全重试。

请求示例：

```text
GET /investment-plans/00000000-0000-0000-0000-000000000001/decisions?limit=20
```

响应是 decision record 数组。每条记录包含 execution、fundamental、trend、可选 sentiment、decision 与可选 broker 的快照，以及最终 summary 和创建时间。

#### `GET /decisions/:id`

按 ID 查询单条 decision record。不存在时返回 `404 not_found`。

## Market Sentiment API

### 阿里云 Qwen Market Sentiment API

#### `POST /market-sentiment/preview`

后端拉取 CNBC RSS 新闻并调用 DashScope/OpenAI-compatible Qwen，返回有界情绪值。设置 `DASHSCOPE_API_KEY` 后由 server 在启动时构造并注入真实 provider；未设置 Key 时 server 仍可启动，但本路由返回统一的 `503 service_unavailable`，不暴露 provider、URL 或凭据细节。`Decision Preview` 会自动复用同一条 pipeline：成功时将情绪作为 10% 输入，失败时显式降级为 `90/10/0`。

响应字段：

- `score`：`[-1.0, 1.0]` 内的情绪分数。
- `label`：`positive`、`neutral` 或 `negative`，由分数正负确定。

本阶段刻意不返回 LLM 自由文本解释、新闻正文、Key、provider URL 或模型内部错误。后续 structured-output PR 再补受控 explanation 与来源摘要，避免把未经约束的模型文本直接纳入 API 契约。

本地真实 Key smoke（不要把 Key 写入仓库或终端输出）：

```bash
read -r -s DASHSCOPE_API_KEY
export DASHSCOPE_API_KEY
cargo test -p ai-client --test news real_cnbc_with_qwen -- --ignored --nocapture
```

HTTP smoke：在同一终端环境启动 `cargo run -p indexlink-server` 后，执行：

```bash
curl -X POST http://127.0.0.1:8080/market-sentiment/preview
```

## Quant Signal APIs

### Automatic Market Signal Input API

#### `GET /signals/market-input/:symbol`

读取并组装当前计划标的的自动信号输入，供 Dashboard 填充既有 Fundamental/Trend Preview 表单；该端点只读，不会创建订单、不会访问交易账户，也不会保存密钥。server 必须已配置本机 loopback OpenD；未配置或任一数据源不可用时，统一返回安全的 `503 service_unavailable`。

- 价格与技术层：本机 OpenD 的美股日线，后端本地计算 MA200 distance 与 14 日 RSI，并按月保留最近 60 个快照。
- 基本面层：公开 Shiller CAPE 月度表；ERP 明确使用代理口径 `100 / CAPE - 美国财政部 10 年期国债收益率`，不是前瞻盈利预测。
- 波动层：Cboe 公开 VIX 历史 CSV，按每月最后一个可用观测值保留最近 60 个快照。

响应含 `fundamental`、`trend`、`as_of` 与来源说明。页面只在用户点击醒目的“自动拉取市场信号”按钮后请求；返回值仍展示在可编辑字段中，用户可在运行 Decision Preview 前审查。实际执行时，同一输入最终会随 Decision Record 保存到本地 SQLite 审计快照。

### Fundamental Signal API

#### `POST /signals/fundamental/preview`

用调用方提供的月度 CAPE/ERP 历史快照计算 70% 基本面层信号。历史数组须按旧到新排列，默认至少 `60` 个有效月度样本；不满足领域校验或出现未知字段时返回统一 `400 bad_request`。

请求字段：`cape_history`、`cape_current`、`erp_history`、`erp_current`。

响应字段：

- `score`：基本面综合分数，`0` 表示历史相对便宜、`1` 表示历史相对昂贵。
- `cape_percentile`、`erp_percentile`：用于 decision record 和演示解释的原始审计分位。

### Trend Signal API

#### `POST /signals/trend/preview`

用调用方提供的月度 MA200 distance、RSI、VIX 历史快照计算 20% 趋势层信号。历史数组须按旧到新排列，默认至少 `60` 个有效月度样本；不满足领域校验或出现未知字段时返回统一 `400 bad_request`。

请求字段：`ma_distance_history`、`ma_distance_current`、`rsi_history`、`rsi_current`、`vix_history`、`vix_current`。

响应字段：

- `score`：趋势综合分数。
- `ma_distance_percentile`、`rsi_percentile`、`vix_percentile`：原始审计分位。
- `regime`：`neutral`、`overheated` 或 `falling_knife`。

这两个端点不保存调用方请求本身；无论来自手工输入、JSON 导入还是自动市场快照，只有最终提交的 Decision Preview 输入会作为审计记录保存到本地 SQLite。

### Futu/Moomoo OpenD Paper Trading API

已具备 broker port、MockBroker、OpenD raw TCP paper session 与下单 adapter。server 未设置 `OPEND_PROVIDER` 时保留 MockBroker；设置 `futu` 或 `moomoo` 后，server 在启动时连接本机 loopback OpenD 并注入真实 `OpenDPaperBroker`。启动失败会安全失败，绝不会静默降级到 mock broker。

真实 OpenD 下单暂不需要单独 HTTP endpoint；它复用 `POST /investment-plans/:id/decision-preview` 的 `paper_order`，以确保订单必须经过计划、执行日和决策保护。

#### `GET /paper-portfolio`

读取当前已配置 OpenD 模拟账户的 USD 资金、当前美股持仓和近期美股订单状态，供 Dashboard 展示账户净资产、现金、证券市值、持仓盈亏、持仓与订单。它只调用 OpenD 的资金、持仓和订单读取协议；不会下单、撤单、改价、解锁交易，也不会返回 account id、登录凭据或 provider 原始错误。

请求需要已配置并成功初始化 `OPEND_PROVIDER`。未配置、OpenD 不可用、返回账户/环境不匹配或响应不完整时统一返回 `503 service_unavailable`。路由使用 OpenD 的强制缓存刷新读取最新状态，应只由用户点击“刷新模拟账户”触发，而不是高频轮询。

OpenD 模拟账户不支持独立成交列表与现金流记录；IndexLink 因此不把 `accepted` 伪装为成交，而是在本地账本中根据后续订单的累计成交数量、累计均价和订单状态增量生成可审计的本地 fill。该来源会在 Dashboard 明确标记，且只覆盖账本启用后由 IndexLink 接受并持续观察的订单。

#### `PUT /investment-plans/:id/paper-performance/opening-balance`

保存用户确认的本地模拟账户起始资金基准。请求体：

```json
{
  "amount": "10000.00",
  "occurred_at": "2026-07-19T10:00:00.000Z"
}
```

金额必须为非负 decimal 字符串；时间必须为 UTC RFC3339 毫秒格式。它只写入本机 SQLite 的 `cash_flows`，不会访问 OpenD、下单或修改模拟账户资金。每个 plan 仅保留一条可覆盖的 `opening_balance`；没有该基准时服务不会声称计算了总收益。

#### `GET /investment-plans/:id/paper-performance`

只读刷新 OpenD 资金、持仓和近期订单状态，然后将已知订单的成交增量、FIFO 持仓成本和本次估值写入本机 SQLite。响应包含：

- `net_contributions`、`realized_pnl`、`unrealized_pnl`、`total_return`；
- `adaptive_value` 与同一执行价下的 `plain_dca_value`；
- 用于 Dashboard 曲线的本地 `points`；
- `data_complete`，只有本地观察到的 FIFO 数量与当前 provider 持仓数量一致且已配置起始基准时才为 `true`。

普通定投基准为每个已观察到的买入订单，在该订单首次成交价按计划 `base_contribution` 买入的假想仓位；它不使用未来价格，也不伪造未观察到的历史成交。当前限制仍然是：历史数据从本地账本启用后开始积累，不能反推出启用前的完整模拟账户交易/入金历史；多计划共享同一模拟账户时，需分别持续追踪各计划发出的订单，不能把整个账户余额随意归因给某一计划。

本地启用配置：

```bash
OPEND_PROVIDER=futu
OPEND_HOST=127.0.0.1
OPEND_PORT=11111
OPEND_ACCOUNT_ID='<paper-account-id>'
```

- 配置仅接受 `futu` / `moomoo` 和 loopback host（`127.0.0.1`、`::1`、`localhost`）。
- server 配置层只构造 `Paper` adapter；没有 live environment 或 live gate 配置项。
- 未设置 `OPEND_PROVIDER` 时，演示继续使用 paper-only `MockBroker`。
- 真实 smoke 是忽略式测试，必须显式确认且提供唯一 idempotency key、symbol 与 quantity；它不读取、不传输 OpenD 登录密码或 token。

真实 smoke 前先在 OpenD GUI 中登录并确认选择的是虚拟账户；以下命令会提交一笔 paper market order，不应在 CI 中执行：

```bash
export OPEND_PROVIDER=futu
export OPEND_HOST=127.0.0.1
export OPEND_PORT=11111
read -r -p 'Paper account ID: ' OPEND_ACCOUNT_ID
export OPEND_ACCOUNT_ID
read -r -p 'Unique idempotency key: ' OPEND_SMOKE_IDEMPOTENCY_KEY
export OPEND_SMOKE_IDEMPOTENCY_KEY
read -r -p 'US symbol: ' OPEND_SMOKE_SYMBOL
export OPEND_SMOKE_SYMBOL
read -r -p 'Quantity: ' OPEND_SMOKE_QUANTITY
export OPEND_SMOKE_QUANTITY
OPEND_SMOKE_CONFIRM=submit-paper-order \
  cargo test -p indexlink-server real_opend_paper_order_smoke -- --ignored --nocapture
```

### Decision Preview 输入与摘要

当前 `POST /investment-plans/:id/decision-preview` 已由后端自动获取 Qwen market sentiment，并将 execution、fundamental、trend、可选 sentiment、decision 和 broker 快照写入本地 SQLite decision record。fundamental 与 trend 由调用方传入，可先用上述预览端点的响应直接填充。

`summary` 已按 execution、计划金额、基本面、趋势和 regime、Qwen 情绪/降级权重、最终分数、倍率/action、双桶拆分和 paper-order 状态给出稳定分层解释。输入快照不得包含 Qwen API key、OpenD 密码、account id、token 或其他 secret。

## 前端当前建议对接顺序

1. `GET /health`、`GET /ready`
2. `POST /investment-plans`
3. `GET /investment-plans`
4. `GET /investment-plans/:id`
5. `PATCH /investment-plans/:id`
6. `POST /investment-plans/:id/execution-preview`
7. `POST /investment-plans/:id/decision-preview`
8. `POST /signals/fundamental/preview`
9. `POST /signals/trend/preview`
10. `GET /investment-plans/:id/decisions`
11. `GET /decisions/:id`

## 当前 MVP 缺口优先级

1. 使用真实 DashScope Key 完成一次本机 Qwen network smoke。
2. 使用 Futu/Moomoo 虚拟账户完成一次真实 OpenD paper-order smoke。
3. 由前端或一个经确认的数据供应商完成 CAPE、ERP、MA200 distance、RSI、VIX 月度快照的受控采集；本服务端已提供计算与校验接口，但不会在未指定供应商的情况下擅自抓取或持久化第三方行情数据。
