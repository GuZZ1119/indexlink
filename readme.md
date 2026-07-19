<p align="center">
  <img src="assets/icons/indexlink-logo.png" alt="IndexLink" width="400">
</p>

<p align="center">
  <a href="./readme.en.md">English</a> | 中文文档
</p>

<p align="center">
  <a href="https://github.com/jamesra26/indexlink/blob/main/Cargo.toml"><img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Version"></a>
  <a href="https://github.com/jamesra26/indexlink/releases"><img src="https://img.shields.io/github/v/release/jamesra26/indexlink?display_name=tag" alt="Latest Release"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://github.com/jamesra26/indexlink"><img src="https://img.shields.io/badge/status-demo%20MVP-blue" alt="Status"></a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-edition%202021-orange.svg" alt="Rust"></a>
  <a href="https://doc.rust-lang.org/cargo/"><img src="https://img.shields.io/badge/Cargo-workspace-lightgrey.svg" alt="Cargo Workspace"></a>
  <a href="https://github.com/jamesra26/indexlink"><img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg" alt="Platform"></a>
  <a href="https://github.com/jamesra26/indexlink/tree/main/crates"><img src="https://img.shields.io/badge/crates-core--domain%20%7C%20quant--engine-blue" alt="Crates"></a>
</p>

<p align="center">
  <a href="https://conventionalcommits.org"><img src="https://img.shields.io/badge/Conventional%20Commits-1.0.0-yellow.svg" alt="Conventional Commits"></a>
  <a href="./CHANGE_LOG.md"><img src="https://img.shields.io/badge/changelog-CHANGE__LOG.md-green" alt="Changelog"></a>
  <a href="./AGENTS.md"><img src="https://img.shields.io/badge/contributing-AGENTS.md-blue" alt="Contributing"></a>
</p>

<p align="center">
  <a href="https://github.com/jamesra26/indexlink/stargazers"><img src="https://img.shields.io/github/stars/jamesra26/indexlink?style=social" alt="GitHub Stars"></a>
  <a href="https://github.com/jamesra26/indexlink/commits/main"><img src="https://img.shields.io/github/last-commit/jamesra26/indexlink" alt="Last Commit"></a>
  <a href="https://github.com/jamesra26/indexlink/graphs/commit-activity"><img src="https://img.shields.io/github/commit-activity/m/jamesra26/indexlink" alt="Commit Activity"></a>
</p>

<p align="center">
  <a href="https://github.com/jamesra26/indexlink/issues"><img src="https://img.shields.io/github/issues/jamesra26/indexlink" alt="Open Issues"></a>
  <a href="https://github.com/jamesra26/indexlink/pulls"><img src="https://img.shields.io/github/issues-pr/jamesra26/indexlink" alt="Open PRs"></a>
  <a href="https://github.com/jamesra26/indexlink/graphs/contributors"><img src="https://img.shields.io/github/contributors/jamesra26/indexlink" alt="Contributors"></a>
</p>

<p align="center">
  <a href="https://github.com/jamesra26/indexlink/issues">Issue Tracker</a> •
  <a href="./LICENSE">License</a> •
  <a href="./CHANGE_LOG.md">Changelog</a>
</p>

IndexLink 是一个为长期指数投资者设计的智能定投执行系统。它通过 **“历史分位锚点 + AI 语义感知”** 双引擎，在定投日进行微调：相对低位多投、相对高位减投、过热延时。

许多学生与上班族只能以有限预算长期投入，也很难在忙碌、波动或情绪化的市场环境中始终按纪律执行计划：上涨时容易追高，回撤时容易暂停，事后又缺少“这次为何这样调整”的可核查记录。IndexLink 将固定定投纪律、可复现的市场位置指标与受限的 AI 新闻解释结合起来，帮助使用者在**不声称预测市场**的前提下，按计划审计、调整并留存依据。

> **核心前提：** 我们无法判断市场是否「低估」，但可以用数据检测它在历史分布中所处的「位置」。IndexLink 只测量位置，不声称知道价值——这是「自适应定投」与「择时投机」的本质区别。

---

## 核心哲学

传统的定投（DCA）在极端行情下存在僵化问题。IndexLink 的存在是为了解决：

