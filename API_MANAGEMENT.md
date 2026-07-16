# IndexLink API 管理清单

本文档用于前后端对接和 MVP 范围管理，记录当前已经可用的 HTTP API、请求/响应约定，以及后续仍需补充的接口。

## 通用约定

- 默认请求和响应均为 JSON。
- 金额、比例、数量等 decimal 字段在 JSON 中使用字符串，例如 `"1000.00"`、`"0.80"`、`"1.00"`。
- UUID 路径参数非法时返回 `400 bad_request`。
- 资源不存在时返回 `404 not_found`。
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

### Decision Preview + MockBroker

#### `POST /investment-plans/:id/decision-preview`

当前最适合前端演示主链路的接口。它会串联：

```text
investment plan
-> execution preview
-> bucket split
-> 70/20/10 decision engine
-> optional MockBroker paper order
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
  "sentiment": {
    "score": 0.80
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

- `paper_order` 可省略；省略时只做 preview，不提交 mock order。
- 只有 `execution.status == "due"` 且 action 不是 `skip` / `tactical_delay` 时才提交 mock paper order。
- 即使不会提交订单，只要请求中带了非法 `paper_order`，也会返回 `400 bad_request`。
- broker port 调用有 5 秒超时保护。

### Decision Record / History

#### `GET /investment-plans/:id/decisions`

列出一个已存在投资计划的历史 decision record，按 `created_at DESC, id DESC` 返回。

- `limit` 可选，默认 `50`，有效范围为 `1..=200`。
- 非法 plan UUID、非法 query 参数或越界 `limit` 返回 `400 bad_request`。
- 不存在的 investment plan 返回 `404 not_found`。
- 当前返回已经持久化的审计快照；Decision Preview 自动创建 record 仍属于后续工作，因此新环境可能返回空数组。

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

后端拉取 CNBC RSS 新闻并调用 DashScope/OpenAI-compatible Qwen，返回可作为后续 70/20/10 决策链路输入的有界情绪值。设置 `DASHSCOPE_API_KEY` 后由 server 在启动时构造并注入真实 provider；未设置 Key 时 server 仍可启动，但本路由返回统一的 `503 service_unavailable`，不暴露 provider、URL 或凭据细节。当前 `Decision Preview` 仍使用调用方传入的 sentiment，尚未自动调用本路由。

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

## 待补充 API

### Fundamental Signal API

当前 `quant-engine` 已有 fundamental 计算能力，但没有 HTTP API。

建议新增：

#### `POST /signals/fundamental/preview`

目标：

- 输入当前 CAPE、ERP 与历史序列。
- 返回 70% fundamental score 和审计字段。

### Trend Signal API

当前 `quant-engine` 已有 trend 计算能力，但没有 HTTP API。

建议新增：

#### `POST /signals/trend/preview`

目标：

- 输入 MA200 distance、RSI、VIX 与历史序列。
- 返回 20% trend score、regime 和审计字段。

### Futu/Moomoo OpenD Paper Trading API

当前已具备 broker port、MockBroker、OpenD 配置模型和 OpenD paper adapter，但尚未实现真实 OpenD gateway transport，也没有单独 HTTP API。

建议新增：

#### `POST /broker/paper-orders`

目标：

- 通过 Futu/Moomoo OpenD paper trading 提交虚拟账户订单。
- 返回 broker order ack。
- 默认只支持 paper trading。

必须保持：

- live trading 默认关闭。
- request 必须带 idempotency key。
- 错误不得暴露 account id、token、OpenD 密码或内部连接细节。

### Decision Preview 真实上游升级

当前 `POST /investment-plans/:id/decision-preview` 的 `fundamental`、`trend`、`sentiment` 由调用方传入。

后续可新增或扩展：

#### `POST /investment-plans/:id/decision-preview/live`

目标：

- 后端自动获取 Qwen sentiment。
- 后端自动获取或计算 fundamental/trend signal。
- 可选择使用真实 OpenD paper gateway 替代 MockBroker。
- 返回更完整的分层解释 summary。

### Decision Record 自动存证

Decision record 的 SQLite 本地 storage adapter 与只读 history API 已具备；当前尚未在 `POST /investment-plans/:id/decision-preview` 成功后自动写入 record。PostgreSQL adapter 保留为兼容实现，默认运行时不使用。

后续需要在受控的服务端编排层创建 record，并保存：

- fundamental、trend、sentiment 输入快照。
- execution preview、bucket split、decision 输出与 summary。
- 可选 broker request / ack。

输入快照不得包含 Qwen API key、OpenD 密码、account id、token 或其他 secret。

## 前端当前建议对接顺序

1. `GET /health`、`GET /ready`
2. `POST /investment-plans`
3. `GET /investment-plans`
4. `GET /investment-plans/:id`
5. `PATCH /investment-plans/:id`
6. `POST /investment-plans/:id/execution-preview`
7. `POST /investment-plans/:id/decision-preview`
8. `GET /investment-plans/:id/decisions`
9. `GET /decisions/:id`

## 当前 MVP 缺口优先级

1. 阿里云 Qwen Market Sentiment API。
2. Fundamental/Trend signal API，或明确由前端 demo 手工输入 signal。
3. Futu/Moomoo OpenD 真实 paper trading transport。
4. Decision Preview 接真实 Qwen 与真实 OpenD paper gateway。
5. Decision Preview 自动写入 Decision Record。
