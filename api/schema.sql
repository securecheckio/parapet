-- Sol-Shield Escalation Audit Trail Database Schema
-- PostgreSQL schema for permanent storage of escalation events

-- Escalations table: stores all escalation records
CREATE TABLE IF NOT EXISTS escalations (
    id SERIAL PRIMARY KEY,
    escalation_id VARCHAR(64) UNIQUE NOT NULL,
    canonical_hash VARCHAR(128) NOT NULL,
    requester_wallet VARCHAR(44) NOT NULL,
    approver_wallet VARCHAR(44) NOT NULL,
    risk_score SMALLINT NOT NULL CHECK (risk_score >= 0 AND risk_score <= 100),
    status VARCHAR(32) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    
    -- Indexed for queries
    INDEX idx_escalation_id (escalation_id),
    INDEX idx_requester_wallet (requester_wallet),
    INDEX idx_approver_wallet (approver_wallet),
    INDEX idx_canonical_hash (canonical_hash),
    INDEX idx_created_at (created_at),
    INDEX idx_status (status)
);

-- Escalation details: stores warnings, decoded instructions, and suggested rules
CREATE TABLE IF NOT EXISTS escalation_details (
    id SERIAL PRIMARY KEY,
    escalation_id VARCHAR(64) NOT NULL REFERENCES escalations(escalation_id) ON DELETE CASCADE,
    warnings JSONB,
    decoded_instructions JSONB,
    suggested_rules JSONB,
    
    INDEX idx_escalation_id (escalation_id)
);

-- Escalation events: audit trail of status changes
CREATE TABLE IF NOT EXISTS escalation_events (
    id SERIAL PRIMARY KEY,
    escalation_id VARCHAR(64) NOT NULL REFERENCES escalations(escalation_id) ON DELETE CASCADE,
    event_type VARCHAR(32) NOT NULL, -- 'created', 'approved', 'denied', 'expired', 'forwarded'
    actor_wallet VARCHAR(44), -- Wallet that performed the action (NULL for system events)
    actor_ip VARCHAR(45), -- IP address of actor (IPv4 or IPv6)
    event_data JSONB, -- Additional event-specific data
    timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
    
    INDEX idx_escalation_id (escalation_id),
    INDEX idx_event_type (event_type),
    INDEX idx_actor_wallet (actor_wallet),
    INDEX idx_timestamp (timestamp)
);

-- Dynamic rules: stores rules created from escalation approvals
CREATE TABLE IF NOT EXISTS dynamic_rules (
    id SERIAL PRIMARY KEY,
    rule_id VARCHAR(64) UNIQUE NOT NULL,
    escalation_id VARCHAR(64) REFERENCES escalations(escalation_id) ON DELETE SET NULL,
    rule_definition JSONB NOT NULL,
    source VARCHAR(32) NOT NULL, -- 'user_consent', 'system', etc.
    created_by VARCHAR(44) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP,
    use_count INTEGER DEFAULT 0,
    use_count_limit INTEGER,
    is_active BOOLEAN DEFAULT TRUE,
    
    INDEX idx_rule_id (rule_id),
    INDEX idx_escalation_id (escalation_id),
    INDEX idx_created_by (created_by),
    INDEX idx_is_active (is_active)
);

-- Transaction signatures: links escalations to actual on-chain transactions
CREATE TABLE IF NOT EXISTS escalation_transactions (
    id SERIAL PRIMARY KEY,
    escalation_id VARCHAR(64) NOT NULL REFERENCES escalations(escalation_id) ON DELETE CASCADE,
    signature VARCHAR(88) NOT NULL, -- Solana transaction signature (base58)
    sent_at TIMESTAMP NOT NULL DEFAULT NOW(),
    confirmed BOOLEAN DEFAULT FALSE,
    confirmed_at TIMESTAMP,
    slot BIGINT,
    
    INDEX idx_escalation_id (escalation_id),
    INDEX idx_signature (signature),
    INDEX idx_sent_at (sent_at)
);

-- Approver statistics: track approval patterns
CREATE TABLE IF NOT EXISTS approver_stats (
    wallet VARCHAR(44) PRIMARY KEY,
    total_escalations INTEGER DEFAULT 0,
    total_approved INTEGER DEFAULT 0,
    total_denied INTEGER DEFAULT 0,
    total_expired INTEGER DEFAULT 0,
    avg_response_time_seconds INTEGER,
    last_activity TIMESTAMP,
    
    INDEX idx_last_activity (last_activity)
);

