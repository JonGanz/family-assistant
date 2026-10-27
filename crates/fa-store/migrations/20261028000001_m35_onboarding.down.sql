ALTER TABLE advisory
    ADD COLUMN dismissed_at timestamptz,
    ADD COLUMN dismissed_by uuid REFERENCES member(id);
ALTER TABLE household DROP COLUMN setup_completed_at, DROP COLUMN dashboard_pending_approvals;
DROP TABLE instance_setting;
