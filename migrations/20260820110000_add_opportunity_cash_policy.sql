-- Persist the user's intent for opportunity-bucket funds that are not used in
-- the current period. A ledger-backed balance is intentionally deferred.

ALTER TABLE investment_plan_execution_configurations
    ADD COLUMN opportunity_cash_policy TEXT NOT NULL
    DEFAULT 'expire_each_period'
    CONSTRAINT investment_plan_execution_configurations_cash_policy_check
    CHECK (opportunity_cash_policy IN ('expire_each_period', 'carry_forward'));