-- Create function to update escalation updated_at timestamp
CREATE OR REPLACE FUNCTION update_escalation_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to auto-update updated_at
CREATE TRIGGER escalation_update_timestamp
    BEFORE UPDATE ON escalations
    FOR EACH ROW
    EXECUTE FUNCTION update_escalation_timestamp();

-- Create function to update approver stats
CREATE OR REPLACE FUNCTION update_approver_stats()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.event_type IN ('approved', 'denied', 'expired') THEN
        INSERT INTO approver_stats (wallet, total_escalations, total_approved, total_denied, total_expired, last_activity)
        SELECT 
            e.approver_wallet,
            1,
            CASE WHEN NEW.event_type = 'approved' THEN 1 ELSE 0 END,
            CASE WHEN NEW.event_type = 'denied' THEN 1 ELSE 0 END,
            CASE WHEN NEW.event_type = 'expired' THEN 1 ELSE 0 END,
            NOW()
        FROM escalations e
        WHERE e.escalation_id = NEW.escalation_id
        ON CONFLICT (wallet) DO UPDATE SET
            total_escalations = approver_stats.total_escalations + 1,
            total_approved = approver_stats.total_approved + CASE WHEN NEW.event_type = 'approved' THEN 1 ELSE 0 END,
            total_denied = approver_stats.total_denied + CASE WHEN NEW.event_type = 'denied' THEN 1 ELSE 0 END,
            total_expired = approver_stats.total_expired + CASE WHEN NEW.event_type = 'expired' THEN 1 ELSE 0 END,
            last_activity = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to update stats on escalation events
CREATE TRIGGER update_approver_stats_trigger
    AFTER INSERT ON escalation_events
    FOR EACH ROW
    EXECUTE FUNCTION update_approver_stats();

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_escalations_by_approver_and_status 
    ON escalations(approver_wallet, status);

CREATE INDEX IF NOT EXISTS idx_escalations_by_requester_and_date 
    ON escalations(requester_wallet, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_escalation_events_by_type_and_date 
    ON escalation_events(event_type, timestamp DESC);

-- View for escalation summary with latest event
CREATE OR REPLACE VIEW escalation_summary AS
SELECT 
    e.escalation_id,
    e.canonical_hash,
    e.requester_wallet,
    e.approver_wallet,
    e.risk_score,
    e.status,
    e.created_at,
    e.updated_at,
    e.expires_at,
    ed.warnings,
    ed.decoded_instructions,
    ed.suggested_rules,
    (SELECT COUNT(*) FROM escalation_events ee WHERE ee.escalation_id = e.escalation_id) as event_count,
    (SELECT event_type FROM escalation_events ee WHERE ee.escalation_id = e.escalation_id ORDER BY timestamp DESC LIMIT 1) as last_event_type,
    (SELECT timestamp FROM escalation_events ee WHERE ee.escalation_id = e.escalation_id ORDER BY timestamp DESC LIMIT 1) as last_event_time
FROM escalations e
LEFT JOIN escalation_details ed ON e.escalation_id = ed.escalation_id;

-- View for approval metrics
CREATE OR REPLACE VIEW approval_metrics AS
SELECT 
    approver_wallet,
    COUNT(*) as total,
    COUNT(*) FILTER (WHERE status = 'approved') as approved,
    COUNT(*) FILTER (WHERE status = 'approved_fast_path') as approved_fast,
    COUNT(*) FILTER (WHERE status = 'approved_slow_path') as approved_slow,
    COUNT(*) FILTER (WHERE status = 'denied') as denied,
    COUNT(*) FILTER (WHERE status = 'expired') as expired,
    AVG(risk_score) as avg_risk_score,
    MIN(created_at) as first_escalation,
    MAX(created_at) as last_escalation
FROM escalations
GROUP BY approver_wallet;

-- Comments for documentation
COMMENT ON TABLE escalations IS 'Main escalation records with core metadata';
COMMENT ON TABLE escalation_details IS 'Detailed information about each escalation (warnings, instructions, rules)';
COMMENT ON TABLE escalation_events IS 'Audit trail of all status changes and actions on escalations';
COMMENT ON TABLE dynamic_rules IS 'Rules created from escalation approvals';
COMMENT ON TABLE escalation_transactions IS 'Links escalations to on-chain Solana transactions';
COMMENT ON TABLE approver_stats IS 'Aggregated statistics for each approver wallet';
COMMENT ON VIEW escalation_summary IS 'Convenient view combining escalation data with latest event';
COMMENT ON VIEW approval_metrics IS 'Aggregated approval metrics by approver';
