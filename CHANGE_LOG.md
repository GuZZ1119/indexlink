# Change Log

## Unreleased

### 2026-07-18 22:30 AEST

- 执行模型：GPT-5。
- 变更类型：Futu/Moomoo OpenD / Part 3：server paper broker 装配与受控 virtual-account smoke。
- 涉及文件：
  - `.env.example`
  - `API_MANAGEMENT.md`
  - `Cargo.lock`
  - `apps/server/Cargo.toml`
  - `apps/server/src/config.rs`
  - `apps/server/src/main.rs`
  - `crates/broker/src/opend_session.rs`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/src/state.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - server 仅在显式设置 `OPEND_PROVIDER=futu|moomoo` 时读取 loopback OpenD 的 host、port 与可选 account id，并构造固定 `Paper`、live gate 关闭的 `OpenDConnectionConfig`；未配置时保留 `MockBroker`。没有 live environment 配置项。
  - 配置 OpenD 后，server 在监听 HTTP 前建立 `OpenDPaperSession` 并注入 `OpenDPaperBroker`；连接、登录状态或模拟账户选择失败会阻止启动，不会悄然退回 mock broker。
  - `ApiState` 增加受文档约束的 broker 注入入口；`Decision Preview` 的可选订单始终经该 broker port，继续沿用 due/action 门控和安全 API 错误契约。
  - 新增默认 ignored 的真实 paper-order smoke：必须设置 `OPEND_SMOKE_CONFIRM=submit-paper-order`、显式 `OPEND_ACCOUNT_ID`、唯一 idempotency key、symbol 与 quantity，才会以临时内存 SQLite 计划穿过 production composition root 发送一笔虚拟订单。凭据、账户和订单 ID 不进入日志或断言。
  - 该提交只提供可执行的本机 smoke 入口；实际虚拟订单将在本机 OpenD 已登录并配置后单独执行与记录。
  - 审查修正：API 文档中的交互变量读取改用 Bash 兼容的 `read -r -p`；`localhost` 在 server 配置阶段固定规范化为字面 `127.0.0.1`，其他地址必须解析为 `IpAddr` 且满足 `is_loopback()`，同时 raw TCP adapter 复用相同 IP 语义。
  - 审查修正：server composition root 现在接收可替换的异步 broker factory；非 ignored 测试覆盖 factory session 失败阻止启动，以及 factory 成功后 HTTP Decision Preview 实际调用替换后的 broker，而非默认 mock。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-server --locked` 通过（26 passed、1 ignored；含 loopback 边界和 OpenD factory 成功/失败组合根测试）。
  - `cargo test -p indexlink-api --locked` 通过（33 tests，含 broker 注入替换默认 mock 的聚焦测试）。
  - `cargo test -p broker --locked` 通过（36 tests，含 raw TCP loopback IP 语义边界）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-server -p indexlink-api -p broker --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p indexlink-api --no-deps --locked` 通过。
  - 本机未检测到可执行的 OpenD smoke 配置前，不提交任何虚拟订单；真实 smoke 结果待 OpenD GUI 登录、paper account 与显式确认变量就绪后补记。

### 2026-07-18 21:42 AEST

- 执行模型：GPT-5。
- 变更类型：Futu/Moomoo OpenD / Part 2：paper order gateway。
- 涉及文件：
  - `crates/api/src/error.rs`
  - `crates/broker/src/lib.rs`
  - `crates/broker/src/opend_session.rs`
  - `API_MANAGEMENT.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - `OpenDPaperSession` 现在实现既有 `OpenDOrderGateway`，通过同一条串行 TCP 通道提交 PlaceOrder（2202）；每笔请求携带 OpenD connection id、递增的 anti-replay packet serial、已选模拟账户和 `Paper` 环境。
  - MVP 明确仅支持美股股票/ETF 订单：普通限价单映射 OpenD `Normal`，市价单映射 `Market`；回执必须确认模拟环境、同一账户和美股市场，才会生成 `BrokerOrderAck::Accepted`。
  - idempotency key 不直接写入 provider 备注，而是确定性 SHA-1 摘要，控制在 OpenD 64-byte remark 限制内；该备注只用于关联，当前 adapter 不对网络失败自动重试，也不声称跨请求幂等。不把 provider 的拒绝文案、网络细节或账户信息暴露给调用方。
  - 新增安全 `BrokerError::Rejected`，API 映射为既有统一 `bad_request` envelope；请求自身的环境不匹配仍为 `EnvironmentMismatch` / `bad_request`，只有回执中的账户/环境不匹配、协议畸形等才映射为 `Unavailable`。
  - 当 PlaceOrder 已开始写入后发生写入、flush、读取超时、断连或响应格式异常时，返回不可自动重试的 `OutcomeUnknown`；API 以 `409 order_outcome_unknown` 明确要求客户端不要重试，避免未知结果被 `503` 诱导重复下单。
  - 本 PR 只使用本地协议 fake，不连接真实 OpenD、不提交任何虚拟订单；server 注入和本机虚拟账户 smoke 仍留给 `opend-03-server-wiring-smoke`。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p broker --locked` 通过（35 tests，含 PlaceOrder protocol fake、未发送前置拒绝与结果未知边界）。
  - `cargo test -p indexlink-api --locked` 通过（33 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p broker -p indexlink-api --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p broker --no-deps --locked` 通过。

### 2026-07-17 23:03 AEST

- 执行模型：GPT-5。
- 变更类型：Futu/Moomoo OpenD / Part 1：paper-only raw TCP session transport。
- 涉及文件：
  - `Cargo.lock`
  - `Cargo.toml`
  - `crates/broker/Cargo.toml`
  - `crates/broker/src/lib.rs`
  - `crates/broker/src/opend_session.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `OpenDPaperSession`，使用 Futu/Moomoo 官方 raw TCP 帧（`FT` 标志、44 字节 little-endian header、JSON body 与 SHA-1 完整性校验）建立会话；本阶段完成 `InitConnect`（1001）、交易登录状态检查（1002）、交易账户列表（2001）和按服务端间隔发送的 KeepAlive（1004），不提交订单。
  - 会话在 `InitConnect` 后确认 `trdLogined=true`；OpenD 的用户登录仍由本机 OpenD 进程负责，IndexLink 不读取、传输或记录 Futu/Moomoo 密码、token 或登录凭据。
  - 仅接受 `Paper` 环境且 live gate 关闭；当未显式指定 account id 时，必须恰好有一个模拟账户，多个候选或无候选都会安全拒绝。显式 account id 也只能匹配模拟账户。
  - raw TCP 暂时只允许 loopback OpenD（`127.0.0.1`、`::1`、`localhost`）；`localhost` 会固定映射为字面回环地址，不依赖系统 hosts 解析。官方可选 RSA packet encryption 尚未实现，拒绝远端明文 TCP 以避免交易元数据跨网络泄露。
  - 通过独立 golden frame 与损坏帧拒绝测试覆盖固定帧编码/完整性校验；本地 TCP protocol fake 覆盖初始化、KeepAlive、登录状态、默认/显式 paper account 选择及远端主机拒绝。没有真实下单、server 注入或 virtual-account smoke，本部分保留给后续两份 OpenD PR。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p broker --locked` 通过（27 tests，含独立 golden/损坏帧、KeepAlive 与 loopback TCP protocol fake）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p broker --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p broker --no-deps --locked` 通过。

