# IndexLink V2.1 产品化与前端重构计划 / Productization and Frontend Plan

> 状态 / Status：**计划中 / Planned**。本文件定义下一阶段的产品定位、前端信息架构和上线门槛；不改变当前 V2 的策略或交易行为。

## 1. 定位修正 / Positioning

IndexLink V2.1 不与 QMT、PTrade 或券商终端竞争“任意代码策略、全市场行情、实盘自动交易和低延迟执行”。这些是券商与专业量化基础设施的能力边界。

V2.1 的定位是：

> **面向长期指数投资者的、本地优先的策略治理、决策审计与 paper-trading 工作台。**

产品负责让计划、预算、策略版本、证据、审批、订单意图和复盘相互可追溯；Broker、OpenD、QMT/PTrade 等系统只可作为未来的可替换账户或执行适配器。

IndexLink V2.1 is not a replacement for a general-purpose quantitative trading terminal. It is a **local-first policy governance, decision-audit, and paper-trading workspace for long-term index investors**.

## 2. 目标用户与待验证假设 / Target User and Hypothesis

首个目标用户不是高频或 Python 量化交易者，而是：

- 有稳定现金流、维护 1–5 个 ETF/指数定投计划的长期投资者；
- 不希望把账户密码或全部投资数据托管给第三方；
- 希望理解每次建议、预算变化和是否执行，而不是接受黑箱买卖指令；
- 可以接受固定 DCA 作为默认基准，并只在明确审阅后使用自定义机会桶规则；
- 需要按月/周查看计划、决策记录和模拟执行结果。

需要通过 10–15 次用户访谈验证的核心问题：

1. 用户是否确实会因计划分散、预算不透明或“为什么本次建议不同”而感到管理负担？
2. 用户是否愿意每月花约五分钟审阅决策存证，而不是只依赖券商的自动定投？
3. 用户真正需要的是账户只读聚合、可读决策解释、策略比较，还是自动交易？
4. 若用户主要需要任意策略代码、Tick 数据或无人值守实盘，IndexLink 不应把该人群当作目标市场。

The product hypothesis is deliberately narrow: users value transparent plan governance and review more than unrestricted strategy coding or automatic execution.

## 3. 产品原则与非目标 / Principles and Non-goals

| 原则 / Principle | V2.1 落地 / Product consequence |
| --- | --- |
| 本地优先 / Local first | SQLite、策略版本和审计记录默认保留在用户设备；云端同步只能是以后明确授权的可选能力。 |
| Fixed DCA 为基准 | 新计划默认 `fixed_dca@1`；任何策略比较必须显示匹配的 Fixed DCA 对照。 |
| 策略先验证再启用 | DSL 草案需要校验、固定样本准入、人工保存与计划激活。 |
| AI 无交易授权 | AI 只能解释、提示风险、生成只读受限 DSL 草案；不能保存、激活、下单或绕过预算。 |
| 人工掌握执行权 | scheduler 只生成存证；审批模式必须确认既有决策记录才可提交 paper order。 |
| 证据优先 / Evidence first | 决策保留策略版本、`as_of`、数据来源、AI 降级原因、推荐与订单回执。 |

明确不做 / Explicit non-goals：

- 任意 Python/JavaScript 策略代码执行；
- 高频、Tick、盘口、算法拆单或自动报撤；
- 默认实盘自动交易；
- 把回测或 AI 输出包装为收益承诺；
- 复制 QMT/PTrade 的行情终端、策略 IDE 或券商基础设施；
- 将券商密码、API Key 或交易凭据交给浏览器。

## 4. V2.1 信息架构 / Information Architecture

```text
今日 / Today
├─ 我的计划 / Plans
├─ 决策与审批 / Decisions
├─ 组合与表现 / Portfolio
├─ 策略实验室 / Strategy Lab (advanced)
└─ 设置与连接 / Settings & Connections
```

| 页面 / Page | 面向用户的内容 / User-facing purpose | 复用的现有能力 / Existing capability |
| --- | --- | --- |
| 今日 | 下一次执行、最新建议、是否需审批、简短风险与运行状态 | Decision Preview、scheduler、AI Evidence、runtime status |
| 我的计划 | 标的、预算、周期、策略、风险模式、启停 | Investment Plans、策略绑定、周期与双桶配置 |
| 决策与审批 | 可读解释、证据、策略版本、差异、订单状态与 paper 确认 | Decision Records、approval paper order |
| 组合与表现 | 本地 paper 持仓、订单、成交、轨迹与空状态 | OpenD/Mock、ledger、performance API |
| 策略实验室 | DSL、Copilot 草案、证据、准入和 Fixed DCA 对照 | Strategy Studio、admission、AI provider registry |
| 设置与连接 | API、SQLite、Qwen、OpenD 状态与本地连接说明 | health、ready、runtime-status |

`CoreOpportunityV1`、70/20/10 分层指标和旧历史回放属于**研究/兼容能力**，不应占据普通用户的首页；它们应只在策略详情或 Strategy Lab 中出现。

## 5. 现有页面迁移 / Current-page Migration

