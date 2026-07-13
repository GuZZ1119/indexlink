-- Decision records table for audit-ready preview and execution snapshots.
--
-- Scope:
-- - records are tied to investment plans;
-- - external inputs and outputs are stored as JSONB snapshots so Qwen,
--   quant, and broker adapters can evolve without losing replay context;
-- - this table stores audit records, not broker order state transitions.

CREATE TABLE decision_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id UUID NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    symbol TEXT NOT NULL,
    currency TEXT NOT NULL,
    execution_status TEXT NOT NULL,
    planned_contribution TEXT,
    execution_snapshot JSONB NOT NULL,
    fundamental_snapshot JSONB NOT NULL,
    trend_snapshot JSONB NOT NULL,
    sentiment_snapshot JSONB,
    decision_snapshot JSONB NOT NULL,
    broker_order_request JSONB,
    broker_order_ack JSONB,
    summary TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT decision_records_symbol_snapshot_check
        CHECK (
            symbol = btrim(symbol)
            AND symbol = upper(symbol)
            AND char_length(symbol) BETWEEN 1 AND 32
            AND octet_length(symbol) = char_length(symbol)
        ),
    CONSTRAINT decision_records_currency_check
        CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT decision_records_execution_status_check
        CHECK (execution_status IN ('due', 'waiting', 'inactive')),
    CONSTRAINT decision_records_planned_contribution_check
        CHECK (
            planned_contribution IS NULL
            OR planned_contribution ~ '^[0-9]+(\.[0-9]+)?$'
        ),
    CONSTRAINT decision_records_summary_not_blank_check
        CHECK (summary = btrim(summary) AND char_length(summary) BETWEEN 1 AND 2000)
);

CREATE INDEX decision_records_plan_created_idx
    ON decision_records (plan_id, created_at DESC, id DESC);

CREATE INDEX decision_records_created_idx
    ON decision_records (created_at DESC, id DESC);