### 2026-07-16 23:58 AEST

- 执行模型：GPT-5。
- 变更类型：阿里云 Qwen 市场情绪 API 接入 / OpenD 后续实施计划。
- 涉及文件：
  - `.env.example`
  - `API_MANAGEMENT.md`
  - `Cargo.lock`
  - `apps/server/Cargo.toml`
  - `apps/server/src/config.rs`
  - `apps/server/src/main.rs`
  - `crates/ai-client/src/news.rs`
  - `crates/api/Cargo.toml`
  - `crates/api/src/error.rs`
  - `crates/api/src/routes/market_sentiment.rs`
  - `crates/api/src/routes/mod.rs`
  - `crates/api/src/state.rs`
  - `crates/api/tests/market_sentiment.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - server 读取可选的 `DASHSCOPE_API_KEY`、base URL、model、timeout、max tokens 与 temperature；Key 未配置时不阻止本地 SQLite 服务启动，市场情绪路由统一返回安全的 `503`。配置和日志不会输出 Key。
  - `ApiState` 新增受控的新闻源与 AI provider 注入点；production composition root 使用 `RssNewsSource + QwenClient`，测试使用 fake adapter，不发起网络请求。
  - 新增 `POST /market-sentiment/preview`，返回 `score` 与稳定 `positive` / `neutral` / `negative` 标签；新闻源和 Qwen 失败统一映射为既有 JSON `service_unavailable` 错误，不向客户端泄露 provider 内部错误。
  - 审查修正：明确该路由尚未自动串联至 Decision Preview；真实 Key smoke 改为 shell 隐藏输入后 export，避免 Key 写入命令历史；server 将 Qwen 装配提取为 helper 并覆盖已配置/未配置两个分支；API 层在安全映射前记录已脱敏的管线错误，便于排障。
  - `ai-client` 的市场情绪管线允许通过 trait object 注入，保持 library 与 HTTP adapter 的六边形边界。
  - 真实 Key smoke 使用已有忽略式 Qwen 新闻集成测试；API 文档补充启动后 HTTP smoke 命令。真实凭据不进入仓库、日志或测试断言。
  - OpenD 按三份 PR 实施：
    1. `opend-01-session-transport`：在 broker adapter 内实现 TCP/SDK transport 边界、连接生命周期、认证与 paper account 选择；以协议 fake 覆盖，不改 server 组合根。
    2. `opend-02-order-gateway`：实现 market/limit 下单请求与 ack 转换、超时、网络错误和安全脱敏；继续强制 paper-only，不引入 live 下单路径。
    3. `opend-03-server-wiring-smoke`：从 server 环境变量构造并注入真实 `OpenDPaperBroker`，替换 production 固定 `MockBroker`，保留未配置时的 Mock 回退；使用虚拟账户执行一次真实 smoke 并记录安全操作步骤。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-api --offline` 通过（31 tests，含 fake provider 注入、未配置 provider 与 provider 错误映射）。
  - `cargo test -p indexlink-server --locked` 通过（21 tests，含可选 Qwen 配置、参数解析与 Key 空值拒绝，以及 Qwen 装配的已配置/未配置分支）。
  - `cargo test -p core-domain --offline` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p ai-client -p indexlink-api -p indexlink-server --all-targets --all-features --locked -- -D warnings` 通过。
  - 已尝试 `cargo test -p ai-client --test news real_cnbc_with_qwen -- --ignored --nocapture`；当前环境未设置 `DASHSCOPE_API_KEY`，测试在网络请求前退出。待本机配置 Key 后按 API 文档命令重跑；不得在 CI 或日志中输出凭据。

### 2026-07-15 23:01 AEST

- 执行模型：GPT-5。
- 变更类型：SQLite runtime 审查修正。
- 涉及文件：
  - `Cargo.lock`
  - `apps/server/src/config.rs`
  - `crates/storage/Cargo.toml`
  - `crates/storage/src/sqlite_decision_records.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 配置层现在只接受非空 `sqlite:` `DATABASE_URL`，会在启动连接前明确拒绝遗留 `postgres://` URL。
  - 提炼 SQLite row 列读取助手，统一 `try_get` 的安全错误映射，保留 UUID、金额、时间和 JSON 的原有解析语义。
  - storage crate 复用 workspace `tracing` 依赖；decision record SQLite adapter 在折叠非 `RowNotFound` SQLx 错误前记录内部 warning，HTTP/领域层仍只得到安全的 `Unavailable`。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-storage --offline` 通过（30 tests）。
  - `cargo test -p indexlink-server --offline` 通过（15 tests）。
  - `cargo test -p indexlink-api --offline` 通过（28 tests）。
  - `cargo test -p core-domain --offline` 通过（13 tests）。
  - `cargo check --workspace --offline` 通过。
  - `cargo clippy -p indexlink-storage -p indexlink-api -p indexlink-server --all-targets --all-features --offline -- -D warnings` 通过。

### 2026-07-15 22:44 AEST

- 执行模型：GPT-5。
- 变更类型：SQLite decision record 审查修正 / 演示 MVP 缺口审计。
- 涉及文件：
  - `crates/storage/src/sqlite.rs`
  - `crates/storage/src/sqlite_decision_records.rs`
  - `migrations/sqlite/20260715012000_reject_null_decision_record_snapshots.sql`
  - `CHANGE_LOG.md`
- 变更内容：
  - 修正 JSON `null` 绕过问题：SQLite adapter 的读取端现在会拒绝 `null` 快照并安全映射为 repository unavailable；可选快照若以 JSON `null` 而非 SQL `NULL` 存储，也同样不会进入领域模型。
  - 新增 SQLite migration，以 insert/update trigger 阻止任一必填 decision record snapshot（execution、fundamental、trend、decision）被直接写入 JSON `null`；既有损坏行不会被静默修复，而会在读取时被拒绝，避免伪造有效审计记录。
  - 全项目演示 MVP 审计结论：本地 SQLite、计划管理、双桶执行预览、70/20/10 纯函数决策、MockBroker 串联和只读 decision history 已可用；但 `apps/web` 当前仍是 Vite 模板，尚未实现演示界面。
  - 演示 MVP 的阻塞项依优先级为：
    1. 将 DashScope/Qwen client 接入 server config、API state 与真实 market sentiment route，并以真实 key smoke test。
    2. 实现 Futu/Moomoo OpenD 的真实 TCP/SDK gateway transport，注入 server，并以 paper/virtual account 提交订单和获取 ack。
    3. 将 Decision Preview 升级为受控服务端编排：接入真实 Qwen、确定 fundamental/trend 的演示输入来源、生成分层 summary，并在成功结果后自动写入本地 decision record。
    4. 由前端负责方把当前 Vite 模板替换为计划、信号、决策、双桶、paper order 与 history 的演示闭环。
    5. 补全真实凭据的端到端 smoke 文档；Docker Compose 的 SQLite named volume 写权限仍需在有 Docker 的环境实测并修正（当前镜像以内置目录 chown，挂载 volume 后权限可能变化）。
  - 自动 Scheduler、成交回报状态机、多用户和 live trading 均不属于本次“演示可用”最小 MVP 的阻塞项。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-storage --locked` 通过（30 tests，覆盖 adapter 读取拒绝 JSON `null`、migration 阻止 insert/update 直接写入 JSON `null`）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-storage --all-targets --all-features --locked -- -D warnings` 通过。

### 2026-07-15 22:30 AEST

- 执行模型：GPT-5。
- 变更类型：SQLite 本地持久化 / Part 3：Decision Record adapter 与 production runtime wiring。
- 涉及文件：
  - `.env.example`
  - `README.md`
  - `API_MANAGEMENT.md`
  - `docs/minimum_mvp.md`
  - `deployment/Dockerfile`
  - `deployment/docker-compose.yml`
  - `apps/server/src/config.rs`
  - `apps/server/src/main.rs`
  - `crates/api/src/state.rs`
  - `crates/storage/src/lib.rs`
  - `crates/storage/src/sqlite.rs`
  - `crates/storage/src/sqlite_decision_records.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `SqliteDecisionRecordRepository`，以静态 SQLite 查询实现审计快照的 create、按计划有上限列表查询与单条查询；金额沿用固定精度文本编码，JSON、UUID、时间或状态快照损坏时安全映射为后端不可用。
  - `ApiState` 生产组合根改用 SQLite plan 与 decision record adapter，旧 PostgreSQL adapter 保留但不再进入默认运行路径。
  - server 使用 SQLite 默认 URL 连接本地文件，并在 HTTP 监听前执行编译期嵌入的 migration；migration 失败将阻止服务启动。
  - 配置、示例环境变量、Dockerfile 与 Compose 改为本地 SQLite。Compose 使用 `sqlite-data` volume 保留数据，不再依赖 PostgreSQL 容器。
  - 更新 MVP 与 API 文档，明确默认本地存储、旧 PostgreSQL adapter 的兼容定位，以及 Decision Preview 自动存证仍是后续工作。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-storage --locked` 通过（30 tests，含 SQLite decision record 的外键、金额精度、UTC `Z` 时间、JSON 快照与 history limit）。
  - `cargo test -p indexlink-api --locked` 通过（28 tests）。
  - `cargo test -p indexlink-server --locked` 通过（14 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-storage -p indexlink-api -p indexlink-server --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p indexlink-storage --no-deps --locked` 通过。
  - 使用临时 SQLite 文件启动 `indexlink-server`，`GET /ready` 返回 `{"status":"ready","database":"ok"}`；确认 migration 在监听 HTTP 前完成。
  - 未安装 Docker CLI，未能在本机执行 `docker compose ... config`；Compose 文件仅做静态审查。

