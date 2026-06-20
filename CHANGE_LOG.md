# Change Log

## Unreleased

### 2026-06-20 21:36 UTC+10

- 执行模型：Codex。
- 变更类型：后端测试覆盖率提升。
- 涉及文件：
  - `crates/api/Cargo.toml`
  - `crates/api/src/error.rs`
  - `crates/api/src/state.rs`
  - `crates/api/tests/health.rs`
  - `crates/storage/src/lib.rs`
  - `apps/server/src/config.rs`
  - `Cargo.lock`
  - `CHANGE_LOG.md`
- 变更内容：
  - API 集成测试保留 `/health`、`/ready` 与 CORS 跨模块 HTTP 契约；错误序列化、readiness backend 和 Debug 脱敏测试贴近对应源文件放置。
  - 补充 health 不访问数据库、自定义版本、CORS 预检与拒绝、安全 503 JSON、request ID 序列化以及 Storage backend 错误脱敏测试。
  - Storage 补充非法 URL、lazy pool、关闭连接池 ping 映射和结构化错误安全文案测试，并新增 `Storage::from_pool` 作为连接池依赖注入入口。
  - 将 server 环境读取重构为委托给纯 `Config::from_lookup` 解析入口，覆盖默认值、自定义值、非法输入、CORS 列表和敏感信息保护；未改变环境变量名及 `APP_PORT=0` 行为。
  - 未修改 CI workflow；`.github/workflows/rust-ci.yml` 与本次分支基线一致。
- llvm-cov 修改前：
  - `indexlink-api`：region 75.76%，function 82.35%，line 83.00%。
  - `indexlink-storage`：region 43.90%，function 50.00%，line 54.55%。
  - `indexlink-server`：region/function/line 均为 0.00%。
- llvm-cov 修改后：
  - `indexlink-api`：region 98.15%，function 96.15%，line 98.36%。
  - `indexlink-storage`：region 84.71%，function 95.00%，line 91.03%。
  - `indexlink-server`：整体 region 75.83%，function 75.00%，line 79.78%；其中 `config.rs` region 96.17%，function 93.75%，line 98.19%。
- 验证：
  - API 6 项单元测试与 8 项 HTTP 集成测试通过；Storage 6 项单元测试通过；server config 15 项单元测试通过。
  - 三个后端包的 llvm-cov 干净复测通过。
  - HTML 报告生成于本地 `target/llvm-cov/html`，未纳入 Git。
  - `cargo check --workspace --locked` 通过；`cargo test --workspace --locked` 共 86 项测试通过。
  - 三个后端包的 rustfmt check 与严格 Clippy（`-D warnings`）通过。
  - 全 workspace rustfmt check 仍被 `crates/core-domain/src/lib.rs` 三处既有格式阻塞；全 workspace Clippy 仍被该文件两个 `double_must_use` lint 阻塞，按责任边界未修改。

### 2026-06-20 21:30 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：CI 配置重构。
- 涉及文件：
  - `.github/workflows/rust-ci.yml`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 Rust CI 从单个串行 `test` job 拆分为独立的 `fmt`、`clippy`、`test`、`coverage` jobs，便于 GitHub Checks 单独定位失败阶段。
  - `fmt` job 仅安装 `rustfmt` 并执行 `cargo fmt --all -- --check`；`clippy` job 仅安装 `clippy` 并执行严格 clippy；`test` job 执行 workspace 测试。
  - 新增 `coverage` job，安装 `llvm-tools-preview` 与 `cargo-llvm-cov`，执行 `cargo llvm-cov --workspace --all-features --summary-only`。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - `cargo llvm-cov --workspace --all-features --summary-only` 通过：workspace 行覆盖率 67.77%，`core-domain` 与 `quant-engine` 行覆盖率均为 100.00%。

### 2026-06-20 21:07 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：代码质量修复（Clippy warnings）。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 移除 `Weight::complement` 上与 `Weight` 类型级 `#[must_use]` 重复的函数级 `#[must_use]`，修复 `clippy::double_must_use`。
  - 将 `FundamentalConfig::new` 中不必要的 `ok_or_else` 改为 `ok_or`，修复 `clippy::unnecessary_lazy_evaluations`。
  - 将测试中的 `std::iter::repeat(...).take(...)` 改为 `std::iter::repeat_n(...)`，修复 `clippy::manual_repeat_n`。
- 验证：
  - `cargo fmt --all` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。
  - `cargo test --workspace` 通过。

### 2026-06-20 19:38 UTC+10

