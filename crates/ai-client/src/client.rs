//! OpenAI 兼容 API 客户端。
//!
//! [`QwenClient`] 实现 [`AiProvider`] trait，
//! 对接 Qwen DashScope / OpenAI / 任何兼容 `/v1/chat/completions` 的服务。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::{AiClientError, AiConfig, AiProvider, Sentiment};

// ─── System Prompt ───────────────────────────────────────────────────────────

/// 系统提示词：指导 LLM 输出结构化情绪 JSON。
///
/// 设计要点：
/// - 强制 JSON-only 输出，禁止附带解释文本
/// - 保守打分：无明确方向信号时倾向近 0
/// - 分类指引覆盖财报、宏观、政策等主要场景
const SYSTEM_PROMPT: &str = "\
You are a financial sentiment analyzer. Analyze the given financial news and \
output ONLY a JSON object with a single \"sentiment\" field.

Output format (exactly):
{\"sentiment\": <float between -1.0 and +1.0>}

Scoring guide:
- +1.0: Strong bullish signal (major earnings beat, positive macro surprise, \
central bank dovish pivot)
- +0.5: Moderate bullish (minor beat, favorable guidance, sector tailwind)
- +0.1 to +0.3: Slightly positive tone (in-line results with optimistic commentary)
- 0.0: Neutral or mixed signals, no clear directional bias
- -0.1 to -0.3: Slightly negative tone (minor miss, cautious commentary)
- -0.5: Moderate bearish (guidance cut, sector headwinds, trade friction)
- -1.0: Strong bearish signal (major miss, systemic risk event, credit event)

IMPORTANT: Be conservative. Unless there is a clear directional signal, output a \
value close to 0. Do NOT include any text other than the JSON object.";

// ─── Request / Response Types ────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct SentimentResponse {
    sentiment: f64,
}

// ─── QwenClient ────────────────────────────────────────────────────────────

/// OpenAI 兼容 API 客户端（支持 Qwen / OpenAI / 任何兼容服务）。
///
/// # 降级策略
///
/// 任何错误（超时、网络、解析）都返回 [`AiClientError`]。
/// 调用方使用 `.unwrap_or(Sentiment::neutral())` 即可安全降级。
///
/// # 示例
///
/// ```rust,no_run
/// use ai_client::{AiConfig, AiProvider, QwenClient, Sentiment};
///
/// # async fn example() {
/// let config = AiConfig::default();
/// let client = QwenClient::new(config);
/// let sentiment = client
///     .analyze("央行宣布降准50bp，释放长期流动性约1万亿元")
///     .await
///     .unwrap_or_else(|_| Sentiment::neutral());
/// # }
/// ```
pub struct QwenClient {
    http: reqwest::Client,
    config: AiConfig,
}

impl QwenClient {
    /// 使用给定配置创建客户端。
    ///
    /// 若 `api_key` 为空字符串，客户端仍可创建但所有请求将因认证失败而返回错误。
    /// 这遵循「延迟失败」原则——直到实际调用时才报错，便于测试和配置热更新。
    #[must_use]
    pub fn new(config: AiConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("reqwest::Client::builder with standard options must not fail");
        Self { http, config }
    }

