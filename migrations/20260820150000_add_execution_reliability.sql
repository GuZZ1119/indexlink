-- PostgreSQL schema parity for the local SQLite execution-reliability migration.
ALTER TABLE investment_plan_execution_configurations
    ADD COLUMN opportunity_cash_cap NUMERIC(20, 8) NULL CHECK (opportunity_cash_cap > 0),
    ADD COLUMN period_execution_limit NUMERIC(20, 8) NULL CHECK (period_execution_limit > 0);

ALTER TABLE investment_plan_execution_configurations
    DROP CONSTRAINT investment_plan_execution_configurations_cash_policy_check,
    ADD CONSTRAINT investment_plan_execution_configurations_cash_policy_check
    CHECK (opportunity_cash_policy IN ('expire_each_period', 'carry_forward', 'carry_with_cap'));

ALTER TABLE paper_orders
    ADD COLUMN decision_record_id UUID NULL REFERENCES decision_records(id) ON DELETE SET NULL;

ALTER TABLE opportunity_cash_events
    ADD COLUMN core_contribution NUMERIC(20, 8) NOT NULL DEFAULT 0 CHECK (core_contribution >= 0),
    ADD COLUMN cash_cap NUMERIC(20, 8) NULL CHECK (cash_cap > 0),
    ADD COLUMN actual_allocated_amount NUMERIC(20, 8) NULL CHECK (actual_allocated_amount >= 0),
    ADD COLUMN reconciled_at TIMESTAMPTZ NULL;

CREATE TABLE plan_period_execution_reservations (
    decision_record_id UUID PRIMARY KEY REFERENCES decision_records(id) ON DELETE CASCADE,
    plan_id UUID NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    period_key TEXT NOT NULL,
    amount NUMERIC(20, 8) NOT NULL CHECK (amount >= 0),
    state TEXT NOT NULL CHECK (state IN ('reserved', 'accepted', 'released')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX plan_period_execution_reservations_plan_period_idx
    ON plan_period_execution_reservations (plan_id, period_key, state);