- **处于历史高位区间时仍机械全量买入：** 当 P/E 处于历史 90% 分位且情绪过热时，自动触发“延迟”或“减量”。
- **处于历史低位区间时金额未随位置自适应：** 当价格 / ERP 分位处于历史低位区间时，自动建议/执行适度加码。
- **利好出尽的陷阱：** 结合财报季预期差与宏观新闻，识别“虚假繁荣”。

---

## 决策模型：70/20/10 法则

系统拒绝“盲目 AI 幻想”，每一笔指令都基于以下加权逻辑：

| 维度                       | 权重    | 核心指标                             | AI 的角色                                                   |
| :------------------------- | :------ | :----------------------------------- | :---------------------------------------------------------- |
| **历史位置 (Fundamental)** | **70%** | P/E Ratio (Shiller), ERP, 历史分位点 | **硬约束**：计算当前价格在历史分布中所处的分位。            |
| **近期趋势 (Technical)**   | **20%** | 200日均线距离, RSI, 波动率 (VIX)     | **节奏控制**：判断是否处于“接飞刀”或“赶顶”状态。            |
| **语义感知 (Sentiment)**   | **10%** | 财报预期差、宏观新闻、用户自定义源   | **软微调**：通过 Qwen 识别新闻/机构评级调整背后的逻辑偏向。 |

---

## 关键功能

- 🤖 **Qwen 决策引擎**：负责阅读本周核心财经新闻及财报指引，识别预期差。
- 🦀 **Rust 后端与本地账本**：使用 Rust (Axum + Tokio) 和 SQLite 保存计划、自动决策、订单回执与演示账本；可在本机或 Alibaba Cloud ECS 的 Docker Compose 中运行。
- 📊 **动态动作空间**：
  - **Overweight (+20~50%)**：处于历史低位区间且未在极端急跌中时，定投纪律内适度加码。
  - **Standard (100%)**：处于历史中性区间（约 30%~70% 分位）时稳健执行。
  - **Tactical Delay**：因重大新闻（如非农、议息）或技术过热建议延迟 3-5 天。
  - **Underweight (-50%) / Skip**：处于历史高位区间或系统性风险时缩量或观望。
- 💰 **双桶金额拆分**：在到期计划的预览中可按核心/机会比例拆分单次计划金额；完整副桶余额调度仍在后续计划中。
- 🔌 **Paper Trading 接口**：支持本地 MockBroker 与 loopback Futu/Moomoo OpenD 模拟账户；只接受 paper 环境，绝不开放实盘下单。
- 📜 **可读 AI 决策存证**：每次自动或手动 Decision Preview 都写入本地记录，展示时间、标的、计划金额、70/20/10 输入来源、Qwen 理由/新闻/警告、动作、订单请求与回执。

### 当前 MVP 状态（2026-07）

当前分支已具备**本机可演示闭环**：创建/修改/删除定投标的，自动获取 70/20 市场输入，调用 Qwen 生成受限的 10% 情绪依据，产生并保存决策存证，在执行日按双桶拆分，并可由操作者明确确认后向 MockBroker 或本机 Futu/Moomoo OpenD **模拟账户**提交 paper order。Dashboard 同时提供决策历史、模拟账户只读刷新、本地收益账本、真实持仓价格走势及一年历史价格规则回放。

这不等同于生产交易系统。演示前仍需由操作者在本机配置 Qwen Key、已登录的 OpenD 模拟账户和网络；任一外部数据源不可用时，系统会安全降级或拒绝本次自动决策，绝不伪造数据或自动提交订单。

| 能力 | MVP 状态 | 重要边界 |
| :--- | :--- | :--- |
| 计划管理与本地 SQLite 持久化 | 已完成 | 单用户、本地文件；删除计划不会撤销 OpenD 的既有订单。 |
| 自动 70/20 市场输入 | 已完成 | 依赖本机 OpenD 日线、Shiller CAPE、美国国债与 Cboe VIX 的可用性。 |
| Qwen 情绪与证据 | 已完成 | 仅在配置 `DASHSCOPE_API_KEY` 时可用；不可用时显式降级为 90/10/0。 |
| 固定月日自动审计 | 已完成 | Scheduler 只写决策存证，**从不自动下单**。 |
| 双桶拆分与 paper order | 已完成 | 仅命中执行日且操作者显式请求时才提交；只允许模拟账户。 |
| 本地收益账本与图表 | 已完成 | 仅覆盖账本启用后、可归属到计划的已观察成交；不会反推历史账户交易。 |
| 一年历史比较图 | 已完成 | 是基于 MA200 的价格规则回放，**不是**完整 70/20/10 历史回测。 |

