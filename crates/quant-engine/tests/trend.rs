// 覆盖 Quant Engine 第二层（20% 趋势）当前中性存根的契约。

mod common;

use common::NEUTRAL_PERCENTILE;
use quant_engine::evaluate_trend_stub;

#[test]
fn trend_stub_returns_neutral_score() {
    // 第二层（20% 趋势）当前为存根，按 readme 约定应始终返回中性 0.5，
    // 使 Decision Engine 在趋势层接入前不会产生任何方向性偏移。
    let signal = evaluate_trend_stub();
    assert_eq!(
        signal.score.value(),
        NEUTRAL_PERCENTILE,
        "趋势存根应返回中性 0.50，实际 {}",
        signal.score
    );
}
