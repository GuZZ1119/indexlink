<p align="center">
  <img src="assets/icons/indexlink-logo.png" alt="IndexLink" width="400">
</p>

<p align="center">
  English | <a href="./README.md">中文文档</a>
</p>

<p align="center">
  <a href="https://github.com/jamesra26/indexlink/blob/main/Cargo.toml"><img src="https://img.shields.io/badge/version-0.1.0-blue" alt="Version"></a>
  <a href="https://github.com/jamesra26/indexlink/releases"><img src="https://img.shields.io/github/v/release/jamesra26/indexlink?display_name=tag" alt="Latest Release"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://github.com/jamesra26/indexlink"><img src="https://img.shields.io/badge/status-early%20development-orange" alt="Status"></a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-edition%202021-orange.svg" alt="Rust"></a>
  <a href="https://doc.rust-lang.org/cargo/"><img src="https://img.shields.io/badge/Cargo-workspace-lightgrey.svg" alt="Cargo Workspace"></a>
  <a href="https://github.com/jamesra26/indexlink"><img src="https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg" alt="Platform"></a>
  <a href="https://github.com/jamesra26/indexlink/tree/main/crates"><img src="https://img.shields.io/badge/crates-core--domain%20%7C%20quant--engine-blue" alt="Crates"></a>
</p>

<p align="center">
  <a href="https://conventionalcommits.org"><img src="https://img.shields.io/badge/Conventional%20Commits-1.0.0-yellow.svg" alt="Conventional Commits"></a>
  <a href="./CHANGE_LOG.md"><img src="https://img.shields.io/badge/changelog-CHANGE__LOG.md-green" alt="Changelog"></a>
  <a href="./AGENTS.md"><img src="https://img.shields.io/badge/contributing-AGENTS.md-blue" alt="Contributing"></a>
</p>

<p align="center">
  <a href="https://github.com/jamesra26/indexlink/stargazers"><img src="https://img.shields.io/github/stars/jamesra26/indexlink?style=social" alt="GitHub Stars"></a>
  <a href="https://github.com/jamesra26/indexlink/commits/main"><img src="https://img.shields.io/github/last-commit/jamesra26/indexlink" alt="Last Commit"></a>
  <a href="https://github.com/jamesra26/indexlink/graphs/commit-activity"><img src="https://img.shields.io/github/commit-activity/m/jamesra26/indexlink" alt="Commit Activity"></a>
</p>

<p align="center">
  <a href="https://github.com/jamesra26/indexlink/issues"><img src="https://img.shields.io/github/issues/jamesra26/indexlink" alt="Open Issues"></a>
  <a href="https://github.com/jamesra26/indexlink/pulls"><img src="https://img.shields.io/github/issues-pr/jamesra26/indexlink" alt="Open PRs"></a>
  <a href="https://github.com/jamesra26/indexlink/graphs/contributors"><img src="https://img.shields.io/github/contributors/jamesra26/indexlink" alt="Contributors"></a>
</p>

<p align="center">
  <a href="https://github.com/jamesra26/indexlink/issues">Issue Tracker</a> •
  <a href="./LICENSE">License</a> •
  <a href="./CHANGE_LOG.md">Changelog</a>
</p>

IndexLink is an intelligent dollar-cost averaging (DCA) execution system designed for long-term index investors. Powered by a dual engine of **historical percentile anchors + AI semantic sensing**, it fine-tunes each scheduled investment day: invest more at relative lows, invest less at relative highs, and delay when overheated.

> **Core premise:** We cannot determine whether the market is "undervalued," but we can use data to detect its **position** within a historical distribution. IndexLink measures position only—it does not claim to know fair value. That is the essential difference between **adaptive DCA** and **market-timing speculation**.

---

## Core Philosophy

Traditional DCA becomes rigid in extreme market conditions. IndexLink exists to address:

- **Mechanical full-size buys at historical highs:** When P/E sits at the 90th percentile historically and sentiment is overheated, automatically trigger "delay" or "reduce size."
- **Fixed amounts at historical lows:** When price / ERP percentiles fall in a historical low band, automatically suggest or execute a modest increase within DCA discipline.
- **The "good news is priced in" trap:** Combine earnings-season expectation gaps with macro news to flag "false prosperity."

---

## Decision Model: The 70/20/10 Rule

The system rejects "blind AI fantasy." Every instruction follows this weighted logic:

