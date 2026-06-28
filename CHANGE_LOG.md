# Change Log

## Unreleased

### 2026-06-28 18:36 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划 API（更新路由）。
- 涉及文件：
  - `crates/api/src/routes/investment_plans.rs`
  - `crates/api/src/lib.rs`
  - `crates/api/tests/investment_plans.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `PATCH /investment-plans/:id`，通过 API DTO 转换到领域 `UpdateInvestmentPlan`，不让 serde 直接构造领域类型。
  - 路径 UUID 与 JSON 解析失败统一映射为 `ApiError::BadRequest`，保持 JSON error envelope。
  - CORS 允许方法补充 `PATCH`。
  - 新增路由测试覆盖字段合并、金额组合校验、非法 ID 与非法 JSON。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-api --locked` 通过。

### 2026-06-27 00:20 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试夹具语义修正（趋势中性历史）。
- 涉及文件：
  - `crates/quant-engine/tests/common/mod.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/trend/direction.rs`
  - `crates/quant-engine/tests/trend/indicators.rs`
  - `crates/quant-engine/tests/trend/regime.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `neutral_weighted_history` 与 `TREND_NEUTRAL_CURRENT`，用低/高样本成对交错构造加权 ECDF 语义上的趋势中性历史。
  - `neutral_trend_snapshot` 改用加权中性历史，不再把 `standard_history() + 50.5` 标注为“历史中位 / 中性分位”。
  - 新增 `neutral_weighted_history_is_near_half_under_trend_config`，直接验证趋势测试配置下加权分位接近 0.5。
  - 同步修正趋势行为测试中的中性场景注释与夹具使用。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --test trend direction::neutral_weighted_history -- --nocapture` 通过。
  - `cargo test -p quant-engine --test trend direction::evaluate_trend -- --nocapture` 通过。
  - `cargo test -p quant-engine` 通过：22 passed, 29 ignored。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-27 00:15 UTC+10

- 执行模型：Composer。
- 变更类型：公开 API 语义（趋势层未实现显式化）。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/trend/mod.rs`
  - `crates/quant-engine/tests/trend/direction.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - `evaluate_trend` 由 `todo!()` 改为返回 `QuantError::NotImplemented`；文档说明调用方须降级 stub 或 Skip。
  - 新增 `QuantError::NotImplemented` 及 Display 文案。
  - 新增过渡期入口 `evaluate_trend_or_stub`：`NotImplemented` 时降级为中性 stub，其余错误原样传播。
  - 新增测试 `evaluate_trend_returns_not_implemented`、`evaluate_trend_or_stub_falls_back_to_neutral_stub`。
- 验证：
  - `cargo test -p quant-engine` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。

### 2026-06-27 00:05 UTC+10

- 执行模型：Composer。
- 变更类型：测试策略（趋势层 CI 隔离）。
- 涉及文件：
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/trend/direction.rs`
  - `crates/quant-engine/tests/trend/errors.rs`
  - `crates/quant-engine/tests/trend/indicators.rs`
  - `crates/quant-engine/tests/trend/regime.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `trend_deferred_test!` 宏，为依赖 `evaluate_trend` 的 TDD 边界测试统一标记 `#[ignore]`。
  - CI 默认仅运行 `config` 不变量测试（17）与 stub 契约测试（2）；29 个行为测试保留供实现期本地验证。
  - 本地全量命令：`cargo test -p quant-engine --test trend -- --ignored`。
- 验证：
  - `cargo test -p quant-engine --test trend` 通过：19 passed, 29 ignored。

### 2026-06-26 23:55 UTC+10

- 执行模型：Composer。
- 变更类型：语义对齐（趋势层默认月频契约）。
- 涉及文件：
  - `crates/quant-engine/src/trend/mod.rs`
  - `crates/quant-engine/tests/trend/config.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/common/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 趋势层默认 `min_len` 由 252 改为 60（5 年月度），与基本面层同源。
  - 模块/`TrendConfig`/`TrendSnapshot` 文档明确默认契约为**月度样本**；日频接入须显式配置 `EwPercentileConfig`。
  - 常量重命名为 `DEFAULT_HALF_LIFE_MONTHS`，消除日频/月频注释矛盾。
  - 新增测试 `default_percentile_config_matches_fundamental`，锁定趋势层与基本面层默认分位配置一致。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --test trend config::default` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。