尚未实现：每计划 1–31 天的多次审计周期与月度预算、自动下单/撤单、完整的历史 70/20/10 回测、云端多用户同步、税费/汇率/分红处理及任何实盘交易能力。

---

## 技术架构

### 设计原则

1. **确定性优先，AI 受限**：70% + 20% 为纯函数式、可复现的计算；10% 的 AI 仅在有界区间内微调。AI 不可用时自动降级为 90/10/0，系统照常运行。
2. **位置语言贯穿数据模型**：核心输出是历史分位而非价值判断。
3. **金融可靠性三件套**：**幂等**（同一计划同一 UTC 执行日只自动创建一次决策）、**审计**（每笔决策可回放）、**安全降级**（Qwen 不可用时 90/10/0；70/20 数据不可用则不创建决策）。
4. **决策与执行分离**：决策计算与下单是两个阶段，中间可插入用户确认。

### 分层总览

```mermaid
graph TD
    WEB[Web Dashboard / API Client]

    subgraph Ingestion[数据接入层]
        MD[市场行情<br/>价格/PE/VIX]
        NEWS[新闻/财报源]
    end

    subgraph Core[Rust 核心 Axum + Tokio]
        SCH[Scheduler<br/>固定月日审计触发]
        QUANT[Quant Engine<br/>分位/均线/ERP]
        AICLI[AI Client<br/>Qwen 适配]
        DEC[Decision Engine<br/>70/20/10 加权]
        EXEC[Paper order gate<br/>人工确认]
    end

    subgraph Adapters[外部适配层]
        BROKER[Broker Adapter<br/>Mock / Real]
    end

    subgraph Storage[持久化]
        DB[(状态/审计/缓存)]
    end

    subgraph Cloud[Alibaba Cloud 运行时]
        ECS[Alibaba Cloud ECS<br/>Docker Compose + SQLite Volume]
        QWEN[Model Studio / DashScope<br/>Qwen API]
    end

    WEB --> ECS
    ECS -. 承载 .-> SCH
    ECS -. 承载 .-> QUANT
    ECS -. 承载 .-> AICLI
    ECS -. 承载 .-> DEC
    ECS -. 承载 .-> EXEC
    MD --> QUANT
    NEWS --> AICLI
    SCH --> DEC
    QUANT --> DEC
    AICLI --> DEC
    AICLI -- OpenAI-compatible HTTPS --> QWEN
    DEC --> EXEC
    EXEC --> BROKER
    DEC --> DB
    EXEC --> DB
    QUANT --> DB
```

### 模块职责

| 模块                       | 权重      | 职责                                                                                            |
| :------------------------- | :-------- | :---------------------------------------------------------------------------------------------- |
| **Scheduler**              | —         | 已实现最小固定月日 Scheduler：每 60 秒检查一次，到期日用本地 SQLite `(plan_id, UTC 日期)` 幂等键创建自动决策存证；不自动下单。 |
| **Quant Engine**           | 70% + 20% | 将所有指标转为「在自身历史分布中的**指数加权分位**」；以半衰期为唯一旋钮（默认 36 个月月度数据），消除硬窗口的「幽灵跌落」效应；纯函数，无 IO，实盘与回测共用。 |
| **AI Client**              | 10%       | 封装 Qwen，输出有界情绪偏移 `sentiment ∈ [-1, +1]`；超时/解析失败即返回 0（降级）。             |
| **Decision Engine**        | —         | 按 70/20/10 合成综合得分，映射为定投倍率，输出含输入快照的 `Decision`。                         |
| **Execution Gate**         | —         | 决策 → 可选人工确认 → paper order；自动 Scheduler 不携带订单数量，不能绕过确认下单。           |
| **Broker Adapter**         | —         | 一个 trait，两个 paper-only 实现：`MockBroker` 与本机 OpenD adapter；不支持实盘。               |

### 决策管线