### 2026-07-15 AEST

- 执行模型：GPT-5。
- 变更类型：SQLite 本地持久化 / Part 2：Investment Plan repository adapter。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/storage/src/lib.rs`
  - `crates/storage/src/sqlite.rs`
  - `crates/storage/src/sqlite_investment_plans.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `SqliteInvestmentPlanRepository`，实现计划的 create、list、get、原子 update 与 set_active。
  - 金额在 SQLite 边界使用固定 12 位整数 + 8 位小数文本编码；不满足原 PostgreSQL `NUMERIC(20, 8)` 范围的值不会写入。
  - update 使用 `BEGIN IMMEDIATE` 获取 SQLite 写锁，在同一事务中读取、合并、校验并更新最终金额，避免并发读改写窗口。
  - 金额编码会归一化仅含尾随零的额外小数位；只有归一化后会改变数值的精度溢出才拒绝写入。
  - `updated_at` 始终由 SQLite 写为 UTC RFC 3339 `Z` 格式，并通过当前时间与前值加 1ms 的较大值保证严格递增；读取端解析并拒绝损坏的时间或金额快照。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-storage --locked` 通过（24 tests）。
  - `cargo test -p investment-plans --locked` 通过（31 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-storage --all-targets --all-features --locked -- -D warnings` 通过。
  - `cargo doc -p indexlink-storage --no-deps` 通过。