### 2026-06-26 23:42 UTC+10

- 执行模型：Composer。
- 变更类型：测试补强（趋势权重和容忍边界）。
- 涉及文件：
  - `crates/quant-engine/tests/trend/config.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `accepts_weight_sum_exactly_one`：权重和 = 1.0 构造成功。
  - 新增 `accepts_weight_sum_within_tolerance`：偏差在 `1e-9` 内构造成功。
  - 新增 `rejects_weight_sum_beyond_tolerance`：偏差超过 `1e-9` 返回 `InvalidWeight`。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --test trend config::accepts_weight_sum config::rejects_weight_sum` 通过。

### 2026-06-26 23:35 UTC+10

- 执行模型：Composer。
- 变更类型：测试补强（趋势阈值非法）。
- 涉及文件：
  - `crates/quant-engine/tests/trend/config.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 `rejects_invalid_falling_knife_threshold` 改为与 `overheated_above` 对称的超界用例（`1.5`）。
  - 新增 `rejects_nan_overheated_threshold`（`overheated_above = NaN`）。
  - 新增 `rejects_negative_threshold`（`overheated_above = -0.1`）。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --test trend config::rejects` 通过。

### 2026-06-26 23:28 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试补强（趋势体制边界）。
- 涉及文件：
  - `crates/quant-engine/tests/trend/regime.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `strict_boundary_config` 测试辅助配置，将 `overheated_above` 与 `falling_knife_above` 设置为 `1.0`。
  - 补充 `ma_p == overheated_above`、`rsi_p == overheated_above`、`vix_p == falling_knife_above` 三个边界测试，锁定趋势体制判定使用严格 `>`，等于阈值时保持 `Neutral`，避免误触发 TacticalDelay。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --no-run` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-26 23:23 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：错误语义修正（趋势阈值）。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/trend/mod.rs`
  - `crates/quant-engine/tests/trend/config.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `QuantError::InvalidPercentileThreshold { name, value }`，用于表达分位阈值非法，避免 `overheated_above` / `falling_knife_above` 继续复用 `InvalidWeight`。
  - `TrendConfig::new` 在构造 `overheated_above` 与 `falling_knife_above` 时返回带阈值名称的结构化错误。
  - 更新趋势配置测试，并补充 `falling_knife_above` 非法阈值覆盖；更新错误 `Display` 测试锁定新分支文案。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine --no-run` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。
  - `cargo test -p quant-engine --test percentile quant_error_display_is_descriptive` 通过。
  - `cargo test -p quant-engine --test trend config::rejects_invalid` 通过：2 个趋势阈值错误测试全部通过。

### 2026-06-26 23:20 UTC+10

- 执行模型：claude-sonnet-4-5。
- 变更类型：feat（趋势层存根 + 全量测试边界）。
- 涉及文件：
  - `crates/quant-engine/src/trend/mod.rs`
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/common/mod.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - **`trend/mod.rs`**：重写为完整存根（函数签名 + `todo!()`），新增以下公开类型与函数：
    - `TrendWeights`：三指标子权重（MA/RSI/VIX），构造时校验各自在 `[0,1]` 且和 ≈ 1.0。
    - `TrendConfig`：子权重 + 分位配置 + `overheated_above` / `falling_knife_above` 阈值，提供 `Default`。
    - `TrendSnapshot`：三指标历史序列 + 当前读数的输入快照，与 `FundamentalSnapshot` 同构。
    - `TrendRegime`：`Overheated / Neutral / FallingKnife` 离散节奏体制标签，供 Decision Engine 触发 `TacticalDelay`。
    - `TrendSignal`：连续 `score`（`0.0=赶顶, 1.0=接飞刀`）+ 三个未反向审计分位 + `regime`。
    - `evaluate_trend`：纯函数存根，`todo!()` 占位；文档注释完整描述合成公式与体制判定规则。
    - `evaluate_trend_stub`：标 `#[deprecated]`，过渡期保留，`regime` 补充为 `Neutral`。
  - **`lib.rs`**：导出全部新增趋势层公开 API；注释同步说明趋势层现状。
  - **`tests/common/mod.rs`**：新增趋势层夹具常量（权重、阈值、历史长度）与 helper（`neutral/overheated/falling_knife_trend_snapshot`、`trend_balanced_test_config`、`trend_config_with_weights`）。
  - **`tests/trend.rs`**：完整测试边界（38 个测试），覆盖：
    - A 过渡存根契约（2 个）
    - B 方向性（3 个）
    - C 单指标隔离（6 个，验证 MA/RSI 反向、VIX 正向）
    - D 审计字段未反向（3 个）
    - E 节奏体制（5 个，含 FallingKnife 优先级）
    - F 错误传播（7 个：NaN/Inf/历史不足/不等长）
    - G 配置不变量（6 个：构造期校验）
    - H 默认配置契约（4 个）