    /// 构造请求体。
    fn build_request(&self, prompt: &str) -> ChatRequest {
        ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                Message {
                    role: "system",
                    content: SYSTEM_PROMPT.to_owned(),
                },
                Message {
                    role: "user",
                    content: prompt.to_owned(),
                },
            ],
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        }
    }

    /// 拼接 chat completions 端点 URL。
    fn chat_url(&self) -> String {
        format!("{}/v1/chat/completions", self.config.base_url.trim_end_matches('/'))
    }

    /// 执行 HTTP 请求并解析响应。
    async fn call_api(&self, prompt: &str) -> Result<f64, AiClientError> {
        let url = self.chat_url();
        let body = self.build_request(prompt);

        debug!(url = %url, model = %self.config.model, "sending AI sentiment request");

        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                if err.is_timeout() {
                    warn!(
                        seconds = self.config.timeout.as_secs(),
                        "AI service request timed out"
                    );
                    AiClientError::Timeout {
                        seconds: self.config.timeout.as_secs(),
                    }
                } else {
                    warn!(?err, "AI service transport error");
                    AiClientError::Transport(err)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            warn!(status = status.as_u16(), "AI service returned non-success status");
            return Err(AiClientError::HttpStatus {
                status: status.as_u16(),
            });
        }

        let body = response.text().await.map_err(|err| {
            warn!(?err, "failed to read AI service response body");
            AiClientError::Transport(err)
        })?;

        let chat: ChatResponse = serde_json::from_str(&body).map_err(|err| {
            warn!(?err, "failed to parse AI service response as JSON");
            AiClientError::InvalidJson(err)
        })?;

        let content = chat
            .choices
            .first()
            .map(|c| c.message.content.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                warn!("AI service returned empty or missing choices");
                AiClientError::EmptyResponse
            })?;

        debug!(content = %content, "received AI model output");

        let parsed: SentimentResponse = serde_json::from_str(content).map_err(|err| {
            warn!(?err, content, "failed to parse sentiment from model output");
            AiClientError::ParseFailure
        })?;

        Ok(parsed.sentiment)
    }
}

#[async_trait]
impl AiProvider for QwenClient {
    async fn analyze(&self, prompt: &str) -> Result<Sentiment, AiClientError> {
        let raw = self.call_api(prompt).await?;
        Ok(Sentiment::new_clamped(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_includes_user_prompt() {
        let config = AiConfig {
            model: "test-model".to_owned(),
            ..Default::default()
        };
        let client = QwenClient::new(config);
        let req = client.build_request("沪深300指数今日大幅上涨");

        assert_eq!(req.model, "test-model");
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, "system");
        assert!(req.messages[0].content.contains("sentiment"));
        assert_eq!(req.messages[1].role, "user");
        assert_eq!(req.messages[1].content, "沪深300指数今日大幅上涨");
        assert_eq!(req.temperature, 0.0);
        assert_eq!(req.max_tokens, 128);
    }

    #[test]
    fn chat_url_trims_trailing_slash() {
        let config = AiConfig {
            base_url: "https://api.example.com/v1/".to_owned(),
            ..Default::default()
        };
        let client = QwenClient::new(config);
        assert_eq!(
            client.chat_url(),
            "https://api.example.com/v1/v1/chat/completions"
        );
    }

    #[test]
    fn chat_url_without_trailing_slash() {
        let config = AiConfig::default();
        let client = QwenClient::new(config);
        assert_eq!(
            client.chat_url(),
            "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
        );
    }

    #[test]
    fn parse_valid_sentiment_json() {
        let content = r#"{"sentiment": 0.7}"#;
        let parsed: SentimentResponse =
            serde_json::from_str(content).expect("valid sentiment JSON must parse");
        assert!((parsed.sentiment - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_negative_sentiment_json() {
        let content = r#"{"sentiment": -0.5}"#;
        let parsed: SentimentResponse =
            serde_json::from_str(content).expect("valid sentiment JSON must parse");
        assert!((parsed.sentiment - (-0.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_sentiment_json_rejects_missing_field() {
        let content = r#"{"other": 0.5}"#;
        let result = serde_json::from_str::<SentimentResponse>(content);
        assert!(result.is_err());
    }

    #[test]
    fn client_constructs_without_panic() {
        let config = AiConfig::default();
        let client = QwenClient::new(config);
        assert!(client.chat_url().contains("dashscope"));
    }

    #[test]
    fn system_prompt_includes_required_fields() {
        assert!(SYSTEM_PROMPT.contains("sentiment"));
        assert!(SYSTEM_PROMPT.contains("-1.0"));
        assert!(SYSTEM_PROMPT.contains("+1.0"));
        assert!(SYSTEM_PROMPT.contains("JSON"));
    }
}