### 2026-07-15 AEST

- 执行模型：GPT-5。
- 变更类型：SQLite 本地持久化 / Part 1：基础设施与 migration。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/storage/src/lib.rs`
  - `crates/storage/src/sqlite.rs`
  - `migrations/sqlite/20260715010000_create_investment_plans.sql`
  - `migrations/sqlite/20260715011000_create_decision_records.sql`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增独立 `SqliteStorage`，为本地 `.db` 文件提供连接、外键、WAL、busy timeout、健康检查和编译期嵌入的 migration 基础设施；尚未替换现有 PostgreSQL production wiring。
  - 新增 SQLite 专用 baseline schema：UUID、精确金额、时间和 JSON snapshot 使用 TEXT；金额使用固定 12 位整数 + 8 位小数格式，以文本比较保留正数与 `max_single_execution >= base_contribution` 约束；时间强制为 UTC RFC 3339 `Z` 格式。
  - PostgreSQL migration 与 SQLite migration 分目录维护，避免不同数据库执行不兼容 SQL。
- 三个 PR 计划：
  1. **Part 1（本 PR）**：SQLite 连接与 migration 基础设施、SQLite baseline schema、聚焦迁移测试。
  2. **Part 2**：实现 SQLite Investment Plan repository adapter，并验证创建、读取、原子更新、启停及本地持久化。
  3. **Part 3**：实现 SQLite Decision Record repository adapter，并将 server/API/config/Docker 默认 wiring 切换为本地 SQLite 文件；同时移除 MVP 对 PostgreSQL 容器的运行依赖。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-storage --locked` 通过（17 tests，含 SQLite 内存数据库 migration 与金额、时间约束执行）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo check --workspace --locked` 通过。
  - `cargo clippy -p indexlink-storage --all-targets --all-features --locked -- -D warnings` 通过。

### 2026-07-15 00:36 AEST

- 执行模型：GPT-5。
- 变更类型：Decision Record 持久化（Part 3：History Query API）。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/decision-records/src/lib.rs`
  - `crates/api/Cargo.toml`
  - `crates/api/src/error.rs`
  - `crates/api/src/state.rs`
  - `crates/api/src/routes/mod.rs`
  - `crates/api/src/routes/decision_records.rs`
  - `crates/api/tests/decision_records.rs`
  - `crates/api/tests/health.rs`
  - `API_MANAGEMENT.md`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `GET /investment-plans/:id/decisions?limit=` 与 `GET /decisions/:id`，分别查询已存在计划的最新 decision record 列表和单条审计记录。
  - history list 默认返回 50 条、最大 200 条；非法 UUID、query 参数或 limit 统一映射为安全的 `bad_request`，不存在计划或记录映射为 `not_found`。
  - 生产 `ApiState` 注入现有 `PostgresDecisionRecordRepository`；隔离测试状态使用显式 unavailable repository，避免意外访问真实数据库。
  - Decision record 的 `created_at` JSON 序列化改为 RFC 3339 字符串，避免将 `OffsetDateTime` 内部数组暴露给前端。
  - 将 `service_unavailable` 文案改为中性 `service is unavailable`，准确覆盖数据库、broker 与 decision record 后端不可用。
  - 更新 API 管理与 MVP 文档：明确只读 history API 已可用，但 Decision Preview 自动写入审计记录仍留待后续独立 PR。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-api --locked` 通过（28 tests）。
  - `cargo test -p decision-records --locked` 通过（10 tests）。
  - `cargo test -p core-domain --locked` 通过（13 tests）。
  - `cargo clippy -p indexlink-api --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过；2 个依赖真实网络/API key 的测试按项目约定 ignored。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-13 23:56 UTC+10