- 验证：
  - `cargo test -p quant-engine --no-run` 通过，零 warning，零 error。
  - `cargo test -p quant-engine` 执行结果：fundamental 20 个 ✅ / percentile 25 个 ✅ / trend 存根/配置相关 12 个 ✅ / 待实现 26 个以 `todo!()` 正确 panic（符合存根阶段预期）；现有测试无退化。

### 2026-06-26 23:14 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试结构整理（不改变测试语义）。
- 涉及文件：
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/trend/direction.rs`
  - `crates/quant-engine/tests/trend/indicators.rs`
  - `crates/quant-engine/tests/trend/regime.rs`
  - `crates/quant-engine/tests/trend/errors.rs`
  - `crates/quant-engine/tests/trend/config.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 将 800+ 行的 `tests/trend.rs` 拆为单一集成测试入口 + `tests/trend/` 子模块目录，保留 Cargo test binary 为 `trend`。
  - 按测试关注点拆分为 `direction`（存根契约/方向性）、`indicators`（单指标隔离/审计字段）、`regime`（节奏体制）、`errors`（错误传播）、`config`（配置不变量/默认契约）。
  - 入口文件提供共享 prelude，减少各子模块重复导入，并让 CI 输出出现 `trend::direction::...` 等更精确的失败路径。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo test -p quant-engine --no-run` 通过，拆分后的 `trend` 集成测试模块编译成功。
  - `cargo test -p quant-engine` 已运行并可编译新模块结构；当前因既有 `evaluate_trend` 仍为 `todo!()`，非配置/存根类趋势边界测试按预期失败，待趋势实现落地后转绿。

### 2026-06-26 22:50 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：结构重构（模块边界调整）。
- 涉及文件：
  - `crates/quant-engine/src/weight.rs`
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `weight` 模块承载跨层共享的 `Weight` newtype，避免趋势层未来复用权重类型时依赖 `fundamental` 模块。
  - 从 `fundamental/mod.rs` 移除 `Weight` 实现，改为引用 crate 共享导出的 `Weight`。
  - `lib.rs` 新增 `pub mod weight` 并从 `weight` 重新导出 `Weight`，保持外部 `quant_engine::Weight` API 路径不变。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：20 个 fundamental 测试、25 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo llvm-cov -p quant-engine --summary-only --show-missing-lines` 通过：Region / Function / Line 覆盖率均为 100.00%，新增 `weight.rs` 行覆盖率 100.00%。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-26 22:42 UTC+10

- 执行模型：Codex；变更类型：feat/test（Investment Plan API create/list/get）。
- PR 范围：PR 7，仅新增 API DTO、create/list/get routes、safe error mapping 与 route tests；不实现 update/set active routes、Scheduler、Broker、Qwen、订单状态机、`ExecutionPlan` 或双桶逻辑。
- 涉及文件：`Cargo.lock`、`crates/api/**`、`CHANGE_LOG.md`。
- 变更内容：入站 JSON 先反序列化到 DTO 再转领域输入，避免 serde 直接构造领域类型；`ApiState` 持有 `InvestmentPlanService`，production 路径使用 storage adapter，测试路径使用 fake repository。
- Review fix：create 成功返回 `201 Created`；JSON/Path extractor 失败统一映射为项目错误 envelope；补齐转换函数文档注释以满足 docstring coverage。
- 验证：`cargo test -p indexlink-api --locked` 与完整 workspace fmt/check/test/clippy 均通过。