```text
综合得分 S = 0.70 * f_value(加权分位)        // 历史位置，主导
          + 0.20 * f_trend(均线/RSI/VIX)  // 节奏
          + 0.10 * sentiment              // AI 有界微调

倍率 multiplier = clamp( map(S), 0.0, x )   // 上限x为用户决定，下限 Skip
```

- **历史位置**使用**指数加权 ECDF**：权重 $w_k = (1-\alpha)^k$，$\alpha = 1 - 0.5^{1/H}$，$H$ 为半衰期（默认 36 个月月度数据）。越近的样本权重越高，滞后 $H$ 处权重恰好衰减至 $0.5$；无分布假设，输出仍为 `[0, 1]` 分位，天然消除硬窗口的「幽灵跌落」效应。
- **低位但急跌**时，`f_trend` 给出负向修正，体现「不接飞刀」——加码更保守。
- `clamp` 是硬安全边界：无论 AI 如何输出，倍率永远落在 `[0, 1.5]`。
- 动作（Overweight / Standard / Delay / Underweight / Skip）只是倍率所在区间的标签。

### 双桶现金池（后续设计，当前未启用余额调度）

当前 MVP 只会在 Decision Preview 中把已确定的计划金额按 core/opportunity 比例拆分并保存审计快照；**不会**保存副桶余额、调拨现金或自动改变订单金额。以下规则是后续执行层的设计目标，不应视为当前演示行为。

**四条核心规则**

1. **副桶是弹药缓冲池**：资金最终目的地始终是主桶（主标的）；「从副桶取出」= 完成此前缓存的定投指令，不构成择时卖出。
2. **取出量受余额约束**：实际从副桶划出金额 = min（理论补足额, 副桶当前余额）；余额不足时按可配置策略处理（见下方）。
3. **副桶设累积上限**：上限为基准金额的若干倍（默认 3 倍）；超出上限的累积金额直接买入主桶，避免副桶意外堆成大仓位。
4. **现金流策略可配置**：

| 策略 | 副桶余额不足时的处理 | 适合场景 |
| :--- | :--- | :--- |
| `Conservative`（默认） | 按实际余额部分执行，审计日志标注「弹药不足，部分执行」 | 严格限制单期现金流、收入不稳定 |
| `Aggressive` | 从当期额外现金流补足缺口，保证倍率完整执行 | 愿意在信号强时多投、收入稳定 |

**资金流示意**（$M$ = 决策倍率，$B$ = 基准金额）

```text
M < 1：主桶买入 B×M，差额 B×(1-M) 存入副桶（若副桶未满）
M = 1：主桶全额买入 B，不操作副桶
M > 1：主桶买入 B×M；超出部分 B×(M-1) 来自副桶（受余额约束）
```

> 双桶逻辑完全属于执行层（`Execution Orchestrator`），与决策数学解耦——`Decision` 只输出倍率 $M$，双桶在倍率之后才介入，不污染无 IO 的 `decision` / `quant-engine` crate。

### 工程结构（当前 Cargo Workspace）

```text
indexlink/
├─ crates/
│  ├─ core-domain/      # 数据结构: Action, Percentile, Multiplier (无 IO)
│  ├─ quant-engine/     # 70%+20% 纯函数计算 (无 IO)
│  ├─ ai-client/        # Qwen 适配 + 降级逻辑
│  ├─ decision-engine/  # 70/20/10 合成 + 映射函数
│  ├─ investment-plans/ # 月度计划与到期预览
│  ├─ decision-records/ # 审计记录领域 port
│  ├─ market-data/      # 自动 70/20 输入 provider
│  ├─ broker/           # Broker trait + Mock/OpenD paper 实现
│  ├─ storage/          # DB 访问 (审计/状态/缓存)
│  └─ api/              # Axum HTTP 层 (确认/查询/手动干预)
└─ apps/
   ├─ server/           # 组装、最小 Scheduler 的可执行入口
   └─ web/              # Vite + React 演示界面
```

> `quant-engine` / `decision-engine` 保持无 IO 的纯逻辑 crate，使**本地演示与历史回放共享决策数学**；当前历史回放仍是简化的 MA200 价格规则，不是完整 70/20/10 回测。

### 持久化与审计