- 执行模型：GPT-5。
- 变更类型：PR review fix。
- 涉及文件：
  - `crates/decision-records/src/lib.rs`
  - `crates/storage/src/decision_records.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - Review fix：新增 `DecisionRecordListQuery`，为 decision record 历史查询提供默认分页上限，避免 `list_by_plan` 长期无限制返回全部记录。
  - Review fix：`PostgresDecisionRecordRepository::list_by_plan` 绑定 `LIMIT` 参数，并保留按 `created_at DESC, id DESC` 的稳定排序。
  - Review fix：将 decision record storage SQL 改为编译期静态常量，避免 `format!()` 拼接查询语句造成 SAST 噪音和运行时分配。
  - 补充聚焦测试，覆盖 list query 边界、bounded list 服务路径和静态 SQL 中的 `LIMIT` 约束。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check -p decision-records --locked` 通过。
  - `cargo check -p indexlink-storage --locked` 通过。
  - `cargo test -p decision-records --locked` 通过。
  - `cargo test -p indexlink-storage --locked` 通过。
  - `cargo clippy -p indexlink-storage --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-13 23:22 UTC+10

- 执行模型：GPT-5。
- 变更类型：Decision Record 持久化（Part 2：PostgreSQL storage adapter）。
- 涉及文件：
  - `Cargo.lock`
  - `crates/storage/Cargo.toml`
  - `crates/storage/src/lib.rs`
  - `crates/storage/src/decision_records.rs`
  - `migrations/20260713093000_create_decision_records.sql`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `decision_records` PostgreSQL migration，用于保存 decision preview / execution 的审计快照。
  - migration 使用 `JSONB` 保存 execution、fundamental、trend、sentiment、decision 与 broker 输入输出快照，并通过 `plan_id` 外键关联 `investment_plans`。
  - 新增 `PostgresDecisionRecordRepository`，实现 `DecisionRecordRepository` 的 create、list_by_plan 与 get。
  - storage adapter 在写入前再次调用 `CreateDecisionRecord::normalize()`，避免绕过服务层时写入未规范化数据。
  - 为 plan + created_at 与全局 created_at 添加查询索引，支持后续 history API。
  - storage adapter 使用 SQL `::jsonb` 写入，并在 Rust 侧解析 JSON snapshot，避免扩大 SQLx workspace feature 面。
- 接下来计划：
  1. Part 3：新增 decision record 查询 API，并更新 `API_MANAGEMENT.md` / `docs/minimum_mvp.md`。
  2. 后续阶段：在 Decision Preview API 中接入持久化写入。
  3. 后续阶段：接入阿里云 Qwen Market Sentiment API 与 Futu/Moomoo OpenD paper gateway transport。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check -p indexlink-storage --locked` 通过。
  - `cargo test -p indexlink-storage --locked` 通过。
  - `cargo clippy -p indexlink-storage --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-12 00:15 UTC+10

- 执行模型：GPT-5。
- 变更类型：Decision Record 持久化（Part 1：领域层）。
- 涉及文件：
  - `Cargo.toml`
  - `Cargo.lock`
  - `crates/decision-records/Cargo.toml`
  - `crates/decision-records/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `decision-records` crate，定义 `DecisionRecord`、`CreateDecisionRecord`、repository port 与应用服务。
  - 使用 JSON snapshot 字段保存 execution、fundamental、trend、sentiment、decision 与 broker 输入输出，优先保留审计输入快照而不是只保存结论。
  - 自查修正：新增 `DecisionExecutionStatus`，避免执行状态以任意字符串绕过领域边界。
  - 自查修正：新增 `CreateDecisionRecord::normalize()` 与 `DecisionRecordValidationError`，在 repository 前校验 symbol、currency、planned contribution、summary 和必需 JSON snapshot。
  - 自查修正：snapshot 字段 rustdoc 明确不得保存 API key、account id、OpenD 密码或其他 secrets。
  - Review fix：补充 normalize 边界测试，覆盖 symbol、currency、summary、必需 snapshot 与可选 snapshot 的非法分支。
  - 新增领域层单元测试，覆盖 create/list/get 服务路径、repository not found 映射、创建输入规范化和非法输入拒绝。
- 接下来计划：
  1. Part 2：新增 PostgreSQL `decision_records` migration 与 `PostgresDecisionRecordRepository`。
  2. Part 3：新增 decision record 查询 API，并更新 `API_MANAGEMENT.md` / `docs/minimum_mvp.md`。
  3. 后续阶段：接入阿里云 Qwen Market Sentiment API，并在真实执行链路中写入 decision record。
  4. 后续阶段：实现 Futu/Moomoo OpenD paper gateway transport，继续默认 paper trading。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check -p decision-records --locked` 通过。
  - `cargo test -p decision-records --locked` 通过。
  - `cargo clippy -p decision-records --all-targets --all-features -- -D warnings` 通过。

### 2026-07-10 11:53 UTC+10

- 执行模型：GPT-5。
- 变更类型：API 管理文档。
- 涉及文件：
  - `API_MANAGEMENT.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增根目录 `API_MANAGEMENT.md`，面向前端对接整理当前已有 API、请求/响应约定、统一错误格式和对接顺序。
  - 补充待实现 API 清单，包括阿里云 Qwen market sentiment、fundamental/trend signal、Futu/Moomoo OpenD paper trading、Decision Preview 真实上游升级和 decision record/history。
- 验证：
  - 文档变更，无需运行 Rust 测试。
  - `git status --short` 已检查。

