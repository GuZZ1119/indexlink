-- PostgreSQL parity migration for the V1.1 executable schedule/cash ledger.
ALTER TABLE investment_plan_execution_configurations
    ADD COLUMN schedule_days_json JSONB NOT NULL DEFAULT '[]'::jsonb;

UPDATE investment_plan_execution_configurations
SET schedule_days_json = jsonb_build_array(schedule_day)
WHERE schedule_days_json = '[]'::jsonb;

CREATE TABLE opportunity_cash_balances (
    plan_id UUID PRIMARY KEY REFERENCES investment_plans(id) ON DELETE CASCADE,
    balance NUMERIC(20, 8) NOT NULL DEFAULT 0 CHECK (balance >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE opportunity_cash_events (
    id UUID PRIMARY KEY,
    plan_id UUID NOT NULL REFERENCES investment_plans(id) ON DELETE CASCADE,
    decision_record_id UUID NOT NULL UNIQUE REFERENCES decision_records(id) ON DELETE CASCADE,
    scheduled_for DATE NOT NULL,
    policy TEXT NOT NULL CHECK (policy IN ('expire_each_period', 'carry_forward')),
    balance_before NUMERIC(20, 8) NOT NULL CHECK (balance_before >= 0),
    period_budget NUMERIC(20, 8) NOT NULL CHECK (period_budget >= 0),
    allocated_amount NUMERIC(20, 8) NOT NULL CHECK (allocated_amount >= 0),
    balance_after NUMERIC(20, 8) NOT NULL CHECK (balance_after >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
