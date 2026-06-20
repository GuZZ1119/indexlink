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
    assert!(matches!(
        err,
        QuantError::InvalidCurrentValue {
            indicator: "ERP",
            ..
        }
    ));
}

#[test]
fn zero_min_len_returns_error_before_empty_history_division() {
    // min_len = 0 曾会让空历史通过长度检查，随后 0/0 产生 NaN 并 panic。
    let h = Vec::new();
    let err = percentile_of("CAPE", &h, 10.0, 0).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidMinHistoryLen { value: 0 }),
        "min_len = 0 应在分位计算前被拒绝，实际 {:?}",
        err
    );
}

#[test]
fn nan_in_history_is_filtered() {
    let mut h: Vec<f64> = make_history(90);
    h.extend(std::iter::repeat_n(f64::NAN, TEST_MIN_HISTORY_LEN));
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
fn positive_infinity_current_returns_error() {
    // 金融读数必须是有限数；+Inf 更像上游数据管道异常，而不是合法极端估值。
    let h = standard_history();
    let err = percentile_of("T", &h, f64::INFINITY, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidCurrentValue { indicator: "T", .. }),
        "+Inf 当前读数应返回 InvalidCurrentValue，实际 {:?}",
        err
    );
}

#[test]
fn negative_infinity_current_returns_error() {
    // 金融读数必须是有限数；-Inf 同样应被视为非法输入。
    let h = standard_history();
    let err = percentile_of("T", &h, f64::NEG_INFINITY, TEST_MIN_HISTORY_LEN).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidCurrentValue { indicator: "T", .. }),
        "-Inf 当前读数应返回 InvalidCurrentValue，实际 {:?}",
        err
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

    let invalid = QuantError::InvalidCurrentValue {
        indicator: "ERP",
        value: f64::INFINITY,
    };
    assert_eq!(
        invalid.to_string(),
        "ERP current value must be finite, got inf"
    );

    let invalid_min_len = QuantError::InvalidMinHistoryLen { value: 0 };
    assert_eq!(
        invalid_min_len.to_string(),
        "min_history_len must be greater than 0, got 0"
    );

    let invalid_weight = QuantError::InvalidWeight { value: 1.5 };
    assert_eq!(
        invalid_weight.to_string(),
        "weight must be finite and in [0.0, 1.0], got 1.5"
    );
}