### 2026-07-09 23:43 UTC+10

- 执行模型：GPT-5。
- 变更类型：PR review fix。
- 涉及文件：
  - `crates/api/Cargo.toml`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/tests/decision_preview.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - Review fix：将 `paper_order` DTO 到 broker 领域请求的结构校验前置，避免 waiting 或 `TacticalDelay` 路径静默接受非法 market/limit payload。
  - Review fix：在可替换 broker port 调用外层增加 5 秒超时，超时后返回安全的 `service_unavailable` API envelope。
  - 新增回归测试，覆盖 waiting 与 tactical delay 路径中非法 paper order 仍返回 `bad_request`。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p indexlink-api --locked` 通过。
  - `cargo clippy -p indexlink-api --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-09 23:10 UTC+10

- 执行模型：GPT-5。
- 变更类型：Decision Preview API + MockBroker 串联。
- 涉及文件：
  - `Cargo.lock`
  - `crates/api/Cargo.toml`
  - `crates/api/src/error.rs`
  - `crates/api/src/routes/mod.rs`
  - `crates/api/src/routes/decision_preview.rs`
  - `crates/api/src/state.rs`
  - `crates/api/tests/decision_preview.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `POST /investment-plans/:id/decision-preview`，将 investment plan、执行预览、双桶拆分、70/20/10 decision engine 与 broker paper order 串成后端演示闭环。
  - API 入站仍使用 DTO 转换到领域类型，复用 `Percentile`、`PreviewInvestmentPlanExecution`、`BucketAllocationRatio` 与 broker order 构造器的不变量校验。
  - `ApiState` 新增可替换 broker port，生产默认使用 `MockBroker::paper_only()`，测试可注入共享 mock broker 观察订单提交。
  - decision preview 仅在计划 due 且 action 不是 `Skip` / `TacticalDelay` 时提交 paper order；waiting、inactive、跳过和战术延迟都不会触发 broker。
  - 返回执行预览、decision score/multiplier/action、可选 paper order ack 和 demo summary。
  - 新增 broker 错误到 API 安全错误响应的映射，避免向客户端暴露 adapter 内部细节。
  - 新增 HTTP 路由测试，覆盖 due 下单、waiting 不下单、tactical delay 不下单，以及非法 UUID / 非法分位 / 非法 order payload 的统一 `bad_request` envelope。
  - 更新 `docs/minimum_mvp.md`，标记 Decision Preview API + MockBroker 串联已完成，并将后续重点调整为阿里云 Qwen API 与真实 Futu/Moomoo OpenD transport。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check -p indexlink-api --locked` 通过。
  - `cargo test -p indexlink-api --locked` 通过。
  - `cargo clippy -p indexlink-api --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 23:47 UTC+10

- 执行模型：GPT-5。
- 变更类型：Decision Engine。
- 涉及文件：
  - `Cargo.toml`
  - `crates/decision-engine/Cargo.toml`
  - `crates/decision-engine/src/lib.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `decision-engine` crate，保持纯函数、零 IO，用于合成 70/20/10 决策。
  - 新增 `DecisionWeights`、`DecisionConfig`、`DecisionInput`、`DecisionSentiment`、`DecisionSignal` 与 `DecisionWeightMode`。
  - 默认 sentiment 可用时使用 `70/20/10`；sentiment 不可用时降级为 `90/10/0`。
  - 将 fundamental、trend 和 sentiment 归一化后合成为 `final_score`、`multiplier` 与 `action`。
  - trend 非中性体制会触发 `TacticalDelay`，避免过热追高或接飞刀。
  - Review fix：`DecisionSignal` 保留原始 `DecisionInput` 快照，便于后续审计、存储和回放。
  - Review fix：sentiment 不可用时在合成公式中使用中性映射值 `0.5`，避免自定义 fallback 权重误把缺失情绪当成极度悲观。
  - Review fix：极端低分会映射到 `Multiplier::MIN`，使 `Action::Skip` 在 Decision Engine 中可达。
  - Review fix：将 multiplier 映射改为连续函数，并复用 `Multiplier::SKIP_BELOW` 的语义，避免 final score 边界附近从 0% 跳到 55%。
  - 新增 Decision Engine 单元测试，覆盖默认权重、非法权重、标准/加码/减量、TacticalDelay 和 AI 降级。
  - 更新 `docs/minimum_mvp.md`，标记 Decision Engine 已完成，并将下一步调整为 Decision Preview API + MockBroker 串联。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p decision-engine --locked` 通过。
  - `cargo clippy -p decision-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 22:48 UTC+10

- 执行模型：GPT-5。
- 变更类型：Futu/Moomoo OpenD paper adapter。
- 涉及文件：
  - `crates/broker/src/lib.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `OpenDOrderGateway`，作为后续 Futu/Moomoo OpenD TCP/SDK transport 的最小提交订单接口。
  - 新增 `OpenDPaperBroker`，实现 `BrokerClient`，用于把已校验订单提交到 OpenD paper gateway。
  - OpenD paper adapter 在调用 gateway 前拒绝 live config 和 live order，保持 paper trading 默认安全边界。
  - 新增 adapter 测试，覆盖正常 paper 提交、live config 拒绝、live order 不穿透 gateway、gateway unavailable 安全上抛。
  - 更新 `docs/minimum_mvp.md`，明确下一步后端顺序：先 Decision Engine，再 Decision Preview API + MockBroker 串联，之后接阿里云 Qwen API。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p broker --locked` 通过。
  - `cargo clippy -p broker --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 22:03 UTC+10

- 执行模型：GPT-5。
- 变更类型：Futu/Moomoo OpenD paper trading 配置底座。
- 涉及文件：
  - `crates/broker/src/lib.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `BrokerProvider`，区分 Futu 与 Moomoo OpenD 目标 provider。
  - 新增 `OpenDConnectionConfig`，校验 OpenD host、port、paper/live 环境和可选 account id。
  - OpenD 配置默认不允许 live trading；live orders 必须同时满足环境匹配和显式 live gate。
  - OpenD 配置的 account id 可供 adapter 使用，但 debug 输出会脱敏，避免进入日志。
  - 新增配置层测试，覆盖 paper 默认值、非法连接字段、account id 脱敏、环境不匹配和 live gate。
  - 更新 `docs/minimum_mvp.md`，明确演示级前端由 Jame 负责；当前后端分支只提供 API 契约、配置、安全边界和测试。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p broker --locked` 通过。
  - `cargo clippy -p broker --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 21:40 UTC+10