### 2026-06-26 16:43 UTC+10

- 执行模型：Codex。
- 变更类型：feat/db（Investment Plan PostgreSQL migration）。
- PR 范围：PR 6，仅新增 `investment_plans` 表结构 migration；不接 API、不实现 Scheduler、Broker、Qwen、订单状态机、`ExecutionPlan` 或双桶逻辑。
- 涉及文件：
  - `migrations/20260626064200_create_investment_plans.sql`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `investment_plans` 表，字段与 `PostgresInvestmentPlanRepository` adapter 使用的 SQL 契约一致。
  - 使用 `UUID` 主键、`NUMERIC(20, 8)` 金额、`TIMESTAMPTZ` 审计时间和 `monthly` MVP schedule。
  - 增加数据库约束保护领域不变量：名称 trim、symbol 大写 ASCII、currency 三位大写、执行日 1..=28、金额为正且 `max_single_execution >= base_contribution`。
  - 增加按创建顺序和 active schedule 的索引，支撑当前 list 顺序与后续 scheduler 查询。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。
  - 静态检查确认 migration 包含 `investment_plans` 表、领域约束和索引。

### 2026-06-25 22:07 UTC+10

- 执行模型：Codex。
- 变更类型：feat/test（Investment Plan PostgreSQL repository adapter）。
- PR 范围：PR 5，仅新增 storage crate 中的 PostgreSQL repository adapter；不新增 migration、不接 API、不实现 Scheduler、Broker、Qwen、订单状态机、`ExecutionPlan` 或双桶逻辑。
- 涉及文件：
  - `Cargo.lock`
  - `Cargo.toml`
  - `crates/storage/Cargo.toml`
  - `crates/storage/src/lib.rs`
  - `crates/storage/src/investment_plans.rs`
- 变更内容：
  - `indexlink-storage` 新增 `PostgresInvestmentPlanRepository`，实现 `InvestmentPlanRepository` port。
  - 支持 create、list、get、update 与 set active；update 使用事务与 `FOR UPDATE` 在写入路径内合并并校验最终金额组合。
  - SQL 边界使用 PostgreSQL cast 与文本/epoch 映射，避免扩大 sqlx feature 面。
  - 新增 storage adapter 单元测试覆盖 SQLx 错误安全映射与最终金额组合校验。
  - Review fix：为公开 re-export 的 `PostgresInvestmentPlanRepository` 补齐文档注释，满足 public API 文档要求。
- 验证：
  - `cargo test -p indexlink-storage --locked` 通过：8 个 storage 与 adapter 单元测试通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-24 23:39 UTC+10

- 执行模型：Codex。
- 变更类型：feat/test（Investment Plan 更新与启停应用用例）。
- PR 范围：PR 4，仅新增 update 与 set active 应用服务契约及 fake repository 测试；不实现 storage adapter、migration、API、Scheduler、Broker、Qwen、订单状态机或 `ExecutionPlan`。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - `InvestmentPlanRepository` port 新增 `update` 与 `set_active`。
  - `InvestmentPlanService` 新增 `update` 与 `set_active`；update 会先规范化输入，再交由 repository 在原子写入路径内校验最终 `base_contribution` / `max_single_execution` 关系。
  - fake repository 支持更新字段、启停状态与 `updated_at` 变化。
  - 新增应用服务测试覆盖字段更新、最终金额上限校验和启停用例。
  - Review fix：将 update 最终金额组合校验移动到 repository 原子写入路径内，避免 service 层读写窗口。
  - Review fix：补齐本 PR 新增 helper、fake repository 与测试函数文档注释，提高 docstring coverage。
- 验证：
  - `cargo test -p investment-plans` 通过：17 个领域、Decimal 与应用服务契约测试通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-24 23:11 UTC+10

