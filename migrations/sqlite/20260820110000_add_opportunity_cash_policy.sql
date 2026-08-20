-- Persist the user's intent for opportunity-bucket funds that are not used in
-- the current period. This migration intentionally does not create a cash
-- ledger: balance mutation and capped carry-forward are introduced later.

ALTER TABLE plan_execution_configurations
    ADD COLUMN opportunity_cash_policy TEXT NOT NULL
    DEFAULT 'expire_each_period'
    CHECK (opportunity_cash_policy IN ('expire_each_period', 'carry_forward'));
