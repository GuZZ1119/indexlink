#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Investment Plan 领域与应用层基础。
//!
//! 本 crate 采用模块化单体内的轻量六边形边界：这里定义投资计划的领域模型、
//! 输入校验、应用服务和 repository port；PostgreSQL、Axum、Broker、Qwen、
//! Scheduler 与执行计划生成均属于外部 adapter 或后续阶段。
//!
//! MVP 假设：单用户系统、仅支持 monthly、无计划级 timezone、不验证 symbol 是否
//! 真实可交易、不计算任何本期买入金额或双桶资金分配。
//!
//! 金额统一使用 [`rust_decimal::Decimal`]。HTTP/JSON 边界必须以字符串编码金额，
//! 避免 JavaScript Number 或 JSON 浮点转换造成精度损失。

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};

    #[derive(Debug, Deserialize, Serialize)]
    struct DecimalContract {
        #[serde(with = "rust_decimal::serde::str")]
        amount: Decimal,
    }

    #[test]
    fn decimal_deserializes_from_json_string_without_float() {
        let payload = r#"{"amount":"1000.0001"}"#;

        let decoded: DecimalContract = serde_json::from_str(payload).unwrap();

        assert_eq!(decoded.amount.to_string(), "1000.0001");
    }

    #[test]
    fn decimal_serializes_to_json_string_without_float() {
        let payload = DecimalContract {
            amount: "1500.25".parse().unwrap(),
        };

        let encoded = serde_json::to_value(payload).unwrap();

        assert_eq!(encoded, json!({"amount": "1500.25"}));
        assert!(matches!(encoded["amount"], Value::String(_)));
    }

    #[test]
    fn decimal_rejects_json_number_at_api_boundary() {
        let result = serde_json::from_value::<DecimalContract>(json!({"amount": 1000.00}));

        assert!(result.is_err());
    }
}