| 数据表         | 用途                                                    |
| :------------- | :------------------------------------------------------ |
| `investment_plans` | 定投计划（标的、固定月执行日、基准金额与单次上限） |
| `decision_records` | 每次决策的 **70/20/10 输入快照**、Qwen 证据、动作、订单请求/回执与摘要（《AI 决策存证》） |
| `scheduled_decision_runs` | 自动 Scheduler 的 `(plan_id, UTC 日期)` 幂等 claim，避免重启或下一 tick 重复存证 |
| `paper_orders` / `paper_fills` / `portfolio_snapshots` | 本地模拟账户账本、成交增量与收益快照；只读 OpenD 对账写入 |

> 审计原则：**存输入而非只存结论**——保存当时的分位、趋势、sentiment 与权重，事后才能回答「为何那天加码 30%」。

### 可靠性与安全

- **自动存证幂等**：`scheduled_decision_runs(plan_id, UTC 日期)` 唯一约束，避免自动 Scheduler 重复创建同日记录；订单不由 Scheduler 自动提交。
- **降级链**：AI 挂 → 90/10/0；70/20 行情源挂 → 本次不执行、不写存证，等待下一 tick 或人工重试。
- **金额安全**：硬编码倍率上限 + 单日金额上限，AI 无法突破。
- **人工确认**：操作者可在 Decision Preview 中显式附带 paper order；没有该请求就只生成存证。当前不提供自动下单、自动撤单或实盘交易入口。

### 分阶段落地

1. **MVP**：`core-domain` + `quant-engine`（仅 70%，使用指数加权 ECDF，半衰期 36 个月）+ `MockBroker` + 本地回测，验证加权分位驱动的自适应定投。
2. **加节奏**：接入 20% 趋势 + 熔断。
3. **加 AI**：接入 Qwen 的 10% 有界微调 + 降级。
4. **当前演示闭环**：本地 SQLite、固定月日自动审计、70/20 自动市场输入、Qwen 10% 有界情绪、可读决策存证、paper order 人工确认、OpenD 模拟账户读取与本地收益账本。
5. **下一阶段：周期审计与月度预算**：将单一 `schedule_day` 扩展为每 1–31 天一次的可配置审计周期，并新增每计划月度金额上限、跨月重置、停机补跑与延迟重审规则；该阶段完成前，系统仍是“每月一个固定执行日”。
6. **非 MVP 范围**：实盘交易、自动提交订单、云端同步、多用户认证、税费/分红/汇率处理和完整历史成交回补。

> `serde` 仅提供数据编码/解码能力，不引入 IO；对 `Percentile`、`Multiplier` 等带不变量的 newtype，反序列化必须复用构造校验，避免绕过安全边界。

---

## 免责声明

> **本项目仅供学习与技术研究之用，不构成任何投资建议。**

- **非投资建议**：IndexLink 输出的所有决策、倍率与信号仅为基于历史数据的量化计算结果，不代表任何买卖推荐，也不预测市场涨跌。
- **不保证收益**：指数投资存在本金亏损风险，历史分位与回测表现均**不预示**未来收益。任何依据本系统做出的投资决策，盈亏由使用者自行承担。
- **自适应 ≠ 择时**：本系统只测量价格在历史分布中的「位置」，**不声称**判断市场「低估 / 高估」，更无法保证「买在低点」。
- **风险自负**：在接入真实券商 API 进行实盘交易前，请充分理解代码逻辑与潜在风险，并自行进行充分测试。作者不对因使用本软件造成的任何直接或间接损失负责。
- **合规提醒**：自动化交易可能受所在国家/地区法律法规及券商条款约束，使用前请确认合规性。

---

## 后端基础设施（第一阶段）

当前后端默认使用本地 SQLite 文件，已提供 HTTP 服务、启动 migration、健康检查、就绪检查、结构化日志、优雅停机和持久化 Docker Compose 数据卷。旧 PostgreSQL adapter 仍保留为兼容实现，但不再是 MVP 运行时依赖。

### 本地启动

1. 安装当前 stable Rust 工具链以及 `rustfmt`、`clippy`。
2. 复制示例配置并启动服务。首次启动会自动创建本地 SQLite 文件并执行 migration：

   ```bash
   cp .env.example .env
   cargo run -p indexlink-server
   ```

