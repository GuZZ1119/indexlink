// 覆盖 Quant Engine 第一层（70% 基本面）的 CAPE/ERP 分位合成、审计字段与边界行为。

mod common;

use common::{
    balanced_test_config, make_history, standard_history, test_config, BALANCED_CAPE_WEIGHT,
    CAPE_ONLY_WEIGHT, CHEAP_SCORE_UPPER_BOUND, DEFAULT_MIN_HISTORY_LEN, ERP_ONLY_WEIGHT,
    EXACT_FLOAT_TOLERANCE, EXPENSIVE_SCORE_LOWER_BOUND, MAX_PERCENTILE, MIN_PERCENTILE,
    NEUTRAL_PERCENTILE, NEUTRAL_TOLERANCE, TEST_MIN_HISTORY_LEN, VERY_LOW_PERCENTILE_UPPER_BOUND,
};
use quant_engine::{evaluate_fundamental, FundamentalConfig, MarketSnapshot, QuantError};

#[test]
fn fundamental_expensive_market() {
    let cape_h: Vec<f64> = (1..=100).map(|i| i as f64 * 10.0).collect();
    let erp_h: Vec<f64> = (1..=100).map(|i| i as f64 * 0.1).collect();
    let cfg = balanced_test_config();

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 900.0,
        erp_history: erp_h,
        erp_current: 1.0,
    };

    let sig = evaluate_fundamental(&snapshot, &cfg).unwrap();
    assert!(
        sig.score.value() > EXPENSIVE_SCORE_LOWER_BOUND,
        "贵市场得分应 > {EXPENSIVE_SCORE_LOWER_BOUND:.2}，实际 {}",
        sig.score
    );
}

#[test]
fn fundamental_cheap_market() {
    let cape_h: Vec<f64> = (1..=100).map(|i| i as f64 * 10.0).collect();
    let erp_h: Vec<f64> = (1..=100).map(|i| i as f64 * 0.1).collect();
    let cfg = balanced_test_config();

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 100.0,
        erp_history: erp_h,
        erp_current: 9.0,
    };

    let sig = evaluate_fundamental(&snapshot, &cfg).unwrap();
    assert!(
        sig.score.value() < CHEAP_SCORE_UPPER_BOUND,
        "便宜市场得分应 < {CHEAP_SCORE_UPPER_BOUND:.2}，实际 {}",
        sig.score
    );
}

#[test]
fn fundamental_neutral_market() {
    let cape_h = standard_history();
    let erp_h = standard_history();
    let cfg = FundamentalConfig::default();

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 50.0,
        erp_history: erp_h,
        erp_current: 50.0,
    };

    let sig = evaluate_fundamental(&snapshot, &cfg).unwrap();
    assert!(
        (sig.score.value() - NEUTRAL_PERCENTILE).abs() < NEUTRAL_TOLERANCE,
        "中性市场得分应 ≈ 0.50，实际 {}",
        sig.score
    );
}

