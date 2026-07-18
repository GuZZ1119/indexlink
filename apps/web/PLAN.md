# 前端规划

## 项目一句话

目前期望进行最小MVP的构建，暂时不需要连接到后端，目的是进行前端页面布局规划、优化，并确认最终形态。

## 布局划分

- header左上角显示logo，右上角显示当前账户信息（暂时mock）和切换语言的标志
- header下方有滚动条显示产品相关的实时新闻
- 左侧sidebar显示页面列表，可收放
- 其余部分做页面显示

## 页面划分

### Dashboard `/`

- 当前产品市场估值、建议动作
- 基本面、趋势面、AI情绪得分、综合得分
- 最近一次决策、决策摘要
- 风险提示

- 总收益
- 持仓收益
- 确定收益
- 等等
- 图表显示相比普通定投的收益比较，普通定投 vs 自适应定投曲线

### Decision Detail `/decisions/:id`

由一个副sidebar和页面显示组成，副sidebar是一个列表，可以进行搜索、排序等，列表里的每一项都具有简单信息摘要：

- 产品代码
- 最终动作：Overweight / Standard / Delay / Underweight / Skip
- 金额
- 倍率
- 时间

点开后，右边可以显示详情：

- 最终动作：Overweight / Standard / Delay / Underweight / Skip
- 倍率：例如 0.75x / 1.30x
- 70/20/10 分解
- CAPE 分位
- ERP 分位
- MA200 距离分位
- RSI 分位
- VIX 分位
- AI 情绪偏移
- 自然语言解释
- 输入快照
- 审计记录

### Plans `/plans/:id`

类似Decision Detail的副sidebar和页面显示形式，展示用户有哪些定投计划。

- 计划名称
- 标的
- 基准金额
- 周期
- 最大倍率
- 是否需要人工确认
- 状态：启用 / 暂停

后续再加 /plans/:id 或编辑弹窗。
