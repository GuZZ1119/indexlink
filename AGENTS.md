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