- 执行模型：Codex。
- 变更类型：后端基础设施建设。
- 涉及文件：
  - `Cargo.toml`
  - `.gitignore`
  - `.env.example`
  - `rust-toolchain.toml`
  - `crates/storage/**`
  - `crates/api/**`
  - `apps/server/**`
  - `deployment/**`
  - `.github/workflows/rust-ci.yml`
  - `readme.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 PostgreSQL storage、Axum API 与 server composition root 注册进 Rust 2021 workspace。
  - 新增带连接超时和结构化错误的 PostgreSQL 连接池基础设施，不包含业务表或 repository。
  - 新增 `/health` 与 `/ready`、统一安全错误响应、可替换 readiness 检查、Trace、CORS 配置入口与请求体上限。
  - 新增环境配置、结构化日志、Ctrl+C/SIGTERM 优雅停机、多阶段 Dockerfile、本地 PostgreSQL Compose 与 Rust CI。
  - 补充后端本地启动、环境变量、Docker Compose 和基础端点文档；未修改 `core-domain` 或 `quant-engine`。
- 验证：
  - 安装并使用 Rust/Cargo 1.96.0、rustfmt 与 clippy；`cargo check --workspace --locked` 通过。
  - 新增后端 crate 的 `cargo fmt --check` 与严格 Clippy（`-D warnings`）通过。
  - `cargo test --workspace --locked` 通过：56 项单元、集成与文档测试全部成功。
  - `docker compose -f deployment/docker-compose.yml config` 通过；本机 Docker daemon 未安装/运行，镜像构建与 HTTP 实测未执行成功。
  - workspace 全量 rustfmt/Clippy 被 `crates/core-domain/src/lib.rs` 的既有格式和两个 `double_must_use` lint 阻塞；按任务边界未修改该 crate。
  - `git diff --check` 通过，且 `core-domain`、`quant-engine` 最终均无修改。

### 2026-06-20 14:05 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试补全（覆盖率提升）。
- 涉及文件：
  - `crates/quant-engine/tests/fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 `Weight` 公开转换契约补充集成测试：`TryFrom<f64> for Weight` 接受合法原始权重，以及 `From<Weight> for f64` 可回取底层数值。
  - 覆盖此前 `cargo llvm-cov -p quant-engine --test fundamental` 报告中 `fundamental/mod.rs` 未覆盖的转换 trait 行。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：20 个 fundamental 测试、16 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo llvm-cov -p quant-engine --test fundamental --summary-only --show-missing-lines` 通过：`crates/quant-engine/src/fundamental/mod.rs` 行覆盖率、函数覆盖率与 region 覆盖率均为 100%。

### 2026-06-20 13:56 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：实现可读性整理（不改变行为）。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `FundamentalConfig::default()` 中的默认 CAPE 权重 `0.5` 和默认历史长度 `60` 提升为模块内常量，减少实现侧魔法数字。
  - 保持默认配置行为不变：CAPE/ERP 各半，最少 5 年月度历史数据。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过。
  - `cargo test -p core-domain` 通过。

### 2026-06-20 13:50 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：公开 API 重构（错误类型结构化）。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/src/percentile.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `QuantError::InvalidInput(String)` 拆分为 `InvalidWeight { value }`、`InvalidMinHistoryLen { value }`、`InvalidCurrentValue { indicator, value }`，便于调用方按错误语义精确匹配。
  - 更新 `Weight::new`、`FundamentalConfig::new` 与 `percentile_of`，分别返回对应结构化错误分支。
  - 更新测试断言与 `Display` 测试，避免依赖通用字符串错误来区分输入异常。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：18 个 fundamental 测试、16 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-20 13:47 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：公开 API 易用性增强。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为公开类型 `Weight` 实现 `Display`，以 `50.0%` 这类百分比格式输出，便于审计日志阅读。
  - 补充 `weight_display_uses_percent_format_for_audit_logs` 测试，锁定默认 CAPE 权重的展示格式。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过。
  - `cargo test -p core-domain` 通过。

### 2026-06-20 13:45 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：公开 API 易用性增强。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 为 `FundamentalConfig`、`FundamentalSnapshot`、`FundamentalSignal` 派生 `PartialEq`，方便测试断言和上层审计回放进行逐字段精确比较。
  - 保持不派生 `Eq`，避免为包含浮点语义的类型引入不合适的全等承诺。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过。
  - `cargo test -p core-domain` 通过。

### 2026-06-20 12:12 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：审计修复（金融输入有限性校验）。
- 涉及文件：
  - `crates/quant-engine/src/percentile.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `percentile_of` 的当前读数校验从仅拒绝 `NaN` 收紧为拒绝所有非有限数，`±Inf` 现在返回 `QuantError::InvalidCurrentValue`。
  - 更新 `percentile` 边界测试，不再把 `+Inf` / `-Inf` 锁定为合法极端分位。
  - 更新 fundamental 层传播测试，确认 CAPE/ERP 当前读数为非有限数时向上传播 `InvalidCurrentValue`。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：17 个 fundamental 测试、16 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-20 12:06 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：命名重构（公开 API 语义收敛）。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 fundamental 层输入快照从 `MarketSnapshot` 重命名为 `FundamentalSnapshot`，避免未来与趋势层快照或上层聚合市场快照混淆。
  - 同步根级导出与 fundamental 集成测试，不保留旧名兼容导出。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过。
  - `cargo test -p core-domain` 通过。

### 2026-06-20 12:04 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：审计修复（分位计算输入校验）。
- 涉及文件：
  - `crates/quant-engine/src/percentile.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 在公共函数 `percentile_of` 入口拒绝 `min_len = 0`，避免空历史在长度检查后继续执行并触发 `0 / 0`、NaN 分位和后续 panic。
  - 补充 `zero_min_len_returns_error_before_empty_history_division` 回归测试，锁定 `min_len = 0 + 空历史` 返回 `QuantError::InvalidMinHistoryLen`。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：17 个 fundamental 测试、16 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-20 11:47 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：审计修复（配置不变量保护）。
- 涉及文件：
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `crates/quant-engine/tests/common/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `Weight` newtype，用于表达配置权重并在构造期校验 `[0.0, 1.0]`，避免 `cape_weight` 越界破坏加权平均不变量。
  - 为 `FundamentalConfig` 新增 `new(cape_weight, min_history_len)` 构造函数，并将 `min_history_len` 收紧为 `NonZeroUsize`，防止 0 长度配置进入分位计算。
  - 更新基本面测试：将原先锁定非法权重运行期 panic 的用例改为断言构造期返回结构化错误，并补充 `min_history_len = 0` 的拒绝测试。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：17 个 fundamental 测试、15 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

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
  - `fundamental` 模块承载 `FundamentalConfig`、`FundamentalSnapshot`、`FundamentalSignal` 与 `evaluate_fundamental`；`trend` 模块承载 `TrendSignal` 与当前中性存根 `evaluate_trend_stub`；`percentile` 模块承载 `percentile_of`。
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