- 执行模型：GPT-5。
- 变更类型：Broker paper trading 边界与 Futu/Moomoo MVP 路线。
- 涉及文件：
  - `Cargo.toml`
  - `crates/broker/Cargo.toml`
  - `crates/broker/src/lib.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `broker` crate，定义 provider-neutral `BrokerClient` port，为后续 Futu/Moomoo OpenD adapter 预留边界。
  - 新增 `BrokerEnvironment`，区分 `Paper` 与 `Live`，默认 demo 路径面向虚拟账号 / paper trading。
  - 新增 `BrokerOrderRequest`、`BrokerOrderAck`、`BrokerOrderStatus` 与安全错误类型。
  - 订单请求通过构造器校验 idempotency key、ASCII symbol、正数数量、limit order 价格等不变量。
  - 新增 `MockBroker`，默认只接受 paper orders，拒绝 live orders；用于本地 demo 与后续 decision-to-order 测试。
  - 更新 `docs/minimum_mvp.md`，将 Futu/Moomoo OpenD paper trading、broker ack、live trading 保护开关和演示级前端展示纳入全项目 MVP 路线。
  - Review fix：将 broker crate 的 `missing_docs` 提升为 deny，并明确 MVP 只要求最终 summary，decision record 属于可选存证。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p broker --locked` 通过。
  - `cargo clippy -p broker --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 00:37 UTC+10

- 执行模型：GPT-5。
- 变更类型：20% 趋势层真实实现与全项目 MVP 文档补充。
- 涉及文件：
  - `crates/quant-engine/src/lib.rs`
  - `crates/quant-engine/src/trend/mod.rs`
  - `crates/quant-engine/tests/trend.rs`
  - `crates/quant-engine/tests/trend/direction.rs`
  - `crates/quant-engine/tests/DEFERRED_TESTS.md`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - `evaluate_trend` 从 `NotImplemented` 升级为真实趋势计算：MA200 distance 与 RSI 原始分位反向计入，VIX 原始分位正向计入。
  - 新增趋势体制判定：`FallingKnife` 优先于 `Overheated`，否则为 `Neutral`。
  - Review fix：对趋势合成分数执行 `[0, 1]` clamp，避免权重和浮点容忍导致边界 composite 略超上限时 panic。
  - 保留 `evaluate_trend_or_stub` 和 `evaluate_trend_stub` 作为兼容入口，但默认测试已覆盖真实 trend 行为。
  - 打开既有 trend 行为测试，不再让核心 20% trend TDD 边界保持 ignored。
  - 更新 deferred 测试说明：剩余场景主要阻塞在 Decision Engine，而不是 trend stub。
  - 更新 `docs/minimum_mvp.md`：标记 20% trend 已可复用，并补充演示级最小前端属于全项目 MVP。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo test -p quant-engine --locked` 通过。
  - `cargo clippy -p quant-engine --all-targets --all-features -- -D warnings` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-07 00:09 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划双桶执行预览 API 与 MVP 清单文档。
- 涉及文件：
  - `crates/api/src/routes/investment_plans.rs`
  - `crates/api/tests/investment_plans.rs`
  - `docs/minimum_mvp.md`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `POST /investment-plans/:id/execution-preview`，通过 API DTO 接收预览日期与可选双桶比例。
  - 执行预览 API 复用领域构造器校验 `day_of_month`、双桶比例范围与比例合计，不让 serde 直接构造领域类型。
  - due 且提供双桶配置时返回 `bucket_split`；waiting/inactive 不返回投入金额和双桶拆分。
  - 新增 route tests 覆盖 due 拆分、非执行日省略拆分，以及非法 UUID、非法日期、非法比例和非法 JSON 的统一 bad request。
  - 新增 `docs/minimum_mvp.md`，以全项目视角记录 70/20/10 最小 MVP 主线、已完成能力、20% trend/阿里云接入/decision engine/最终 summary 缺口、演示流程和非 MVP 边界。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-06 23:00 UTC+10

