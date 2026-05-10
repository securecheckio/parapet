use crate::types::{DemoStage, StageResult};

pub fn get_demo_stages() -> Vec<DemoStage> {
    vec![
        // Stage 1: Pre-Signing Detection - RPC ALERT during signing (March 23-30)
        DemoStage {
            id: 1,
            name: "March 23-30, 2026: Pre-Signing Admin Transfer".to_string(),
            date: "March 23-30, 2026".to_string(),
            description: "Security Council member signs 'routine upgrade' via Ledger. Ledger shows: AdvanceNonceAccount + Unknown Instruction. Told it's 'air-gapped signing for security.' RPC proxy decodes the FULL transaction.".to_string(),
            tx_signature: Some("2HvMSgDEfKhNryYZKhjowrBY55rUx5MWtcWkG9hqxZCFBaTiahPwfynP1dxBSRk9s5UTVc8LFeS4Btvkm9pc2C4H".to_string()),
            expected_result: StageResult::Alert,
            talking_points: vec![
                "RPC proxy intercepts simulateTransaction during Ledger signing".to_string(),
                "Ledger shows: 'AdvanceNonceAccount + Unknown Instruction' ← Blind signing".to_string(),
                "Parapet decodes using instruction-registry.json: 'update_admin to H7PiGqq...'".to_string(),
                "CRITICAL: This is DRIFT'S OWN INSTRUCTION - but target address is UNKNOWN".to_string(),
                "H7PiGqq... has ZERO prior interactions with Drift protocol".to_string(),
                "Not a multisig member, not a known governance address = RED FLAG".to_string(),
                "PREVENTION: Reject this transaction = Attack stops here".to_string(),
                "Key insight: Problem isn't unknown program - it's signing their own instruction to unknown address".to_string(),
            ],
        },
        
        // Stage 2: Timelock Removal - MONITORING ALERT (March 27)
        DemoStage {
            id: 2,
            name: "March 27, 2026: Timelock Removal (Pre-Drain Setup)".to_string(),
            date: "March 27, 2026".to_string(),
            description: "Drift migrates to new 2/5 Security Council multisig with ZERO timelock. Removes the governance delay that allows emergency intervention. On-chain monitoring detects this critical configuration change.".to_string(),
            tx_signature: Some("9zJGhyotEes1Ni5i4Qki5zUjApWhvWcr5rxJfiLhVGtnDuVzn9eFy1XzvtrZaj8r2SZYRmMQGftGQvDS1o2pPwE".to_string()),
            expected_result: StageResult::Alert,
            talking_points: vec![
                "MONITORING ALERT - Governance timelock set to ZERO".to_string(),
                "Removes safety delay that allows detection and intervention".to_string(),
                "Instant execution = no time window to respond to malicious actions".to_string(),
                "This was the 'pre-drain setup' that enabled the April 1 exploit".to_string(),
                "Combined with pre-signed transactions from Stage 1 = attack ready to execute".to_string(),
            ],
        },
        
        // Stage 3: On-Chain Execution - MONITORING ALERT (April 1, 16:05:18)
        DemoStage {
            id: 3,
            name: "April 1, 16:05:18: Pre-Signed Transaction Executes".to_string(),
            date: "April 1, 2026 16:05:18 UTC".to_string(),
            description: "SAME transaction from Stage 1 - now broadcast by attacker. Already signed in March, hits blockchain in April. Monitoring detects admin transfer pattern.".to_string(),
            tx_signature: Some("2HvMSgDEfKhNryYZKhjowrBY55rUx5MWtcWkG9hqxZCFBaTiahPwfynP1dxBSRk9s5UTVc8LFeS4Btvkm9pc2C4H".to_string()),
            expected_result: StageResult::Alert,
            talking_points: vec![
                "MONITORING ALERT - same transaction from Stage 1, now on-chain".to_string(),
                "Too late to prevent (already signed) but enables incident response".to_string(),
                "Risk Score 95/100 - Durable nonce + admin transfer to unknown address".to_string(),
                "Alert triggers: Emergency pause, security team notification".to_string(),
            ],
        },
        
        // Stage 4: Second transaction - MONITORING ALERT (April 1, 16:05:19)
        DemoStage {
            id: 4,
            name: "April 1, 16:05:19: Approval Transaction (1 second later)".to_string(),
            date: "April 1, 2026 16:05:19 UTC".to_string(),
            description: "Second pre-signed transaction executes 1 second later, completing the admin takeover. Monitoring detects coordinated attack pattern.".to_string(),
            tx_signature: Some("4BKBmAJn6TdsENij7CsVbyMVLJU1tX27nfrMM1zgKv1bs2KJy6Am2NqdA3nJm4g9C6eC64UAf5sNs974ygB9RsN1".to_string()),
            expected_result: StageResult::Alert,
            talking_points: vec![
                "CRITICAL MONITORING ALERT - coordinated attack confirmed".to_string(),
                "Two admin actions 1 second apart = pre-coordinated exploit".to_string(),
                "Risk Score 100/100 - Multiple sequential authority changes".to_string(),
                "Detection enables emergency response (pause contracts, alert team)".to_string(),
            ],
        },
    ]
}

pub fn get_stage_by_id(id: u32) -> Option<DemoStage> {
    get_demo_stages().into_iter().find(|s| s.id == id)
}
