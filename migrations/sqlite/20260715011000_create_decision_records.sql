-- SQLite baseline for locally persisted decision-record audit snapshots.
--
-- JSON snapshots remain TEXT and are checked with SQLite JSON functions. The
-- application validates complete domain snapshots before a repository writes.
-- Planned contributions use the same fixed-width exact-decimal format as plans.

CREATE TABLE decision_records (
    id TEXT PRIMARY KEY NOT NULL,
    plan_id TEXT NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    symbol TEXT NOT NULL,
    currency TEXT NOT NULL,
    execution_status TEXT NOT NULL,
    planned_contribution TEXT,
    execution_snapshot TEXT NOT NULL,
    fundamental_snapshot TEXT NOT NULL,
    trend_snapshot TEXT NOT NULL,
    sentiment_snapshot TEXT,
    decision_snapshot TEXT NOT NULL,
    broker_order_request TEXT,
    broker_order_ack TEXT,
    summary TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    CONSTRAINT decision_records_symbol_snapshot_check
        CHECK (
            symbol = trim(symbol)
            AND symbol = upper(symbol)
            AND length(symbol) BETWEEN 1 AND 32
            AND length(CAST(symbol AS BLOB)) = length(symbol)
        ),
    CONSTRAINT decision_records_currency_check
        CHECK (currency GLOB '[A-Z][A-Z][A-Z]'),
    CONSTRAINT decision_records_execution_status_check
        CHECK (execution_status IN ('due', 'waiting', 'inactive')),
    CONSTRAINT decision_records_planned_contribution_check
        CHECK (
            planned_contribution IS NULL
            OR (
                planned_contribution GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9].[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'
                AND planned_contribution > '000000000000.00000000'
            )
        ),
    CONSTRAINT decision_records_execution_snapshot_json_check
        CHECK (json_valid(execution_snapshot)),
    CONSTRAINT decision_records_fundamental_snapshot_json_check
        CHECK (json_valid(fundamental_snapshot)),
    CONSTRAINT decision_records_trend_snapshot_json_check
        CHECK (json_valid(trend_snapshot)),
    CONSTRAINT decision_records_sentiment_snapshot_json_check
        CHECK (sentiment_snapshot IS NULL OR json_valid(sentiment_snapshot)),
    CONSTRAINT decision_records_decision_snapshot_json_check
        CHECK (json_valid(decision_snapshot)),
    CONSTRAINT decision_records_broker_order_request_json_check
        CHECK (broker_order_request IS NULL OR json_valid(broker_order_request)),
    CONSTRAINT decision_records_broker_order_ack_json_check
        CHECK (broker_order_ack IS NULL OR json_valid(broker_order_ack)),
    CONSTRAINT decision_records_summary_not_blank_check
        CHECK (summary = trim(summary) AND length(summary) BETWEEN 1 AND 2000),
    CONSTRAINT decision_records_created_at_utc_check
        CHECK (
            strftime('%Y-%m-%dT%H:%M:%fZ', created_at) IS NOT NULL
            AND strftime('%Y-%m-%dT%H:%M:%fZ', created_at) = created_at
        )
);

CREATE INDEX decision_records_plan_created_idx
    ON decision_records (plan_id, created_at DESC, id DESC);

CREATE INDEX decision_records_created_idx
    ON decision_records (created_at DESC, id DESC);