- 执行模型：Claude。
- 变更类型：AI 感知层新闻源接入与全链路管线。
- 涉及文件：
  - `Cargo.toml`
  - `crates/ai-client/Cargo.toml`
  - `crates/ai-client/src/lib.rs`
  - `crates/ai-client/src/news.rs`
  - `crates/ai-client/tests/news.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `NewsSource` trait 与 `RssNewsSource`，对接 CNBC US Top News RSS，拉取最近 24h 英文财经新闻。
  - 新增 `NewsItem`、`NewsSourceError`、`PipelineError` 类型。
  - 新增 `format_sentiment_prompt` 将新闻格式化为英文 AI prompt。
  - 新增 `fetch_market_sentiment` 一站式函数：拉取 → 格式化 → AI 分析 → sentiment。
  - `RssNewsSource` 支持 CDATA 解析、HTML 标签穿透、时间过滤（24h）、数量上限（10 条）、句末截断。
  - `lib.rs` 公开导出 news 模块所有类型与函数。
  - 新增 22 个单测覆盖解析/过滤/格式化/pipeline。
  - 新增 2 个 `#[ignore]` 集成测试：`real_cnbc_with_mock`（仅需网络）与 `real_cnbc_with_qwen`（需网络 + `DASHSCOPE_API_KEY`）。
- 待完成：
  - 申请 DashScope API Key，设 `DASHSCOPE_API_KEY` 环境变量，运行 `real_cnbc_with_qwen` 验证真实 Qwen 输出。
- 验证：
  - `cargo test -p ai-client --locked` 通过：106 个测试（77 单测 + 18 集成测试 + 4 doc test + 7 集成测试），含 2 个 ignored。
  - `cargo clippy -p ai-client --all-targets --all-features -- -D warnings` 通过。
  - `cargo fmt -p ai-client --check` 通过。
  - 手动跑过 `real_cnbc_with_mock`，验证 10 条真实 CNBC 新闻正常拉取、描述完整、prompt 格式正确。

### 2026-07-06 23:45 UTC+10

- 执行模型：Claude。
- 变更类型：fix（AI 感知层 code review 修复）。
- 涉及文件：
  - `crates/ai-client/src/news.rs`
  - `crates/ai-client/tests/news.rs`
- 变更内容：
  - `RssNewsSource::new` / `with_config` 改用 `reqwest::Client::builder().timeout(DEFAULT_HTTP_TIMEOUT)`（30 秒），避免 HTTP 请求无超时挂起。
  - `parse_items` 将逐片段 `trim()` 改为条目解析完成后统一起 trim，修复内联 HTML 标签导致词间空格丢失（如 `as<b>investors</b> cheered` → `asinvestors cheered`）。
  - `filter_and_convert` 在 `truncate` 前先 `sort_by_key(|item| Reverse(item.pub_date))`，确保保留最新 N 条，满足 trait 文档「按时间降序」契约。
  - `truncate_at_sentence` 改用 `char_indices().nth(max_chars)` 定位字符边界，修复 `floor_char_boundary` 对多字节字符（中文）的字节/字符语义不一致。
  - 集成测试 `real_cnbc_with_mock` / `real_cnbc_with_qwen` 将 `fetch_market_sentiment` 从重复两次调用改为一次调用复用结果，避免重复网络/API 计费。
- 验证：
  - `cargo test -p ai-client --locked` 22 个 news 单测通过。
  - `cargo clippy -p ai-client --all-targets --all-features -- -D warnings` 通过。
  - 手动跑过 `real_cnbc_with_mock`，10 条真实 CNBC 新闻正常拉取，管道全链路通过。

### 2026-07-06 20:55 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划执行预览接入双桶拆分。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - `InvestmentPlanExecutionPreview` 新增可选 `bucket_split`，仅在 due 且调用方提供双桶配置时返回。
  - 新增 `InvestmentPlanService::preview_execution_with_buckets`，复用现有执行日判断并附带 core/opportunity 拆分。
  - 保留原 `preview_execution` 行为，默认不返回双桶拆分，避免影响现有调用方。
  - 新增测试覆盖 due 拆分、非 due 不拆分和 JSON 字符串金额契约。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-05 22:29 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划双桶投入拆分领域模型。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `TwoBucketContributionSplit`，按已校验的双桶比例拆分本次计划投入金额。
  - 拆分结果通过构造器生成，保证 core + opportunity 等于原始计划投入金额。
  - 新增 `contribution_for`，按 `InvestmentBucket` 读取对应投入金额。
  - 新增测试覆盖总额守恒、按桶读取、非正金额拒绝和金额 JSON 字符串序列化。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-02 00:24 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划双桶配置领域模型。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `InvestmentBucket`、`BucketAllocationRatio` 与 `TwoBucketAllocationConfig`，先定义双桶配置边界，不接执行分配算法。
  - `BucketAllocationRatio` 通过构造器保证比例位于 0..=1，避免公开字段绕过不变量。
  - `TwoBucketAllocationConfig` 要求常规定投桶和机会桶比例合计为 1。
  - 新增测试覆盖比例边界、比例求和和 JSON 字符串序列化契约。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

### 2026-07-01 00:19 UTC+10

- 执行模型：GPT-5。
- 变更类型：投资计划执行预览领域骨架。
- 涉及文件：
  - `crates/investment-plans/src/lib.rs`
  - `CHANGE_LOG.md`
- 变更内容：
  - 新增 `PreviewInvestmentPlanExecution`、`ExecutionPreviewStatus` 与 `InvestmentPlanExecutionPreview`，用于表达计划在指定月内日期的轻量执行预览。
  - `InvestmentPlanService::preview_execution` 复用 repository get，区分 `due`、`waiting`、`inactive`，并仅在 due 时返回不超过单次执行上限的计划投入金额。
  - 明确该预览不生成 broker order、不处理成交状态，也不包含双桶资金分配。
  - 新增测试覆盖 due、waiting、inactive 与非法预览日期。
- 验证：
  - `cargo fmt --all -- --check` 通过。
  - `cargo check --workspace --locked` 通过。
  - `cargo test --workspace --locked` 通过。
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` 通过。

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
