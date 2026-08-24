# IndexLink 策略工作台迁移计划 / Strategy Studio Migration Plan

> 状态：已预登记，尚未开始实现。
> Status: pre-registered; implementation has not started.

## 1. 新定位 / New Positioning

IndexLink 将从单一的“70/20/10 自适应定投模型”演进为一个**透明、可审计、可扩展的个人量化定投策略工作台与 paper-trading 执行平台**。

IndexLink will evolve from a single “70/20/10 adaptive DCA model” into a **transparent, auditable, extensible personal quantitative strategy studio and paper-trading execution platform**.

产品不承诺策略必然跑赢固定定投或市场。它的核心价值是让使用者能够以相同的市场数据、资金流、成本假设和执行时点，创建、验证、比较、激活、执行和回放策略。

The product does not promise that a policy will outperform fixed DCA or the market. Its value is enabling users to create, validate, compare, activate, execute, and replay policies under matched market data, cash flows, cost assumptions, and execution timing.

目标生命周期：

```text
创建策略 → 验证 → 回测 → 审阅 → 保存版本 → 激活 → 调度
→ 评估 → Paper 执行 → 监控 → 审计
```

Target lifecycle:

```text
Create Strategy → Validate → Backtest → Review → Save Version → Activate
→ Schedule → Evaluate → Paper Execute → Monitor → Audit
```

## 2. 迁移原则与非目标 / Migration Principles and Non-goals

1. **不删除历史研究。** 现有 70/20/10、C1–C4、校准数据和报告保留为可复现实验资产；它们不再构成收益承诺。
2. **确定性运行时优先。** 相同的策略版本、完整上下文和执行时点必须产生相同推荐；策略运行时不得发起网络请求、读取环境变量或直接下单。
3. **策略与基础设施解耦。** API、scheduler、broker、SQLite 和 OpenD 不应理解 CAPE、ERP、RSI、VIX、70/20/10 或 `TacticalDelay` 的内部含义。
4. **固定 DCA 是公平基准。** `FixedDcaPolicy` 是后续新计划的默认候选，并是所有策略研究的匹配对照；已有计划不会被静默改变。
5. **AI 不是交易授权者。** Qwen 只能生成策略候选、解释、风险提示和变化摘要；它不能改变已经验证的策略逻辑，不能绕过金额/环境/人工确认边界，也不能直接发单。
6. **限定表达能力。** 自定义策略仅允许受限 DSL/AST、白名单指标和白名单动作；不执行用户代码，不支持任意脚本、实盘自动交易或云端多用户同步。

## 3. 目标领域边界 / Target Domain Boundaries

### 3.1 新增稳定契约 / New Stable Contract

新增一个无 IO 的策略领域 crate（建议名称：`strategy-policy`）。它定义：

| 类型 / Type | 用途 / Purpose |
| :--- | :--- |
| `PolicyId` | 已校验、稳定的策略标识。 |
| `PolicyVersion` | 不可变策略版本。 |
| `PolicyRef` | `id + version` 的激活绑定。 |
| `DecisionContext` | 已解析的执行日期、预算、计划约束、市场证据和 `as_of`；不得包含 IO。 |
| `InvestmentRecommendation` | 推荐金额、桶拆分、动作、原因、风险提示、证据摘要和策略引用。 |
| `InvestmentPolicy` | `DecisionContext -> InvestmentRecommendation` 的确定性评估契约。 |

The proposed `strategy-policy` crate is pure and IO-free. Its runtime contract is:

```text
PolicyRef + complete DecisionContext → InvestmentRecommendation
```

`DecisionContext` contains resolved evidence rather than a database connection, HTTP client, Qwen client, or broker. The same runtime must therefore power both historical evaluation and live preview.

### 3.2 内置策略 / Built-in Policies

| 策略 / Policy | 状态 / Status | 行为 / Behaviour |
| :--- | :--- | :--- |
| `CoreOpportunityV1` | 现有逻辑的兼容包装器 | 包装当前 70/20/10、双桶和动作语义，输出保持逐项回归兼容。 |
| `FixedDcaPolicy` | 首个新增策略 | 固定按周期预算推荐金额，用作新计划默认候选和公平基准。 |
| DSL 策略 | 后续 | 由用户保存的、受限语法的规则策略。 |

`CoreOpportunityV1` will call the existing legacy decision implementation unchanged during the first migration stage. No existing public `decision-engine` type is renamed or removed in that stage.

### 3.3 策略内部证据 / Internal Evidence

CAPE、ERP、MA、RSI、VIX、Qwen 情绪和 `TacticalDelay` 是 `CoreOpportunityV1` 的内部证据或标签，不是平台级 API/Broker 语义。后续 DSL 初始仅支持价格、SMA、EMA、RSI、回撤和 VIX；动作仅包括 `BuyFixedAmount` 与 `Skip`。

## 4. 当前耦合审计 / Current Coupling Audit

当前硬耦合集中在应用编排层，而非全部仓库：

