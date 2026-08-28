# IndexLink Web Plan / 前端计划

## 当前状态 / Current state

Web 已连接 Rust API，而不是静态演示页。所有长期服务端数据均由 React Query 获取、缓存与失效；Valtio 只保存当前选中标的、图表范围与局部交互状态。

The web client is connected to the Rust API, not a static demo. React Query owns server data, cache and invalidation; Valtio is limited to selected holding, chart range and transient UI state.

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
pnpm --dir apps/web test
pnpm --dir apps/web build
```

## 后续 / Next

1. 在有真实多账户与真实填单数据时，增加账户维度筛选和服务端游标分页。
2. 为 Dashboard 图表单独拆分 Recharts vendor chunk，并按性能测量再调整。
3. 用 Playwright 覆盖浏览器级关键路径：后端离线、Qwen 未配置、审批下单与语言切换。