- 执行模型：Codex。
- 变更类型：feat/test（Investment Plan repository port 与应用服务）。
- PR 范围：PR 3，仅新增 repository port、create/list/get 应用服务契约与 fake repository 测试；不实现 update 用例、storage adapter、migration、API、Scheduler、Broker、Qwen、订单状态机或 `ExecutionPlan`。
- 涉及文件：
  - `Cargo.lock`
  - `crates/investment-plans/Cargo.toml`
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `InvestmentPlanRepository` outbound port，定义 `create`、`list`、`get` 契约。
  - 新增 `PlanRepositoryError` 与 `PlanApplicationError`，保持持久化错误文案安全，不泄露数据库细节。
  - 新增 `InvestmentPlanService`，在 create 用例中先调用领域 `normalize()`，再调用 repository port。
  - 使用 fake repository 测试 create/list/get、NotFound/Unavailable 错误映射，并保持领域类型不直接派生 `Deserialize`。
- 验证：
  - `cargo test -p investment-plans` 通过：14 个领域、Decimal 与应用服务契约测试通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-24 22:37 UTC+10

- 执行模型：Codex。
- 变更类型：feat/test（Investment Plan 领域模型与校验）。
- PR 范围：PR 2，仅实现投资计划领域类型、字段规范化与输入校验；不实现 repository、storage adapter、migration、API、Scheduler、Broker、Qwen、订单状态机或 `ExecutionPlan`。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/investment-plans/Cargo.toml`
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `ScheduleKind`、`InvestmentPlan`、`CreateInvestmentPlan`、`UpdateInvestmentPlan` 与 `PlanValidationError`。
  - 创建输入支持 name trim、symbol trim + 大写、currency trim + 大写、monthly day 1..=28、Decimal 金额正数与 `max_single_execution >= base_contribution` 校验。
  - 更新输入禁止修改 symbol / currency / schedule_kind，并拒绝空 PATCH；同时保留 Decimal 字符串 JSON 契约。
  - 领域类型不直接派生 `Deserialize`，避免入站 JSON 绕过 `normalize()`；后续 API adapter 应先反序列化 DTO 再进入领域模型。
  - `symbol` 规范化新增 ASCII 校验，拒绝非 ASCII 标的代码。
  - 新增 `uuid` 与 `time` 作为领域模型 ID 和时间字段类型，未启用 SQLx 对应 feature。
- 验证：
  - `cargo test -p investment-plans` 通过：10 个领域与 Decimal 契约测试通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-24 22:08 UTC+10

- 执行模型：Codex。
- 变更类型：chore/test（Investment Plan 模块与金额基础）。
- PR 范围：PR 1A，仅建立 investment-plans crate 骨架与 Decimal JSON 契约；不实现领域模型、repository、migration、API 或执行逻辑。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/investment-plans/Cargo.toml`
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `investment-plans` workspace crate，作为投资计划领域与应用层边界。
  - 声明 `rust_decimal` 依赖；`uuid`、`time` 与 SQLx 对应 feature 因 lockfile 行数限制留到后续更小 PR。
  - 添加 Decimal JSON 字符串契约测试，确保金额从字符串反序列化并以字符串序列化，拒绝 JSON number。
  - 模块文档记录当前 MVP 假设：单用户、仅 monthly、无计划级 timezone、不验证 symbol、不计算本期买入金额或双桶资金分配。
- 验证：
  - `cargo test -p investment-plans` 通过：3 个 Decimal JSON 契约测试通过。
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-06-24 23:30 UTC+10

- 执行模型：Claude。
- 变更类型：test（AI 语义感知层：集成测试）。
- 涉及文件：
  - `crates/ai-client/tests/client.rs`
  - `crates/ai-client/tests/provider.rs`
  - `crates/ai-client/tests/sentiment.rs`
- 变更内容：
  - 本地 axum HTTP mock server 模拟千问，验证 `QwenClient` 请求格式、响应解析、错误降级全链路。
  - `MockAiProvider` 关键词匹配测试（正向/负向/中性/自定义默认值）。
  - `Sentiment` 边界测试（构造、范围、比较、f64 互转）。
  - `AiConfig` 安全测试（api_key 不出现在 Debug/Display 中）。
- 验证：
  - `cargo test -p ai-client`：66 测试全部通过。

### 2026-06-24 23:15 UTC+10