| Dimension                             | Weight  | Core Indicators                                             | Role of AI                                                                                |
| :------------------------------------ | :------ | :---------------------------------------------------------- | :---------------------------------------------------------------------------------------- |
| **Historical Position (Fundamental)** | **70%** | Shiller P/E, ERP, historical percentiles                    | **Hard constraint:** compute where the current price sits in its historical distribution. |
| **Recent Trend (Technical)**          | **20%** | Distance from 200-day MA, RSI, volatility (VIX)             | **Rhythm control:** detect "catching a falling knife" or "chasing the top."               |
| **Semantic Sensing (Sentiment)**      | **10%** | Earnings expectation gaps, macro news, user-defined sources | **Soft nudge:** use Qwen to infer directional bias behind news and rating changes.        |

---

## Key Features

- 🤖 **Qwen Decision Engine:** Reads key financial news and earnings guidance for the week; identifies expectation gaps.
- 🦀 **Production-grade Rust backend:** Rust (Axum + Tokio) ensures reliable scheduling so financial instructions fire on time.
- 📊 **Dynamic action space:**
  - **Overweight (+20~50%):** Modest increase within DCA discipline when in a historical low band and not in an extreme sharp decline.
  - **Standard (100%):** Steady execution when in a neutral historical band (roughly 30%~70th percentile).
  - **Tactical Delay:** Suggest delaying 3–5 days due to major news (e.g. NFP, FOMC) or technical overheating.
  - **Underweight (-50%) / Skip:** Reduce size or sit out when in a historical high band or under systemic risk.
- 🔌 **Automated trading interface:** Mock mode and real broker APIs (Broker Adapter) for end-to-end decision-to-fill flow.
- 📜 **Transparent audit log:** Each order generates an AI Decision Record explaining why the adjustment was made.

---

## Technical Architecture

### Design Principles

1. **Determinism first, AI constrained:** 70% + 20% are pure, reproducible computations; the 10% AI layer only nudges within bounded limits. If AI is unavailable, degrade to 90/10/0 and keep running.
2. **Position language in the data model:** Core output is historical percentile, not value judgment.
3. **Financial reliability triad:** **Idempotency** (no duplicate orders on the same DCA day), **audit** (every decision replayable), **circuit breaker** (default to Skip on anomalies, never reckless investing).
4. **Decision vs. execution separation:** Decision computation and order placement are two stages; user confirmation can sit in between.

### Layered Overview

```mermaid
graph TD
    subgraph Ingestion[Data Ingestion]
        MD[Market Data<br/>Price/PE/VIX]
        NEWS[News/Earnings Sources]
    end

    subgraph Core[Rust Core Axum + Tokio]
        SCH[Scheduler<br/>DCA Day Trigger]
        QUANT[Quant Engine<br/>Percentile/MA/ERP]
        AICLI[AI Client<br/>Qwen Adapter]
        DEC[Decision Engine<br/>70/20/10 Weighting]
        EXEC[Execution Orchestrator<br/>Idempotency + Confirm]
    end

    subgraph Adapters[External Adapters]
        BROKER[Broker Adapter<br/>Mock / Real]
    end

    subgraph Storage[Persistence]
        DB[(State/Audit/Cache)]
    end

    MD --> QUANT
    NEWS --> AICLI
    SCH --> DEC
    QUANT --> DEC
    AICLI --> DEC
    DEC --> EXEC
    EXEC --> BROKER
    DEC --> DB
    EXEC --> DB
    QUANT --> DB
```

### Module Responsibilities

| Module                     | Weight    | Responsibility                                                                                                                                |
| :------------------------- | :-------- | :-------------------------------------------------------------------------------------------------------------------------------------------- |
| **Scheduler**              | —         | Trigger DCA-day decisions via Tokio + persistent task table; idempotency key; survives process restarts.                                      |
| **Quant Engine**           | 70% + 20% | Convert all indicators to percentiles within their own historical distributions; pure functions, no IO, shared by live trading and backtests. |
| **AI Client**              | 10%       | Wrap Qwen; output bounded sentiment offset `sentiment ∈ [-1, +1]`; return 0 on timeout/parse failure (degraded mode).                         |
| **Decision Engine**        | —         | Combine 70/20/10 into a composite score, map to DCA multiplier, emit `Decision` with input snapshot.                                          |
| **Execution Orchestrator** | —         | Decision → (optional) user confirm → idempotent order; state machine `Pending→Confirmed→Submitted→Filled/Failed/Skipped`.                     |
| **Broker Adapter**         | —         | One trait, two implementations: `MockBroker` (backtest/demo) and `RealBroker` (live).                                                         |

