# Change Log

## Unreleased

### 2026-06-20 11:36 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试注释补充（不改变行为）。
- 涉及文件：
  - `crates/quant-engine/tests/fundamental.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/common/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在 `quant-engine` 各测试入口顶部补充一句模块说明，明确对应第一层基本面、共享分位工具、第二层趋势存根的测试范围。
  - 在 `tests/common/mod.rs` 顶部补充共享夹具说明，区分其与独立集成测试入口的职责。
- 验证：
  - `cargo test -p quant-engine` 通过：32 个集成测试 + 1 个 doc test 全部通过。
  - 相关测试文件无 IDE linter 错误。

### 2026-06-20 11:34 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试结构整理（不改变行为）。
- 涉及文件：
  - `crates/quant-engine/tests/fundamental.rs`（由 `evaluate_fundamental.rs` 重命名）
  - `crates/quant-engine/tests/percentile.rs`（由 `percentile_of.rs` 重命名）
  - `crates/quant-engine/tests/trend.rs`（由 `evaluate_trend.rs` 重命名）
  - `crates/quant-engine/tests/DEFERRED_TESTS.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `quant-engine` 集成测试入口从函数名导向改为模块/层导向，使测试结构与 `src/percentile.rs`、`src/fundamental/mod.rs`、`src/trend/mod.rs` 一一对应。
  - 保留 `tests/common/mod.rs` 作为共享测试夹具与阈值模块，不引入子目录测试 harness，避免 Cargo 集成测试入口复杂化。
  - 更新 `DEFERRED_TESTS.md` 中利率重估测试的落地文件引用为 `fundamental.rs`。
- 验证：
  - `cargo test -p quant-engine` 通过：32 个集成测试 + 1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - 相关测试文件无 IDE linter 错误。

### 2026-06-20 11:25 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：结构重构（不改变行为）。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/percentile.rs`（新增）
  - `crates/quant-engine/src/fundamental/mod.rs`（新增）
  - `crates/quant-engine/src/trend/mod.rs`（新增）
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `quant-engine` 从单文件实现拆分为 `percentile`、`fundamental`、`trend` 三个模块，明确共享分位工具、第一层（70% 基本面）与第二层（20% 趋势）的边界。
  - `fundamental` 模块承载 `FundamentalConfig`、`MarketSnapshot`、`FundamentalSignal` 与 `evaluate_fundamental`；`trend` 模块承载 `TrendSignal` 与当前中性存根 `evaluate_trend_stub`；`percentile` 模块承载 `percentile_of`。
  - `lib.rs` 保留 crate 文档、模块声明、跨层 `QuantError`，并通过 `pub use` 维持原有根级 API（如 `quant_engine::evaluate_fundamental`、`quant_engine::percentile_of`）兼容。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：32 个集成测试 + 1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - `quant-engine/src` 相关文件无 IDE linter 错误。