3. 验证服务：

   ```bash
   curl http://localhost:8080/health
   curl http://localhost:8080/ready
   ```

4. 启动 Web 演示界面。开发服务器会把计划、信号、决策与决策历史请求代理到本机的 Rust 服务；若将前端部署到其他来源，请设置 `VITE_API_BASE_URL`，并将该来源加入 `CORS_ALLOWED_ORIGINS`：

   ```bash
   pnpm --dir apps/web install --frozen-lockfile
   pnpm --dir apps/web dev
   ```

5. 在 Dashboard 按闭环完成演示：
   - 在“定投标的”创建或选择计划；计划当前使用固定的每月执行日。
   - 点击“自动拉取市场信号”或“自动拉取并生成决策存证”。浏览器不再手填 70/20；后端读取 OpenD 日线、Shiller CAPE、美国国债和 Cboe VIX，并在可用时读取 Qwen 情绪。
   - 只有当前 UTC 日期命中计划执行日、动作允许执行且操作者勾选并填写 paper order 时，才会向 MockBroker 或已配置的 OpenD 模拟账户提交订单。其他情况仅生成审计存证。
   - 使用“刷新模拟账户”“刷新收益账本”“拉取走势”和“一年历史模拟”查看只读账户状态、可归属成交后的本地账本与图表。Qwen 或市场数据不可用时，页面会明确显示降级或未执行原因。

`.env` 仅供本地使用且已被 Git 忽略。主要环境变量如下：

| 变量 | 默认示例 | 说明 |
| :--- | :--- | :--- |
| `APP_HOST` | `0.0.0.0` | HTTP 监听地址 |
| `APP_PORT` | `8080` | HTTP 监听端口 |
| `RUST_LOG` | `info,indexlink_server=debug` | 日志过滤规则 |
| `DATABASE_URL` | `sqlite://indexlink.db?mode=rwc` | SQLite 文件地址；未设置时使用该本地默认值 |
| `CORS_ALLOWED_ORIGINS` | `http://localhost:5173` | 逗号分隔的允许来源 |
| `DATABASE_MAX_CONNECTIONS` | `10` | 连接池上限 |
| `DATABASE_CONNECT_TIMEOUT_SECONDS` | `5` | 启动连接超时秒数 |
| `SCHEDULER_ENABLED` | `true` | 是否运行固定月日自动决策 Scheduler；只写审计，绝不自动下单 |
| `SCHEDULER_TICK_SECONDS` | `60` | Scheduler 检查间隔；使用 UTC 日期判断是否到期 |
| `DASHSCOPE_API_KEY` | — | 可选；配置后启用真实 Qwen 情绪与证据。只保存在本机 `.env`，不得提交。 |
| `DASHSCOPE_MODEL` | `qwen-plus` | DashScope OpenAI-compatible 模型名 |
| `OPEND_PROVIDER` | — | 可选；`futu` 或 `moomoo`。配置后使用本机 OpenD 模拟账户替代 MockBroker。 |
| `OPEND_HOST` / `OPEND_PORT` | `127.0.0.1` / `11111` | 仅接受 loopback OpenD。 |
| `OPEND_ACCOUNT_ID` | — | 多模拟账户时指定目标账户；不得提交或写入日志。 |

### Docker Compose

Compose 会使用名为 `sqlite-data` 的本地 Docker volume 保存数据库；执行 `down` 不会删除该数据，若需要清空演示数据请显式删除该 volume。

```bash
docker compose -f deployment/docker-compose.yml up --build -d
docker compose -f deployment/docker-compose.yml ps
docker compose -f deployment/docker-compose.yml down
```

### 基础端点

- `GET /health`：只检查服务进程是否存活，不访问数据库。
- `GET /ready`：执行 SQLite 存活检查；数据库不可用时返回 HTTP `503` 和不含内部错误的统一 JSON 响应。

---

## 版权与贡献者

Copyright © 2026 IndexLink Contributors。项目以 [MIT License](./LICENSE) 发布；许可证正文中的原始版权声明保持不变。

- Jame — 项目原始作者与仓库维护者。
- Xuanzhou Gu — 后端、SQLite 持久化、OpenD paper trading、决策存证与演示闭环贡献者。
- Yucong Peng — 项目贡献者。
