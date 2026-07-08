# Agent 规范

## 项目一句话

> **IndexLink** 是为长期指数投资者设计的自适应定投执行系统：以历史分位锚定估值位置（70%）、趋势节奏（20%）与 AI 语义感知（10%）在定投日微调投入——相对低位加码、相对高位减量、过热延时；只测量价格在历史分布中的位置，不声称判断价值。

## 必须完成的规范

- 始终使用中文回复。
- 修改代码或文档后，在 `CHANGE_LOG.md` 记录时间、执行模型、变更类型、涉及文件、变更内容和验证结果。
- 尊重分层边界
- 新增公开 API 必须补齐文档。
- 带不变量的 newtype（如 `Percentile`、`Multiplier`）必须通过构造函数或 `TryFrom` 保持校验，不能绕过安全边界。
- 为行为变更补充聚焦测试；改动后至少运行 `cargo test -p core-domain`，必要时运行 `cargo llvm-cov -p core-domain --summary-only`。
- 审计/存储相关能力应优先保存输入快照而非只保存结论；后续 `serde` 支持应使用 feature 开关，且反序列化必须复用不变量校验。

## 外部参考

- 仓库: https://github.com/jamesra26/indexlink
- CHANGELOG: `./CHANGE_LOG.md`

### 前端部分

- shadcn: https://ui.shadcn.com/docs/components
- 前端规划：apps\web\PLAN.md
- 使用vite + react + tailwindcss，配合shadcn的组件库进行快速构建。
- 统一使用 pnpm 管理依赖
- 路由统一使用 react-router；来自 Rust API 的服务端数据、缓存、loading/error/retry、mutation 后失效刷新统一使用 @tanstack/react-query；浏览器本地 UI 状态（当前选中的 plan、筛选条件、modal 开关、图表显示范围、临时交互状态）使用 valtio，不要用 valtio 存长期服务端数据。页面样式使用 Tailwind CSS v4 和 @tailwindcss/vite；通用组件优先使用 shadcn 体系，样式组合用 clsx + tailwind-merge，变体组件用 class-variance-authority，动画辅助用 tw-animate-css，图标使用 lucide-react。图表底层使用 recharts，shadcn chart 只作为样式/容器封装。i18n做多语言。代码质量使用 eslint + typescript-eslint + react-hooks / react-refresh 规则，构建脚本保持 tsc -b && vite build。
