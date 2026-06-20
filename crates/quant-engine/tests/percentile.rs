// 覆盖跨层共享的历史分位计算工具，包括边界、脏数据过滤与错误上下文。

mod common;

use common::{
    make_history, standard_history, DEFAULT_MIN_HISTORY_LEN, MAX_PERCENTILE, MIN_PERCENTILE,
    NEUTRAL_PERCENTILE, STANDARD_HISTORY_LEN, TEST_MIN_HISTORY_LEN,
};
use quant_engine::{percentile_of, QuantError};

#[test]
fn percentile_of_median() {
    let h = standard_history();
    let p = percentile_of("T", &h, 50.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert!(
        (p.value() - NEUTRAL_PERCENTILE).abs() < 0.01,
        "中位数应约为 50%，实际 {}",
        p
    );
}

#[test]
fn percentile_of_at_minimum() {
    let h = standard_history();
    let p = percentile_of("T", &h, 1.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(p.value(), 0.01, "只有 1 个值 ≤ 1，分位应为 1%");
}

#[test]
fn percentile_of_at_maximum() {
    let h = standard_history();
    let p = percentile_of("T", &h, 100.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(p.value(), MAX_PERCENTILE, "所有值 ≤ 100，分位应为 100%");
}

#[test]
fn percentile_of_below_all_history() {
    let h = standard_history();
    let p = percentile_of("T", &h, 0.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(p.value(), MIN_PERCENTILE, "无值 ≤ 0，分位应为 0%");
}

#[test]
fn percentile_of_above_all_history() {
    let h = standard_history();
    let p = percentile_of("T", &h, 9999.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(p.value(), MAX_PERCENTILE, "所有值 ≤ 9999，分位应为 100%");
}

#[test]
fn insufficient_history_returns_error() {
    let h = vec![1.0, 2.0, 3.0];
    let err = percentile_of("CAPE", &h, 2.0, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(
        matches!(err, QuantError::InsufficientHistory { found: 3, .. }),
        "应返回 InsufficientHistory，实际 {:?}",
        err
    );
}

#[test]
fn nan_current_returns_error() {
    let h = standard_history();
    let err = percentile_of("ERP", &h, f64::NAN, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(matches!(err, QuantError::InvalidInput(_)));
}

#[test]
fn nan_in_history_is_filtered() {
    let mut h: Vec<f64> = make_history(90);
    h.extend(std::iter::repeat(f64::NAN).take(TEST_MIN_HISTORY_LEN));
    let p = percentile_of("T", &h, 45.0, DEFAULT_MIN_HISTORY_LEN).unwrap();
    assert!(p.value() > MIN_PERCENTILE);
}

#[test]
fn duplicate_values_use_less_than_or_equal() {
    // 50 个 1.0 + 50 个 2.0；current = 1.0 时 "<=" 应计入全部并列值。
    let mut h = vec![1.0_f64; 50];
    h.extend(vec![2.0_f64; 50]);
    let p = percentile_of("T", &h, 1.0, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(
        p.value(),
        NEUTRAL_PERCENTILE,
        "50 个 ≤ 1.0 的并列值应得 50% 分位，实际 {}",
        p
    );
}

#[test]
fn all_nan_history_returns_insufficient() {
    // 全部为 NaN，过滤后有效数据为 0，应判定历史不足而非 panic。
    let h = vec![f64::NAN; STANDARD_HISTORY_LEN];
    let err = percentile_of("CAPE", &h, 10.0, DEFAULT_MIN_HISTORY_LEN).unwrap_err();
    assert!(
        matches!(err, QuantError::InsufficientHistory { found: 0, .. }),
        "全 NaN 历史应返回 InsufficientHistory{{found:0}}，实际 {:?}",
        err
    );
}

#[test]
fn exact_min_len_boundary_succeeds() {
    // 有效数据点数恰好等于 min_len 时应成功（边界包含）。
    let h = make_history(DEFAULT_MIN_HISTORY_LEN);
    let p = percentile_of("T", &h, 30.0, DEFAULT_MIN_HISTORY_LEN);
    assert!(p.is_ok(), "有效点数 == min_len 应成功，实际 {:?}", p);
}

#[test]
fn insufficient_history_reports_indicator_and_required() {
    // 校验错误携带完整上下文，供审计/降级链使用。
    let h = vec![1.0, 2.0, 3.0];
    let err = percentile_of("CAPE", &h, 2.0, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert_eq!(
        err,
        QuantError::InsufficientHistory {
            indicator: "CAPE",
            required: TEST_MIN_HISTORY_LEN,
            found: 3,
        }
    );
}

#[test]
fn positive_infinity_current_is_treated_as_max() {
    // 边界（当前为未定义行为）：实现只拦截 NaN，+Inf 被当作合法值。
    // +Inf ≥ 任意有限历史值 → 全部 <= current → 分位 = 1.0。
    // 若后续实现层加入 is_finite 校验，应改为断言返回 QuantError::InvalidInput。
    let h = standard_history();
    let p = percentile_of("T", &h, f64::INFINITY, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(
        p.value(),
        MAX_PERCENTILE,
        "+Inf 当前被当作历史最高位，实际 {}",
        p
    );
}

#[test]
fn negative_infinity_current_is_treated_as_min() {
    // 边界（当前为未定义行为）：-Inf 同样未被拦截。
    // 没有任何有限历史值 <= -Inf → count_le = 0 → 分位 = 0.0。
    // 若后续实现层加入 is_finite 校验，应改为断言返回 QuantError::InvalidInput。
    let h = standard_history();
    let p = percentile_of("T", &h, f64::NEG_INFINITY, TEST_MIN_HISTORY_LEN).unwrap();
    assert_eq!(
        p.value(),
        MIN_PERCENTILE,
        "-Inf 当前被当作历史最低位，实际 {}",
        p
    );
}

#[test]
fn quant_error_display_is_descriptive() {
    let insufficient = QuantError::InsufficientHistory {
        indicator: "CAPE",
        required: DEFAULT_MIN_HISTORY_LEN,
        found: 3,
    };
    assert_eq!(
        insufficient.to_string(),
        "CAPE: requires at least 60 historical data points, found 3"
    );

    let invalid = QuantError::InvalidInput("ERP current value is NaN".to_string());
    assert_eq!(
        invalid.to_string(),
        "invalid input: ERP current value is NaN"
    );
}
