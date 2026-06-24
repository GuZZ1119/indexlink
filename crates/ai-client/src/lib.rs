#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! AI 语义感知层（10% 权重）。
//!
//! 此 crate 封装 LLM 调用，输出有界情绪偏移 [`Sentiment`]。
//! 超时、解析失败或模型不可用时自动降级为中性（0.0）。
//!
//! # 设计原则
//!
//! - **IO 边界适配器**：本 crate 是唯一进行网络 IO 的 AI 层；
//!   上层（decision engine）仅消费 [`Sentiment`] 值，不感知 HTTP 细节。
//! - **安全降级**：任何错误都不会阻塞定投决策；
//!   调用方使用 `.unwrap_or(Sentiment::neutral())` 即可安全降级。
//! - **密钥安全**：API Key 绝不出现在 Debug / Display / 错误消息中。
//!
//! # 示例
//!
//! ```rust,no_run
//! use ai_client::{AiConfig, AiProvider, QwenClient, Sentiment};
//!
//! # async fn example() {
//! let config = AiConfig::default();
//! let client = QwenClient::new(config);
//!
//! let sentiment = client
//!     .analyze("美联储维持利率不变，点阵图显示年内降息两次")
//!     .await
//!     .unwrap_or_else(|_| Sentiment::neutral());
//!
//! println!("AI sentiment: {sentiment}");
//! # }
//! ```

mod client;
mod error;
mod mock;
mod provider;
mod sentiment;

pub use client::QwenClient;
pub use error::AiClientError;
pub use mock::MockAiProvider;
pub use provider::{AiConfig, AiProvider};
pub use sentiment::{Sentiment, SentimentError};