| 位置 / Location | 当前职责 / Current coupling | 迁移方式 / Migration treatment |
| :--- | :--- | :--- |
| `crates/decision-engine` | 70/20/10 合成、倍率与 `TacticalDelay` | 首期冻结并由 `CoreOpportunityV1` 包装；后续不再作为唯一策略入口。 |
| `crates/api/src/routes/decision_preview.rs` | 直接构造旧输入、调用旧引擎、创建记录和可选订单 | 改为调用 policy resolver；保留旧 HTTP 字段直到版本化 API 完成。 |
| `crates/investment-plans` | 到期预览、预算与双桶金额语义 | 接收通用推荐金额/拆分；旧计划保留旧行为。 |
| `crates/decision-records` 与 SQLite adapter | 保存 70/20/10 输入快照与订单回执 | 追加策略 ID、版本、快照、哈希和通用证据；旧列与旧记录继续可读。 |
| `apps/server` scheduler | 调用固定决策入口 | 迁移为调用 policy resolver，同时保留幂等 claim 与“不自动下单”边界。 |
| `broker` / OpenD | paper-only 订单提交 | 保持不变；只接收已验证的订单请求。 |

## 5. 向后兼容与数据迁移 / Backward Compatibility and Data Migration

1. 已有计划在数据库迁移中显式回填为 `CoreOpportunityV1@1`，不会在升级后自动变为固定 DCA。
2. 新建计划在产品确认后才将 `FixedDcaPolicy@1` 作为默认候选；这一默认值变化必须有 API、UI 和迁移测试。
3. 旧 Decision Preview 响应暂时保留；新响应以附加 `policy` 与 `recommendation` 字段的方式演进，避免破坏前端。
4. `decision_records` 采用追加字段/迁移：`policy_id`、`policy_version`、`policy_snapshot`、`policy_hash`、`evidence_snapshot`、`recommendation_snapshot`。既有 70/20/10 快照不删除。
5. scheduler 幂等键、机会现金池、周期预算预留、paper-only 环境限制和人工确认下单边界必须逐项回归验证。
6. C1–C4 和既有回测报告只作为研究工件保存，不被改写为新策略的业绩宣传。

## 6. 小步实施计划 / Small-PR Delivery Plan

### PR 1 — 策略契约与 Legacy 包装 / Policy Contract and Legacy Wrapper

- 新增 `strategy-policy` 的标识、版本、上下文、推荐与 trait。
- 新增 `CoreOpportunityV1` 适配器，调用现有函数并做逐项输出回归测试。
- 不改数据库、HTTP、scheduler、OpenD 或默认行为。

### PR 2 — 固定 DCA 与统一解析入口 / Fixed DCA and Unified Resolver

- 增加 `FixedDcaPolicy` 与内置策略 registry。
- 为计划增加最小的内置策略绑定，并通过 SQLite migration 将旧计划回填到 Legacy。
- 将手动预览和 scheduler 的应用入口改为 resolver；验证固定 DCA 与 Legacy 都能走同一 paper-only 闭环。

### PR 3 — 策略版本领域与审计升级 / Strategy Version Domain and Audit Upgrade

- 定义 `StrategySpec`、版本状态、激活引用和哈希规则。
- 追加保存策略快照、证据快照和 recommendation 快照；旧记录继续读取。

### PR 4 — 受限 DSL/AST 与校验 / Restricted DSL/AST and Validation

- 实现 `IndicatorSpec`、表达式、条件和动作白名单。
- 拒绝未知指标、未定义变量、除零、越界金额、递归和任意代码执行。

### PR 5 — 确定性 DSL Runtime / Deterministic DSL Runtime

- 将完整 `DecisionContext` 解释为 `InvestmentRecommendation`。
- 为金额守恒、`as_of`、无未来函数、可重复性与错误信息建立聚焦测试。

### PR 6 — 统一历史评估 / Unified Historical Evaluation

- 让 `strategy-evaluation` 使用同一 policy runtime。
- 使用匹配的现金流、成本、成交时点、XIRR、期末净值、最大回撤、波动率、Sortino 与现金使用率比较策略和固定 DCA。

### PR 7 — 策略 API 与 Web Studio / Strategy API and Web Studio

- 提供策略 CRUD、验证、回测、版本、激活、Decision Preview 和审计查询 API。
- Web 仅管理服务端数据；浏览器 UI 状态使用既有前端约定。先呈现内置策略和只读审计，再逐步开放 DSL 编辑器。

### PR 8 — Qwen Strategy Copilot / Qwen Strategy Copilot

- Qwen 根据受限 schema 生成“候选策略草案”、解释与警告。
- 候选必须经确定性 validator、回测和用户审阅后才能保存；AI 输出永不直接激活或下单。

## 7. 验收门槛 / Acceptance Gates

- 每个新公开 Rust API 有 rustdoc，带不变量的类型仅通过构造函数或 `TryFrom` 建立。
- 每个行为变化都有聚焦测试，至少运行 `cargo test -p core-domain`；策略 PR 额外运行相关 crate 测试、fmt、Clippy 与 `git diff --check`。
- 历史评估不得使用未来数据；决策日与成交日至少相隔一个可用交易日。
- 任意策略推荐均受计划预算、周期上限、机会现金、可用现金和 paper-only 安全边界限制。
- 对外文档只报告实际、可复现结果；策略未证明稳定优势时不得宣称提高收益。

## 8. 当前结论 / Current Decision

在上述基础设施建立前，不继续以 C5/C6/C7 方式搜索 70/20/10 权重，也不把 C1–C4 升级为默认生产策略。下一项可执行工作应是 **PR 1：策略契约与 `CoreOpportunityV1` 兼容包装**。

Until this foundation exists, no further C5/C6/C7 weight search will be promoted to production. The next executable work item is **PR 1: the policy contract and a backward-compatible `CoreOpportunityV1` wrapper**.
