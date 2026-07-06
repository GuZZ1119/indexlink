//! 真实新闻获取 + AI 分析的全链路集成测试。
//!
//! 运行方式：
//! ```bash
//! # 用 Mock AI（不需要 API Key）
//! cargo test -p ai-client --test news real_cnbc_with_mock -- --ignored --nocapture
//!
//! # 用真实 Qwen API（需要设置 DASHSCOPE_API_KEY 环境变量）
//! cargo test -p ai-client --test news real_cnbc_with_qwen -- --ignored --nocapture
//! ```

use ai_client::news::{fetch_market_sentiment, format_sentiment_prompt, NewsSource, RssNewsSource};
use ai_client::{AiConfig, MockAiProvider, QwenClient};

/// 真实拉取 CNBC RSS，用 Mock AI 分析情绪。
#[tokio::test]
#[ignore = "需要网络，手动运行"]
async fn real_cnbc_with_mock() {
    let source = RssNewsSource::new();
    let ai = MockAiProvider::new();

    let news = source.fetch().await.expect("failed to fetch CNBC news");
    assert!(!news.is_empty(), "should fetch at least one news item");

    // 每条新闻必须有标题和描述
    for item in &news {
        assert!(!item.title.is_empty(), "news title must not be empty");
        assert!(
            !item.description.is_empty(),
            "news description must not be empty: {}",
            item.title
        );
    }

    let prompt = format_sentiment_prompt(&news);
    assert!(!prompt.is_empty(), "prompt must not be empty");
    assert!(
        prompt.contains("Headline"),
        "prompt must contain 'Headline' label"
    );

    for item in &news {
        println!(
            "[{}] {} — {}",
            item.pub_date.format("%m-%d %H:%M"),
            item.title,
            item.description
        );
    }
    let sentiment = fetch_market_sentiment(&source, &ai).await.unwrap();
    println!("\n===== Mock Market Sentiment: {} =====\n", sentiment);

    assert!(
        (-1.0..=1.0).contains(&sentiment.value()),
        "sentiment must be in [-1, 1], got {}",
        sentiment.value()
    );
}

/// 真实拉取 CNBC RSS，用真实 Qwen API 分析情绪。
#[tokio::test]
#[ignore = "需要网络和 DASHSCOPE_API_KEY，手动运行"]
async fn real_cnbc_with_qwen() {
    let api_key = std::env::var("DASHSCOPE_API_KEY").expect("DASHSCOPE_API_KEY not set");

    let config = AiConfig {
        api_key,
        ..Default::default()
    };
    let source = RssNewsSource::new();
    let ai = QwenClient::new(config);

    let news = source.fetch().await.expect("failed to fetch CNBC news");
    assert!(!news.is_empty(), "should fetch at least one news item");

    for item in &news {
        assert!(!item.title.is_empty(), "news title must not be empty");
        assert!(
            !item.description.is_empty(),
            "news description must not be empty"
        );
    }

    let prompt = format_sentiment_prompt(&news);
    assert!(prompt.contains("Headline"), "prompt must include headlines");

    for item in &news {
        println!(
            "[{}] {} — {}",
            item.pub_date.format("%m-%d %H:%M"),
            item.title,
            item.description
        );
    }
    let sentiment = fetch_market_sentiment(&source, &ai).await.unwrap();
    println!("\n===== Qwen Market Sentiment: {} =====\n", sentiment);

    assert!(
        (-1.0..=1.0).contains(&sentiment.value()),
        "sentiment must be in [-1, 1], got {}",
        sentiment.value()
    );
}