### 2026-06-20 11:15 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试重构（不涉及生产实现）。
- 涉及文件：
  - `crates/quant-engine/tests/common/mod.rs`（新增）
  - `crates/quant-engine/tests/evaluate_fundamental.rs`
  - `crates/quant-engine/tests/percentile_of.rs`
  - `crates/quant-engine/tests/evaluate_trend.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `tests/common/mod.rs`，统一测试侧的领域阈值与常用夹具：中性分位、贵/便宜阈值、默认/测试历史长度、CAPE 权重边界、标准历史序列与测试配置 helper。
  - 将 `evaluate_fundamental`、`percentile_of`、`evaluate_trend` 中反复出现的 `0.50`、`0.80`、`0.20`、`10`、`60`、`0.0`、`1.0` 等语义数字改为命名常量或 helper，提高测试意图一致性。
  - 保留用于构造特定分位的局部夹具数字（如当前值、历史序列缩放），避免过度抽象导致测试可读性下降。
- 验证：
  - `cargo fmt` 通过。
  - `cargo test -p quant-engine` 通过：32 个集成测试 + 1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - 相关测试文件无 IDE linter 错误。

### 2026-06-20 10:37 UTC+10

- 执行模型：Claude Opus 4.8。
- 变更类型：测试补全 + 待办测试登记（不涉及实现）。
- 涉及文件：
  - `crates/quant-engine/tests/evaluate_fundamental.rs`
  - `crates/quant-engine/tests/DEFERRED_TESTS.md`（新增）
  - `CHANGE_LOG.md`
- 变更内容：
  - 评估产品专家提出的 5 条金融场景测试，结论：仅「利率重估」当前可写（落在已实现的基本面层内），其余依赖未实现的趋势层 / Decision Engine / `serde` 快照。
  - `evaluate_fundamental`：新增 `rate_repricing_low_erp_pushes_score_expensive_despite_neutral_cape`，覆盖 CAPE 中性但 ERP 极低（利率重估压缩风险补偿）时综合得分仍偏贵的「背离」场景，验证 ERP 倒置语义在两维背离下正确生效。
  - 新增 `tests/DEFERRED_TESTS.md`，登记暂不能写的场景（高估但趋势强、低估但急跌、审计回放），逐条标注依赖模块、前置条件、建议落地位置与断言要点；并说明「数据缺失」一条已被现有测试覆盖。
- 验证：
  - `cargo test -p quant-engine` 通过：32 个集成测试 + 1 个 doc test 全部通过（原 30 个集成）。
  - `crates/quant-engine/tests/evaluate_fundamental.rs` 无 IDE linter 错误。

### 2026-06-20 10:23 UTC+10

- 执行模型：Claude Opus 4.8。
- 变更类型：测试补全（不涉及实现，表征当前未定义行为的边界）。
- 涉及文件：
  - `crates/quant-engine/tests/percentile_of.rs`
  - `crates/quant-engine/tests/evaluate_fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 针对报告指出的三处未覆盖边界补充表征测试，并在注释中标明「若后续实现层加入校验需改为断言错误」。
  - `±Inf` 当前读数：`percentile_of` 仅拦 NaN，`+Inf` → 分位 `1.0`、`-Inf` → 分位 `0.0`，均不报错；并补 `evaluate_fundamental` 集成层用例（`+Inf` CAPE 与 `-Inf` ERP 合成历史最贵得分 `1.0`）。
  - `cape_weight` 越界：`2.0` 与 `-1.0` 在极值输入下使 `composite` 跌出 `[0,1]`，触发 `Percentile::new(...).expect(...)` panic，以 `#[should_panic]` 锁定当前行为。
  - 历史序列不等长：等长非必需，各指标独立定位（100/60 点均成功）；较短序列低于 `min_history_len` 时明确指向该指标传播 `InsufficientHistory`。
- 验证：
  - `cargo test -p quant-engine` 通过：30 个集成测试 + 1 个 doc test 全部通过（原 24 个集成）。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - 相关测试文件无 IDE linter 错误。

### 2026-06-20 10:14 UTC+10

- 执行模型：Claude Opus 4.8。
- 变更类型：测试补全（不涉及实现）。
- 涉及文件：
  - `crates/quant-engine/tests/percentile_of.rs`
  - `crates/quant-engine/tests/evaluate_fundamental.rs`
  - `crates/quant-engine/tests/evaluate_trend.rs`（新增）
  - `CHANGE_LOG.md`
- 变更内容：
  - 以 `readme.md` 设计约束为依据审计 `quant-engine` 测试覆盖，补齐缺失条目。
  - `percentile_of`：新增并列值（`<=` 语义）、全 NaN 历史降级为 `InsufficientHistory`、有效点数恰等于 `min_len` 边界、`InsufficientHistory` 字段（`indicator`/`required`/`found`）及 `QuantError` 的 `Display` 文案测试。
  - `evaluate_fundamental`：新增默认配置契约（0.5 / 60）、ERP 审计字段未倒置、审计字段如实记录原始分位、`cape_weight` 极值（1.0 纯 CAPE、0.0 纯倒置 ERP）、历史不足与 NaN 当前值的错误传播（对应熔断/降级链）测试。
  - 新增 `evaluate_trend.rs`，覆盖 20% 趋势层存根应返回中性 `0.5`。
- 验证：
  - `cargo test -p quant-engine` 通过：24 个集成测试 + 1 个 doc test 全部通过（原 11 个）。

### 2026-06-20 9:06 UTC+10

- 执行模型：Composer。
- 变更类型：错误信息语言统一。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `QuantError` 的 `Display` 输出统一为英文，与 `PercentileError` 保持一致，避免审计日志中英混排。
  - 将 `percentile_of` 中 `InvalidInput` 的 NaN 错误消息改为英文。