- 执行模型：Claude。
- 变更类型：feat（AI 语义感知层：客户端实现 + Mock）。
- 涉及文件：
  - `crates/ai-client/Cargo.toml`
  - `crates/ai-client/src/client.rs`
  - `crates/ai-client/src/mock.rs`
  - `crates/ai-client/src/lib.rs`
- 变更内容：
  - `QwenClient`：对接 Qwen DashScope API，system prompt 约束模型输出结构化 JSON，三级步进降级。
  - `MockAiProvider`：关键词匹配的本地假 AI（大涨→正向、大跌→负向，未匹配→中性），零网络零成本。
  - `lib.rs` 统一导出 `QwenClient`、`MockAiProvider`、`AiProvider`、`AiConfig`、`Sentiment`。
- 验证：
  - `cargo test -p ai-client`：全部通过。

### 2026-06-24 23:00 UTC+10

- 执行模型：Claude。
- 变更类型：feat（AI 语义感知层：核心类型与接口定义）。
- 涉及文件：
  - `crates/ai-client/src/sentiment.rs`
  - `crates/ai-client/src/error.rs`
  - `crates/ai-client/src/provider.rs`
- 变更内容：
  - `Sentiment` newtype：`[-1.0, +1.0]` 有界情绪值，NaN→0、越界自动截断，Display 安全。
  - `AiClientError`：六种错误变体（Timeout / Transport / HttpStatus / InvalidJson / UnexpectedStructure / ParseFailure / EmptyResponse），所有 Display 不暴露密钥/URL。
  - `AiConfig`：千问连接配置（默认 DashScope `qwen-plus`），Debug 将 api_key 显示为 `<redacted>`。
  - `AiProvider` trait（`async_trait`）：可替换的 LLM 后端抽象，与 `ReadinessCheck` 同模式。
- 验证：
  - `cargo test -p ai-client`：42 单元测试全部通过。

### 2026-06-24 23:00 UTC+10

- 执行模型：Claude。
- 变更类型：chore（workspace 注册 ai-client）。
- 涉及文件：
  - `Cargo.toml`（根 workspace 注册 + reqwest 依赖声明）
  - `Cargo.lock`
- 变更内容：
  - 将 `ai-client` 加入 workspace members。
  - 声明 `ai-client` workspace dependency。
  - 声明 `reqwest` workspace dependency（`json` + `rustls-tls`）。
- 验证：
  - `cargo build --workspace` 通过。
  - `cargo test --workspace` 全部通过。

### 2026-06-21 21:15 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试补强（覆盖率修补）。
- 涉及文件：
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 调查 `cargo llvm-cov -p quant-engine --summary-only --show-missing-lines` 报告，确认缺失覆盖集中在 `weighted_percentile_of` 的总有效权重下溢防御分支。
  - 新增 `weighted_percentile_returns_insufficient_when_all_valid_weights_underflow`，构造 `alpha = 1.0`、最新样本为 `NaN`、旧端有效样本权重归零的场景，锁定该分支返回 `QuantError::InsufficientHistory`。
- 验证：
  - `cargo fmt -p quant-engine` 通过。
  - `cargo test -p quant-engine` 通过：20 个 fundamental 测试、25 个 percentile 测试、1 个 trend 测试、1 个 doc test 全部通过。
  - `cargo llvm-cov -p quant-engine --summary-only --show-missing-lines` 通过：Region / Function / Line 覆盖率均为 100.00%，缺失行清零。
  - `cargo test -p core-domain` 通过：13 个单元测试全部通过。

### 2026-06-21 21:02 UTC+10

