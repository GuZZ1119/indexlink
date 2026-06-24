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
    /// 调用方应使用 `.unwrap_or(Sentiment::neutral())` 实现安全降级。
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
            .field("base_url", &self.base_url)
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("timeout", &self.timeout)
            .field("max_tokens", &self.max_tokens)
            .field("temperature", &self.temperature)
            .finish()
    }
}

impl fmt::Display for AiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AiConfig(model={}, base_url={})", self.model, self.base_url)
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
    fn config_display_hides_api_key() {
        let config = AiConfig {
            api_key: "sk-secret-key-12345".to_owned(),
            ..Default::default()
        };
        let display = format!("{config}");
        assert!(display.contains("qwen-plus"));
        assert!(display.contains("dashscope"));
        assert!(!display.contains("sk-secret-key-12345"));
    }

    #[test]
    fn config_debug_does_not_leak_base_url_with_embedded_secret() {
        let config = AiConfig {
            base_url: "https://user:password@evil.example.com/v1".to_owned(),
            api_key: "sk-abc".to_owned(),
            ..Default::default()
        };
        let debug = format!("{config:?}");
        // base_url is shown in debug (not secret — Debug is for devs)
        assert!(debug.contains("evil.example.com"));
        // but api_key must still be redacted
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("sk-abc"));
    }
}