#[test]
fn rate_repricing_low_erp_pushes_score_expensive_despite_neutral_cape() {
    // 金融场景：利率重估。CAPE 处于历史中性（≈0.50），但利率抬升压缩了风险补偿，
    // 使 ERP 跌至历史极低位。预期：尽管估值锚中性，整体仍应偏贵
    // 因为低 ERP 倒置后变高，把综合得分推向"贵"。这验证 ERP 倒置语义在
    // 两个维度"背离"时仍正确生效（既有方向性测试均为两维同向，未覆盖此处）。
    let cape_h = standard_history();
    let erp_h = standard_history();
    let cfg = balanced_test_config();

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 50.0, // CAPE 分位 = 0.50（中性）
        erp_history: erp_h,
        erp_current: 5.0, // ERP 分位 = 0.05（历史极低，风险补偿被压缩）
    };

    let sig = evaluate_fundamental(&snapshot, &cfg).unwrap();

    // CAPE 维度确实中性。
    assert!(
        (sig.cape_percentile.value() - NEUTRAL_PERCENTILE).abs() < EXACT_FLOAT_TOLERANCE,
        "CAPE 应处于历史中性，实际 {}",
        sig.cape_percentile
    );
    // ERP 原始分位极低（审计字段未倒置）。
    assert!(
        sig.erp_percentile.value() < VERY_LOW_PERCENTILE_UPPER_BOUND,
        "低 ERP 的原始分位应处于历史极低位，实际 {}",
        sig.erp_percentile
    );
    // 关键断言：低 ERP 单独把综合得分推向偏贵，超过中性的 CAPE。
    assert!(
        sig.score.value() > sig.cape_percentile.value(),
        "低 ERP 应把综合得分推得比中性 CAPE 更贵：score {} 应 > cape_p {}",
        sig.score,
        sig.cape_percentile
    );
    assert!(
        sig.score.value() > NEUTRAL_PERCENTILE,
        "利率重估（低 ERP）下整体应偏贵（> {NEUTRAL_PERCENTILE:.2}），实际 {}",
        sig.score
    );
}

#[test]
fn default_config_matches_readme_contract() {
    // 默认配置应为：CAPE/ERP 各半，5 年（60）月度历史下限。
    let cfg = FundamentalConfig::default();
    assert_eq!(cfg.cape_weight, BALANCED_CAPE_WEIGHT);
    assert_eq!(cfg.min_history_len, DEFAULT_MIN_HISTORY_LEN);
}

#[test]
fn erp_percentile_audit_field_is_not_inverted() {
    // FundamentalSignal.erp_percentile 文档声明为"未倒置"（高 ERP = 高分位）。
    // 审计原则：存原始输入分位，倒置只发生在合成 score 内部。
    let cape_h = standard_history();
    let erp_h = standard_history();
    let cfg = balanced_test_config();

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 10.0,
        erp_history: erp_h,
        erp_current: 90.0, // 高 ERP
    };

    let sig = evaluate_fundamental(&snapshot, &cfg).unwrap();
    assert!(
        sig.erp_percentile.value() > EXPENSIVE_SCORE_LOWER_BOUND,
        "高 ERP 的原始分位应处于高位（未倒置），实际 {}",
        sig.erp_percentile
    );
    // 但因为 ERP 越高市场越便宜，倒置后综合得分应偏低。
    assert!(
        sig.score.value() < NEUTRAL_PERCENTILE,
        "高 ERP 应使综合得分偏便宜（< {NEUTRAL_PERCENTILE:.2}），实际 {}",
        sig.score
    );
}

#[test]
fn audit_fields_reflect_raw_percentiles() {
    // 审计字段须如实记录两个维度各自的原始分位，便于回放"为何那天的得分"。
    let cape_h = standard_history();
    let erp_h = standard_history();
    let cfg = balanced_test_config();

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 30.0,
        erp_history: erp_h,
        erp_current: 70.0,
    };

    let sig = evaluate_fundamental(&snapshot, &cfg).unwrap();
    assert_eq!(sig.cape_percentile.value(), 0.30);
    assert_eq!(sig.erp_percentile.value(), 0.70);
}

#[test]
fn cape_weight_full_uses_only_cape() {
    // cape_weight = 1.0：得分应完全等于 CAPE 分位，忽略 ERP。
    let cape_h = standard_history();
    let erp_h = standard_history();
    let cfg = test_config(CAPE_ONLY_WEIGHT);

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 80.0,
        erp_history: erp_h,
        erp_current: 10.0, // 任意值都不应影响结果
    };

    let sig = evaluate_fundamental(&snapshot, &cfg).unwrap();
    assert_eq!(
        sig.score.value(),
        sig.cape_percentile.value(),
        "cape_weight=1.0 时得分应完全等于 CAPE 分位"
    );
}