| 当前能力 | V2.1 去向 | 处理方式 |
| --- | --- | --- |
| Dashboard 的自动决策与运行状态 | 今日 | 保留，并改成单一“下一步行动”主任务。 |
| Dashboard 的市场输入、70/20/10 卡片 | Strategy Lab / 旧策略详情 | 从首页移除；标记为历史研究策略证据。 |
| Dashboard 的 OpenD、账本、轨迹、价格图 | 组合与表现 | 统一显示模拟账户、已确认成交与“暂无数据”。 |
| Dashboard 的一年 replay | Strategy Lab | 与策略准入明确区分，不能作为日常收益承诺。 |
| Plans 参数表单 | 我的计划向导 + 高级设置 | 默认只展示标的、预算、周期和策略；双桶/现金/风险折叠。 |
| Decisions 详情 | 决策与审批 | 保持审计核心，隐藏原始 JSON，仅在高级视图显示技术证据。 |
| Strategy Studio / Copilot | 策略实验室 | 保持受限 DSL 和人工准入，不开放自由代码。 |

## 6. 交付拆分 / Delivery Sequence

### Push 1 — 产品壳与首页收束

- 新建 `Today / Plans / Decisions / Portfolio / Strategy Lab / Settings` 路由与导航。
- 首页只显示计划状态、下一执行日、最新建议、审批 CTA 与安全运行状态。
- 保留旧 URL 的兼容跳转；完整中英文案。
- 不改 Rust 策略、数据库或下单逻辑。

### Push 2 — 计划向导与计划详情

- 将计划创建改为“标的与预算 → 周期与执行方式 → 策略”的三步流程。
- 默认展示 Fixed DCA；`CoreOpportunityV1` 标记为历史研究策略。
- 将双桶、滚存、周期上限和风险模式放到高级设置。

### Push 3 — 决策中心与审批体验

- 决策记录以“本次为何建议这样做”为主叙事。
- 显示计划预算、核心/机会金额、策略版本、`as_of`、AI 解释、风险提示和订单回执。
- 审批计划只能通过“确认提交模拟订单”推进同一份存证。

### Push 4 — 组合与表现

- 统一 OpenD/Mock、订单、持仓、成交和本地账本的展示。
- 区分模拟账户实际已确认成交、本地 ledger 与历史研究；没有成交时显示明确空状态。
- 历史回放从该页面移出。

### Push 5 — Strategy Lab 收束

- 将 DSL、Copilot、技术证据、准入和 Fixed DCA 对照集中到高级页面。
- 将所有回测数字标为研究结果和假设，不作为收益预测。

### Push 6 — 设置与连接

- 清楚显示 API、SQLite、Qwen、scheduler 与 OpenD 的本机状态。
- 设计“检测本机 OpenD → 选择模拟账户 → 本地绑定”的体验；浏览器不接收券商密码。
- 后端仅在此阶段按真实 UI 缺口补充聚合读取或本地账户绑定 API。

## 7. 产品上线标准 / Product Launch Gates

### 必须满足 / Required

1. 用户可在无 Qwen、无 OpenD、无云服务条件下创建 Fixed DCA 计划、生成本地决策记录并查看历史。
2. 任何可执行建议都可显示计划、策略 ID/版本、预算、`as_of`、证据来源、动作、金额和安全降级原因。
3. scheduler 永不自动下单；`approval` 模式只能确认已持久化的决策记录，并仅提交 paper order。
4. Fixed DCA 与任意已激活 DSL 策略均使用相同的计划预算和执行约束；策略准入结果不构成收益承诺。
5. OpenD 未配置/未登录时，核心计划、审计和研究功能仍可用，并给出清楚状态，而非阻塞或伪造账户数据。
6. 浏览器和 API 响应不得返回 API Key、券商密码、endpoint、账户凭据或底层 provider 错误细节。
7. 中英文切换覆盖全部用户可见文案；路由有可恢复错误页；服务端状态使用 React Query 管理。
8. 前端 lint、build、测试覆盖率门槛和相关 Rust 聚焦测试均通过；新增行为具有可读的审计/回归测试。

### Beta 验证门槛 / Beta Validation Gates

1. 完成至少 10–15 次目标用户访谈，并记录上述四项问题的结论。
2. 至少 5 位非开发者能在不看源码的情况下完成：创建计划 → 查看建议 → 理解原因 → 找到决策记录。
3. 至少 3 位用户能正确区分“Fixed DCA、研究策略、模拟订单和已成交记录”。
4. 若多数用户主要索取高频、任意代码或无人值守实盘，停止向该方向扩张，改为将 IndexLink 定位为可选的审计/研究上层工具。

## 8. 后端收束原则 / Backend Follow-up

前端重构期间不推倒现有 Hexagonal Architecture + Modular Monolith，也不重写策略 runtime。后端只在 UI 出现真实重复请求或语义缺口时补充：

- 面向“今日”的聚合只读 API，减少前端拼接多个请求；
- 本机账户绑定持久化，替代单一 `.env` 固定账户 ID；
- 将过大的应用编排逐步拆为 `PolicyExecutionService`、`DecisionAuditService`、`PortfolioReadService`、`AiEvidenceService` 与 `SchedulerService`；
- 将现有 PostgreSQL adapter 降为 feature-gated 的未来云端能力，SQLite 继续是 V2 默认单用户存储。

这些是演进项，不是本轮前端上线的前置条件。

## 9. 成功定义 / Definition of Success

V2.1 的成功不是声称跑赢市场，而是让目标用户在一次定投周期内可以回答：

1. 我有什么计划、下一步是什么？
2. 这次建议投入多少，为什么？
3. 这条建议使用了哪个策略版本和哪些截至当时的数据？
4. 我是否已经确认模拟订单，它是否成交？
5. 如果我不满意，如何退回 Fixed DCA 或停用策略？

If the product makes these answers clear without requiring users to write code, trust opaque AI, or surrender broker credentials, V2.1 meets its productization goal.
