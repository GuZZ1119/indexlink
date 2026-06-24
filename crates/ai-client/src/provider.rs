//! AI 服务提供者抽象与配置。
//!
//! [`AiProvider`] 是 LLM 后端的可替换 trait，
//! 遵循与 [`ReadinessCheck`] 相同的适配器模式。
//!
//! [`ReadinessCheck`]: indexlink_api::state::ReadinessCheck

use std::{fmt, time::Duration};

use async_trait::async_trait;

use crate::{AiClientError, Sentiment};

/// LLM 后端的可替换抽象。
///
/// 当前实现：
/// - [`QwenClient`]：兼容 Qwen / OpenAI API。
/// - 测试：`MockAiProvider`（不发起网络请求）。
///
/// [`QwenClient`]: crate::QwenClient
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// 分析新闻/财报文本，返回有界情绪得分。
    ///
    /// # 错误
    ///
    /// 超时、网络错误、API 错误或响应解析失败时返回 [`AiClientError`]。
    /// ai-client 不在此层降级——由上层 decision engine 按 70/20/10 → 90/10/0
    /// 策略处理错误（AI 权重归零，仅用基本面和趋势数据决策）。
    async fn analyze(&self, prompt: &str) -> Result<Sentiment, AiClientError>;
}

/// AI 服务连接配置。
///
/// [`Debug`] 和 [`Display`] 实现**不暴露** `api_key`。
/// 遵循项目安全规范：连接凭证不可出现在日志或错误消息中。
pub struct AiConfig {
    /// API 基础 URL（如 `https://dashscope.aliyuncs.com/compatible-mode`）。
    pub base_url: String,
    /// API 密钥（不在 Debug/Display 中暴露）。
    pub api_key: String,
    /// 模型名称（如 `qwen-plus`、`qwen-max`）。
    pub model: String,
    /// 单次请求超时。
    pub timeout: Duration,
    /// 最大生成 token 数（响应极短，默认 128 足够）。
    pub max_tokens: u32,
    /// 生成温度（建议 0.0~0.3，降低随机性以保持信号稳定）。
    pub temperature: f32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://dashscope.aliyuncs.com/compatible-mode".to_owned(),
            api_key: String::new(),
            model: "qwen-plus".to_owned(),
            timeout: Duration::from_secs(30),
            max_tokens: 128,
            temperature: 0.0,
        }
    }
}

impl fmt::Debug for AiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AiConfig")
            .field("base_url", &redact_url_userinfo(&self.base_url))
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("timeout", &self.timeout)
            .field("max_tokens", &self.max_tokens)
            .field("temperature", &self.temperature)
            .finish()
    }
}

/// 去除 URL 中的 `user:password@` 部分，防止 Debug/Display 输出泄露嵌入的凭据。
fn redact_url_userinfo(url: &str) -> String {
    match url.find('@') {
        Some(at_pos) if url.contains("://") => {
            let scheme_end = url.find("://").unwrap();
            format!("{}<redacted>@{}", &url[..=scheme_end + 2], &url[at_pos + 1..])
        }
        _ => url.to_owned(),
    }
}

impl fmt::Display for AiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AiConfig(model={}, base_url={})",
            self.model,
            redact_url_userinfo(&self.base_url)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_values() {
        let config = AiConfig::default();
        assert!(config.base_url.contains("dashscope"));
        assert_eq!(config.model, "qwen-plus");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_tokens, 128);
        assert_eq!(config.temperature, 0.0);
        assert!(config.api_key.is_empty());
    }

    #[test]
    fn config_debug_redacts_api_key() {
        let config = AiConfig {
            api_key: "sk-secret-key-12345".to_owned(),
            ..Default::default()
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("sk-secret-key-12345"));
        assert!(debug.contains("qwen-plus"));
        assert!(debug.contains("dashscope"));
    }

    #[test]
    fn config_display_hides_api_key_and_url_credentials() {
        let config = AiConfig {
            base_url: "https://user:password@evil.example.com/v1".to_owned(),
            api_key: "sk-secret-key-12345".to_owned(),
            ..Default::default()
        };
        let display = format!("{config}");
        assert!(display.contains("evil.example.com"));
        assert!(!display.contains("user:password"), "URL 凭据不应出现在 Display 中");
        assert!(display.contains("<redacted>"));
        assert!(!display.contains("sk-secret-key-12345"));
    }

    #[test]
    fn config_debug_redacts_embedded_url_credentials() {
        let config = AiConfig {
            base_url: "https://user:password@evil.example.com/v1".to_owned(),
            api_key: "sk-abc".to_owned(),
            ..Default::default()
        };
        let debug = format!("{config:?}");
        // URL 凭据被 redact，但 host 可保留（Debug 是给开发者看的）
        assert!(debug.contains("evil.example.com"));
        assert!(!debug.contains("user:password"), "URL 凭据不应出现在 Debug 中");
        assert!(debug.contains("<redacted>"), "应有 redact 标记");
        // api_key 同样被 redact
        assert!(!debug.contains("sk-abc"));
    }
}