#[test]
fn cape_weight_zero_uses_only_inverted_erp() {
    // cape_weight = 0.0：得分应完全等于倒置后的 ERP 分位。
    let cape_h = standard_history();
    let erp_h = standard_history();
    let cfg = test_config(ERP_ONLY_WEIGHT);

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 80.0, // 任意值都不应影响结果
        erp_history: erp_h,
        erp_current: 70.0,
    };

    let sig = evaluate_fundamental(&snapshot, &cfg).unwrap();
    let inverted_erp = MAX_PERCENTILE - sig.erp_percentile.value();
    assert!(
        (sig.score.value() - inverted_erp).abs() < EXACT_FLOAT_TOLERANCE,
        "cape_weight=0.0 时得分应等于倒置 ERP 分位 {}，实际 {}",
        inverted_erp,
        sig.score
    );
}

#[test]
fn propagates_insufficient_history_for_circuit_breaker() {
    // 数据不足时应向上传播错误，供熔断/降级链触发默认 Skip。
    let cfg = FundamentalConfig::default(); // min_history_len = 60
    let snapshot = MarketSnapshot {
        cape_history: vec![1.0, 2.0, 3.0],
        cape_current: 2.0,
        erp_history: standard_history(),
        erp_current: 50.0,
    };

    let err = evaluate_fundamental(&snapshot, &cfg).unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InsufficientHistory {
                indicator: "CAPE",
                ..
            }
        ),
        "CAPE 历史不足应传播 InsufficientHistory，实际 {:?}",
        err
    );
}

#[test]
fn propagates_invalid_input_on_nan_current() {
    // 当前读数为 NaN 时应传播 InvalidInput，绝不静默产出得分。
    let cfg = balanced_test_config();
    let snapshot = MarketSnapshot {
        cape_history: standard_history(),
        cape_current: 50.0,
        erp_history: standard_history(),
        erp_current: f64::NAN,
    };

    let err = evaluate_fundamental(&snapshot, &cfg).unwrap_err();
    assert!(
        matches!(err, QuantError::InvalidInput(_)),
        "ERP 当前值为 NaN 应传播 InvalidInput，实际 {:?}",
        err
    );
}

// ─── 边界：±Inf 当前读数（当前为未定义行为） ─────────────────────────────────

#[test]
fn infinite_current_is_accepted_as_extreme_percentile() {
    // 边界（当前为未定义行为）：evaluate_fundamental 经 percentile_of 仅拦截 NaN，
    // 不拦截 ±Inf。cape_current = +Inf → CAPE 分位 1.0（历史最贵）；
    // erp_current = -Inf → ERP 分位 0.0，倒置后 1.0（历史最贵），综合得分应为 1.0。
    // 若后续实现层加入有限性校验，应改为断言返回 QuantError::InvalidInput。
    let cape_h = standard_history();
    let erp_h = standard_history();
    let cfg = balanced_test_config();

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: f64::INFINITY,
        erp_history: erp_h,
        erp_current: f64::NEG_INFINITY,
    };

    let sig = evaluate_fundamental(&snapshot, &cfg).unwrap();
    assert_eq!(
        sig.cape_percentile.value(),
        MAX_PERCENTILE,
        "+Inf CAPE 当前被当作历史最高位"
    );
    assert_eq!(
        sig.erp_percentile.value(),
        MIN_PERCENTILE,
        "-Inf ERP 当前被当作历史最低位"
    );
    assert_eq!(
        sig.score.value(),
        MAX_PERCENTILE,
        "两端极值应合成出历史最贵得分 1.0"
    );
}

// ─── 边界：cape_weight 越界（当前为未定义行为，可能 panic） ──────────────────