### Decision Pipeline

```text
Composite score S = 0.70 * f_value(percentile)     // historical position, dominant
                  + 0.20 * f_trend(MA/RSI/VIX)     // rhythm
                  + 0.10 * sentiment               // bounded AI nudge

Multiplier = clamp( map(S), 0.0, x )               // upper bound x is user-configurable; lower bound Skip
```

- When **low but sharply falling**, `f_trend` applies a negative correction—"don't catch a falling knife"; increases stay conservative.
- `clamp` is a hard safety bound: regardless of AI output, the multiplier always stays within `[0, 1.5]`.
- Actions (Overweight / Standard / Delay / Underweight / Skip) are labels for multiplier bands.

### Project Structure (Cargo Workspace)

```text
indexlink/
├─ crates/
│  ├─ core-domain/      # Types: Decision, Action, Percentile (no IO)
│  ├─ quant-engine/     # 70%+20% pure computation (no IO)
│  ├─ ai-client/        # Qwen adapter + degradation logic
│  ├─ decision/         # 70/20/10 synthesis + mapping
│  ├─ broker/           # Broker trait + Mock/Real impl
│  ├─ scheduler/        # Tokio persistent scheduling
│  ├─ storage/          # DB access (audit/state/cache)
│  └─ api/              # Axum HTTP layer (confirm/query/override)
└─ apps/
   └─ server/           # Binary entrypoint assembling all crates
```

> Designing `quant-engine` / `decision` as pure logic crates with no IO is key: **live trading and backtests share the same decision code**.

### Persistence & Audit

| Table          | Purpose                                                             |
| :------------- | :------------------------------------------------------------------ |
| `plans`        | DCA plans (symbol, schedule, base amount, risk params)              |
| `decisions`    | Each decision + **input snapshot** + rationale (AI Decision Record) |
| `orders`       | Order state machine + idempotency keys                              |
| `market_cache` | Market/indicator cache for same-day reproducibility                 |

> Audit principle: **store inputs, not just conclusions**—persist percentiles, trend signals, sentiment, and weights at decision time so you can answer "why did we add 30% that day?"

### Reliability & Safety

- **Idempotency:** Unique constraint on `(plan_id, as_of_date)` + order idempotency keys to prevent duplicate fills.
- **Circuit breaker / Kill Switch:** Default to **Skip** on missing data or broker errors—never invest when uncertain.
- **Degradation chain:** AI down → 90/10/0; market feed down → cache or skip day; broker down → retry then manual handoff.
- **Amount safety:** Hard-coded multiplier cap + daily amount cap; AI cannot override.
- **Manual override:** Axum endpoints for confirm / reject / manual override; every intervention is audited.

### Phased Rollout

1. **MVP:** `core-domain` + `quant-engine` (70% only) + `MockBroker` + local backtest—validate percentile-driven adaptive DCA.
2. **Add rhythm:** Wire in 20% trend + circuit breaker.
3. **Add AI:** Qwen 10% bounded nudge + degradation.
4. **Scheduling & execution:** Persistent Scheduler + idempotent orders + audit log.
5. **Go live:** `RealBroker` + human confirmation flow.

---

## Disclaimer

> **This project is for learning and technical research only. It does not constitute investment advice.**

- **Not investment advice:** All decisions, multipliers, and signals from IndexLink are quantitative outputs based on historical data. They are not buy/sell recommendations and do not predict market direction.
- **No guarantee of returns:** Index investing carries risk of loss. Historical percentiles and backtest results **do not predict** future performance. You bear full responsibility for any investment decisions made using this system.
- **Adaptive ≠ market timing:** This system measures price **position** within a historical distribution only. It does **not** claim to judge whether the market is "undervalued" or "overvalued," and cannot guarantee "buying the bottom."
- **Use at your own risk:** Before connecting a real broker API, fully understand the code and risks, and test thoroughly. The authors are not liable for any direct or indirect losses from use of this software.
- **Compliance:** Automated trading may be restricted by laws and broker terms in your jurisdiction. Confirm compliance before use.
