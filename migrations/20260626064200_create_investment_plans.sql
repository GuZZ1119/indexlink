-- Investment Plan table for the storage repository adapter.
--
-- Scope:
-- - single-user MVP, so there is no user/account foreign key yet;
-- - only monthly schedules are supported;
-- - this table stores plan configuration, not execution decisions or orders.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE investment_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    symbol TEXT NOT NULL,
    base_contribution NUMERIC(20, 8) NOT NULL,
    currency TEXT NOT NULL,
    schedule_kind TEXT NOT NULL DEFAULT 'monthly',
    schedule_day SMALLINT NOT NULL,
    max_single_execution NUMERIC(20, 8) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT investment_plans_name_normalized_check
        CHECK (name = btrim(name) AND char_length(name) BETWEEN 1 AND 100),
    CONSTRAINT investment_plans_symbol_normalized_check
        CHECK (
            symbol = btrim(symbol)
            AND symbol = upper(symbol)
            AND char_length(symbol) BETWEEN 1 AND 32
            AND octet_length(symbol) = char_length(symbol)
        ),
    CONSTRAINT investment_plans_currency_check
        CHECK (currency ~ '^[A-Z]{3}$'),
    CONSTRAINT investment_plans_schedule_kind_check
        CHECK (schedule_kind = 'monthly'),
    CONSTRAINT investment_plans_schedule_day_check
        CHECK (schedule_day BETWEEN 1 AND 28),
    CONSTRAINT investment_plans_base_contribution_positive_check
        CHECK (base_contribution > 0),
    CONSTRAINT investment_plans_max_single_execution_positive_check
        CHECK (max_single_execution > 0),
    CONSTRAINT investment_plans_max_single_execution_gte_base_check
        CHECK (max_single_execution >= base_contribution),
    CONSTRAINT investment_plans_updated_after_created_check
        CHECK (updated_at >= created_at)
);

CREATE INDEX investment_plans_created_at_id_idx
    ON investment_plans (created_at ASC, id ASC);

CREATE INDEX investment_plans_active_schedule_idx
    ON investment_plans (is_active, schedule_kind, schedule_day);