#[test]
#[should_panic(expected = "加权平均结果必然在 [0.0, 1.0]")]
fn cape_weight_above_one_panics_on_out_of_range_score() {
    // 边界（当前为未定义行为）：实现未校验 cape_weight ∈ [0,1]。
    // cape_weight = 2.0 → composite = 2.0*cape_p + (1-2.0)*inverted_erp。
    // 取 cape_p = 1.0、inverted_erp = 0.0 → composite = 2.0，越界触发
    // Percentile::new(...).expect(...) panic。
    // 若后续在 FundamentalConfig 构造处加入校验，应改为构造期返回错误并移除此 should_panic。
    let cape_h = standard_history();
    let erp_h = standard_history();
    let cfg = FundamentalConfig {
        cape_weight: 2.0,
        min_history_len: TEST_MIN_HISTORY_LEN,
    };

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 100.0, // CAPE 分位 = 1.0
        erp_history: erp_h,
        erp_current: 100.0, // ERP 分位 = 1.0 → 倒置 = 0.0
    };

    let _ = evaluate_fundamental(&snapshot, &cfg);
}

#[test]
#[should_panic(expected = "加权平均结果必然在 [0.0, 1.0]")]
fn cape_weight_below_zero_panics_on_out_of_range_score() {
    // 边界（当前为未定义行为）：负权重同样未被校验。
    // cape_weight = -1.0 → composite = -1.0*cape_p + 2.0*inverted_erp。
    // 取 cape_p = 1.0、inverted_erp = 0.0 → composite = -1.0，越界触发 panic。
    let cape_h = standard_history();
    let erp_h = standard_history();
    let cfg = FundamentalConfig {
        cape_weight: -1.0,
        min_history_len: TEST_MIN_HISTORY_LEN,
    };

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 100.0, // CAPE 分位 = 1.0
        erp_history: erp_h,
        erp_current: 100.0, // ERP 分位 = 1.0 → 倒置 = 0.0
    };

    let _ = evaluate_fundamental(&snapshot, &cfg);
}

// ─── 边界：两个历史序列长度不一致（当前为未定义行为） ────────────────────────

#[test]
fn unequal_history_lengths_are_accepted() {
    // 边界（当前为未定义行为）：MarketSnapshot 不要求两个历史序列等长。
    // 各指标分位独立计算，只要各自有效点数 ≥ min_history_len 即成功。
    // 若后续实现层加入等长校验，应改为断言返回错误。
    let cape_h = standard_history(); // 100 点
    let erp_h = make_history(DEFAULT_MIN_HISTORY_LEN); // 60 点
    let cfg = balanced_test_config();

    let snapshot = MarketSnapshot {
        cape_history: cape_h,
        cape_current: 50.0, // 50/100 = 0.50
        erp_history: erp_h,
        erp_current: 30.0, // 30/60 = 0.50
    };

    let sig = evaluate_fundamental(&snapshot, &cfg).unwrap();
    assert_eq!(
        sig.cape_percentile.value(),
        NEUTRAL_PERCENTILE,
        "CAPE 应按自身 100 点序列定位"
    );
    assert_eq!(
        sig.erp_percentile.value(),
        NEUTRAL_PERCENTILE,
        "ERP 应按自身 60 点序列定位"
    );
}

#[test]
fn unequal_lengths_below_min_propagate_insufficient_for_short_series() {
    // 边界（当前为未定义行为）：不等长且较短序列 < min_history_len 时，
    // 错误应明确指向较短的那个指标（此处为 ERP），而非静默产出得分。
    let cfg = FundamentalConfig {
        cape_weight: BALANCED_CAPE_WEIGHT,
        min_history_len: DEFAULT_MIN_HISTORY_LEN,
    };

    let snapshot = MarketSnapshot {
        cape_history: standard_history(), // 充足
        cape_current: 50.0,
        erp_history: make_history(30), // 不足 60
        erp_current: 15.0,
    };

    let err = evaluate_fundamental(&snapshot, &cfg).unwrap_err();
    assert!(
        matches!(
            err,
            QuantError::InsufficientHistory {
                indicator: "ERP",
                found: 30,
                ..
            }
        ),
        "较短的 ERP 序列应触发 InsufficientHistory{{found:30}}，实际 {:?}",
        err
    );
}
