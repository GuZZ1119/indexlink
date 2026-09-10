# IndexLink Web Plan / 前端计划

## V2.1 当前主路径 / Current V2.1 path

Web 的默认入口现为本地优先的消费级外壳：个人中心、策略中心与高级实验室。它先让普通用户理解并采用长期策略；现有 Rust API 页面、审计与 paper-only 能力保留为旧路径和后续接入基础，而不再占据主导航。

The default entry is now a local-first consumer shell: Personal, Strategy Center, and Advanced Lab. It helps ordinary users understand and adopt long-term strategies first. Existing Rust API pages, audits, and paper-only capabilities remain available as legacy routes and future integration foundations rather than the primary navigation.

### 信息架构 / Information architecture

| 页面 / Page | V2.1 用户任务 / V2.1 user task | 当前数据边界 / Data boundary |
| --- | --- | --- |
| 个人中心 / Personal | 查看正在坚持的策略、下一次行动与近期变化 | 本地演示状态；不伪装为已连接收益或订单数据 |
| 策略中心 / Strategy Center | 理解、选用、比较固定定投与 70/20/10 等精选策略 | 路由为 `/strategy-center`，避免与 Rust `/strategies` API 前缀冲突；含 `/strategy-analysis` 二级导航，按统一起点 100 比较多个本地示例策略；公开分享/fork 与真实版本化回测等待后续契约 |
| 高级实验室 / Advanced Lab | 了解 Docker、Moomoo/OpenD、Qwen、市场数据等可选能力 | 只显示配置入口与安全边界；不保存密钥、不验证账户、不下单 |

## 页面与契约 / Pages and contracts

| 页面 / Page | 已实现 / Implemented | 主要 API / Main API |
| --- | --- | --- |
| 仪表盘 / Dashboard | 自动市场输入、Qwen 情绪、Decision Preview、双桶结果、模拟账户、收益与回放图 | `/signals/*`, `/market-sentiment/preview`, `/investment-plans/:id/*`, `/paper-*` |
| 定投标的 / Holdings | V1.1 周期、多个执行日、桶比例、风险模式、滚存、策略版本创建与编辑 | `/investment-plans` |
| 决策 / Decisions | 跨标的记录、计划/动作/日期筛选、分页、审计详情与审批模式 paper order 确认 | `/decisions`, `/investment-plans/:id/decisions` |
| 策略 Studio / Strategy Studio | 受限 DSL、验证、准入回测、版本激活 | `/strategies`, `/investment-plans/:id/activate-policy` |

## 运行可观测性 / Runtime observability

顶栏读取 `/health`、`/ready` 与 `/runtime-status`，清楚区分 API 离线、SQLite 未就绪、OpenD/Qwen/市场数据未配置，以及 scheduler 最近一次安全计数。状态展示绝不调用 Qwen 或提交订单。

The top status strip reads `/health`, `/ready` and `/runtime-status`. It distinguishes an offline API, unavailable SQLite, optional OpenD/Qwen/market-data configuration, and safe scheduler counters without invoking Qwen or placing an order.

## 前端约束 / Frontend rules

- React Router 路由页面按需加载，并设置可恢复的 `errorElement`；不得向用户展示框架默认异常页。
- 中英文翻译键必须完全对齐；Vitest 会验证两套 locale 的键集合与非空值。
- 服务端数据必须通过 React Query；手动刷新使用 `refetch`，仍写入同一 query cache。
- 不开放自由策略代码编辑器；策略 Studio 只提交后端白名单 DSL。
- 所有交易交互保持 paper-only；审批模式必须确认已有决策存证，不能重新计算后下单。

## 验证 / Verification

```bash
pnpm --dir apps/web lint
pnpm --dir apps/web test:coverage
pnpm --dir apps/web build
```

## 后续 / Next

1. 以版本化策略、数据集、费用与假设契约替换当前策略分析页的本地示例序列。
2. 实现本地计划采用、手动完成/跳过/调整记录与可复核复盘。
3. 在保持用户确认的前提下，先接入一个只读券商账户，再评估预填订单或一键确认执行。