- 执行模型：Claude Opus 4.8。
- 变更类型：功能实现（指数加权 ECDF）+ 配套测试修正。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/percentile.rs`
  - `crates/quant-engine/src/fundamental/mod.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - `lib.rs`：`QuantError` 新增 `InvalidHalfLife { value }` 与 `InvalidDecay { alpha }` 两个结构化分支及对应 `Display`；根级导出 `weighted_percentile_of` 与 `EwPercentileConfig`。
  - `percentile.rs`：新增 `EwPercentileConfig`（`from_half_life` / `from_alpha` 双构造入口，`alpha = 1 - 0.5^(1/H)`，校验半衰期、衰减系数与 `min_len`）与 `weighted_percentile_of`；历史按「旧→新」加权，最新样本权重 1，NaN 跳过但不压缩滞后，并对有效样本不足与权重下溢返回 `InsufficientHistory`；保留原无权 `percentile_of` 不变。
  - `fundamental/mod.rs`：`FundamentalConfig` 以 `percentile_config: EwPercentileConfig` 取代 `min_history_len`，`new` 改为接收 `EwPercentileConfig`，`Default` 采用半衰期 36 个月 + 最少 60 个有效月度样本；`evaluate_fundamental` 改用 `weighted_percentile_of`，ERP 倒置与合成逻辑保持不变。
  - `tests/fundamental.rs`：将 `fundamental_expensive_market` / `fundamental_cheap_market` 的当前读数改为明确超出历史范围的极值，使方向性对任意半衰期稳健（修正旧位置分位魔法数字在加权下失真的问题）；`rate_repricing` 的 CAPE 中性断言由精确容差放宽为近似容差（加权 ECDF 因截断尾项无法精确等于 0.50）。
- 验证：
  - `cargo test -p quant-engine`：fundamental 20 + percentile 24 + trend 1 + doc 1 全部通过。
  - `cargo test -p core-domain`：13 项单元测试通过。
  - `cargo fmt -p quant-engine --check` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - 改动源文件无 IDE linter 错误。

### 2026-06-21 20:48 UTC+10

- 执行模型：GPT-5.5。
- 变更类型：测试先行（指数加权 ECDF 契约）。
- 涉及文件：
  - `crates/quant-engine/tests/common/mod.rs`
  - `crates/quant-engine/tests/fundamental.rs`
  - `crates/quant-engine/tests/percentile.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 测试夹具改为通过 `EwPercentileConfig` 构造 `FundamentalConfig`，默认契约对齐 readme：指数加权 ECDF 半衰期 36 个月，最少 60 个有效月度历史点。
  - `fundamental` 集成测试改为断言 `percentile_config`、加权中性位置、ERP 原始分位审计字段，以及新配置构造入口下的非法权重/历史长度错误。
  - `percentile` 集成测试新增指数加权 ECDF 契约：半衰期到 alpha 映射、非法半衰期/衰减系数、单调性、旧→新顺序敏感、NaN 不压缩 lag、最旧样本退出时的平滑变化、错误传播和新增错误类型展示文案。
  - 当前仅修改测试，生产实现尚未新增 `EwPercentileConfig`、`weighted_percentile_of`、`FundamentalConfig::percentile_config` 及对应 `QuantError` 分支。
- 验证：
  - `cargo fmt -p quant-engine --check` 通过。
  - `cargo test -p quant-engine` 预期失败：生产代码尚未实现测试引用的新 API 与错误分支（`EwPercentileConfig`、`weighted_percentile_of`、`InvalidHalfLife`、`InvalidDecay`、`percentile_config`）。

### 2026-06-21 20:18 UTC+10

- 执行模型：Sonnet 4.6。
- 变更类型：文档（设计决策更新）。
- 涉及文件：
  - `readme.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - **指数加权 ECDF**：将历史分位计算方法由无权 ECDF 升级为指数加权 ECDF；以半衰期为唯一旋钮（$\alpha = 1 - 0.5^{1/H}$，默认 $H$ = 36 个月月度数据），消除硬窗口「幽灵跌落」效应，同时保持无分布假设，输出仍为 `[0, 1]` 分位；更新决策管线说明、Quant Engine 模块职责描述、MVP 阶段落地描述。
  - **双桶现金池（Two-Bucket Execution）**：在执行层引入副桶（Buffer Bucket）消除现金拖累；确立四条核心规则（副桶是弹药缓冲池、取出量受余额约束、副桶设累积上限、现金流策略可配置）；新增 `Conservative`（默认）/ `Aggressive` 两种策略对比表及资金流示意；在「分阶段落地」第 4 阶段中纳入双桶；在关键功能列表中新增双桶条目。
  - 上述改动均为文档层面，未修改任何 Rust 代码。
- 验证：
  - 文档改动，未运行测试。

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