- 验证：
  - `cargo test -p quant-engine` 通过：11 个测试全部通过（含 1 个 doc test）。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - `crates/quant-engine/src/lib.rs` 无 IDE linter 错误。

### 2026-06-19（文档）

- 执行模型：Composer。
- 变更类型：文档。
- 涉及文件：
  - `readme.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `readme.md` 顶部徽章区从 AuroraView 模板链接替换为 IndexLink（`jamesra26/indexlink`）项目链接。
  - 移除不适用的 PyPI、Python、Codecov、CI workflow、pre-commit、ruff、mypy 等徽章。
  - 补充 Rust workspace、crate 结构、CHANGELOG、AGENTS 及 GitHub 社区类徽章；页脚链接改为 Issue Tracker、LICENSE、CHANGELOG。
- 验证：
  - 文档改动，未运行测试。

### 2026-06-19 23:12 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：领域 API 风格一致性。
- 涉及文件：
  - `crates/core-domain/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 `Multiplier` 增加 `Display` 实现，按百分比格式输出倍率，保持与 `Percentile` 的格式化能力对称。
  - 为 `Multiplier` 的 `Display` 行为增加单元测试。
  - 为 `Action` 增加 `Hash` 派生，便于后续作为 `HashMap` 键或用于去重统计。
  - 保持 workspace edition 为 `2021`，未进行 edition 升级。
- 验证：
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - `cargo llvm-cov -p core-domain --summary-only` 通过：Region / Function / Line 覆盖率均为 100.00%。
  - `crates/core-domain/src/lib.rs` 无 IDE linter 错误。

### 2026-06-19（文档）

- 执行模型：Composer。
- 变更类型：文档。
- 涉及文件：
  - `AGENTS.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在 `AGENTS.md` 顶部补充基于 `readme.md` 提炼的项目一句话描述。
- 验证：
  - 文档改动，未运行测试。

### 2026-06-19 23:02 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：Agent 协作规范。
- 涉及文件：
  - `AGENT.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在 `AGENT.md` 写明其他 agent 应遵循的项目规范，包括中文回复、变更日志记录、Rust crate 分层边界、`core-domain` lint 约束、newtype 不变量、测试覆盖率和审计/serde 原则。
- 验证：
  - 文档改动，未运行测试。

### 2026-06-19 23:00 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试覆盖率补强。
- 涉及文件：
  - `crates/core-domain/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 `PercentileError` 的 `Display` 输出增加单元测试，覆盖 `Nan` 与 `OutOfRange` 两种错误格式化。
  - 为 `Percentile` 的 `Display` 输出增加单元测试，覆盖百分比格式化行为。
- 验证：
  - `cargo test -p core-domain` 通过：12 个单元测试全部通过。
  - `cargo llvm-cov -p core-domain --summary-only` 通过：Region / Function / Line 覆盖率均为 100.00%。
  - `crates/core-domain/src/lib.rs` 无 IDE linter 错误。

### 2026-06-19 22:52 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：代码质量 / 文档约束。
- 涉及文件：
  - `crates/core-domain/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在 `core-domain` 增加 crate 级 `#![forbid(unsafe_code)]`，明确基础领域 crate 禁止 unsafe 代码。
  - 在 `core-domain` 增加 crate 级 `#![warn(missing_docs)]`，要求后续公开领域 API 补齐文档。
  - 补齐 `Multiplier::MIN`、`Multiplier::MAX`、`Multiplier::value` 的公开文档。
  - 补齐 `PercentileError::OutOfRange.value` 字段文档，消除新增 `missing_docs` 警告。
- 验证：
  - `cargo check -p core-domain` 通过。
  - `crates/core-domain/src/lib.rs` 无 IDE linter 错误。

### 2026-06-19 22:49 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：路线图 / 审计能力说明。
- 涉及文件：
  - `readme.md`
- 变更内容：
  - 在第 4 阶段路线图中明确后续为纯数据结构补充 feature-gated `serde` 支持。
  - 说明 `serde` 仅提供数据编码/解码能力，不引入 IO。
  - 说明 `Percentile`、`Multiplier` 等带不变量的 newtype 反序列化必须复用构造校验，避免绕过安全边界。
- 验证：
  - 文档改动，未运行测试。
