use crate::types::{DemoStage, StageResult};

pub fn get_demo_stages() -> Vec<DemoStage> {
    vec![
        DemoStage {
            id: 1,
            name: "Pre-sign admin transfer".to_string(),
            date: "March 23–30, 2026".to_string(),
            description: "Team signs blind: advance nonce + unknown IX.".to_string(),
            tx_signature: Some(
                "2HvMSgDEfKhNryYZKhjowrBY55rUx5MWtcWkG9hqxZCFBaTiahPwfynP1dxBSRk9s5UTVc8LFeS4Btvkm9pc2C4H"
                    .to_string(),
            ),
            expected_result: StageResult::Alert,
            talking_points: vec![],
        },
        DemoStage {
            id: 2,
            name: "Multisig migration".to_string(),
            date: "March 27, 2026".to_string(),
            description: "Vault execute: real team, governance-risk pattern.".to_string(),
            tx_signature: Some(
                "9zJGhyotEes1Ni5i4Qki5zUjApWhvWcr5rxJfiLhVGtnDuVzn9eFy1XzvtrZaj8r2SZYRmMQGftGQvDS1o2pPwE"
                    .to_string(),
            ),
            expected_result: StageResult::Alert,
            talking_points: vec![],
        },
        DemoStage {
            id: 3,
            name: "Pre-signed executes".to_string(),
            date: "April 1, 2026 16:05:18 UTC".to_string(),
            description: "Attacker broadcasts.".to_string(),
            tx_signature: Some(
                "2HvMSgDEfKhNryYZKhjowrBY55rUx5MWtcWkG9hqxZCFBaTiahPwfynP1dxBSRk9s5UTVc8LFeS4Btvkm9pc2C4H"
                    .to_string(),
            ),
            expected_result: StageResult::Alert,
            talking_points: vec![],
        },
        DemoStage {
            id: 4,
            name: "Follow-up (+1s)".to_string(),
            date: "April 1, 2026 16:05:19 UTC".to_string(),
            description: "Second chained tx.".to_string(),
            tx_signature: Some(
                "4BKBmAJn6TdsENij7CsVbyMVLJU1tX27nfrMM1zgKv1bs2KJy6Am2NqdA3nJm4g9C6eC64UAf5sNs974ygB9RsN1"
                    .to_string(),
            ),
            expected_result: StageResult::Alert,
            talking_points: vec![],
        },
    ]
}

pub fn get_stage_by_id(id: u32) -> Option<DemoStage> {
    get_demo_stages().into_iter().find(|s| s.id == id)
}
