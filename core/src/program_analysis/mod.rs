// Program Analysis Module
// Three-tier analysis system for Solana programs

pub mod fetcher;
pub mod types;
pub mod disassembler;
pub mod semantic;
pub mod cache;

#[cfg(feature = "ai-analysis")]
pub mod ai_analyzer;

// Re-exports
pub use fetcher::ProgramFetcher;
pub use disassembler::ProgramDisassembler;
pub use semantic::SemanticAnalyzer;
pub use cache::{ProgramCache, CacheConfig};

#[cfg(feature = "ai-analysis")]
pub use ai_analyzer::{AiAnalyzer, AiProviderConfig};

pub use types::*;

use anyhow::Result;

/// Analysis tier determines depth of program inspection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisTier {
    /// Fast checks: known-safe lists, Helius, OtterSec (10-50ms)
    Superficial,
    /// Medium: + bytecode fetch, disassembly, semantic analysis (200-1000ms)
    Deep,
    /// Slow: + AI/LLM analysis (2-10s)
    AI,
}

/// Analysis execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisMode {
    /// Block until analysis completes
    Synchronous,
    /// Enqueue for worker processing, return immediately
    Asynchronous,
}

/// Main program analysis service orchestrator
pub struct ProgramAnalysisService {
    fetcher: ProgramFetcher,
    disassembler: ProgramDisassembler,
    semantic_analyzer: SemanticAnalyzer,
    #[cfg(feature = "ai-analysis")]
    ai_analyzer: Option<AiAnalyzer>,
    // TODO: Add cache, work_queue fields
}

impl ProgramAnalysisService {
    /// Create new analysis service
    pub fn new(rpc_url: String) -> Self {
        Self {
            fetcher: ProgramFetcher::new(rpc_url),
            disassembler: ProgramDisassembler::new().expect("Failed to initialize disassembler"),
            semantic_analyzer: SemanticAnalyzer::new(),
            #[cfg(feature = "ai-analysis")]
            ai_analyzer: None, // Will be enabled if configured
        }
    }

    /// Create new analysis service with AI enabled
    #[cfg(feature = "ai-analysis")]
    pub fn new_with_ai(rpc_url: String, ai_config: AiProviderConfig) -> Self {
        Self {
            fetcher: ProgramFetcher::new(rpc_url),
            disassembler: ProgramDisassembler::new().expect("Failed to initialize disassembler"),
            semantic_analyzer: SemanticAnalyzer::new(),
            ai_analyzer: Some(AiAnalyzer::new(ai_config)),
        }
    }

    /// Analyze a program at specified tier
    pub async fn analyze_program(
        &self,
        program_id: &str,
        tier: AnalysisTier,
        _mode: AnalysisMode, // TODO: Implement async mode with work queue
    ) -> Result<ProgramAnalysisResult> {
        use solana_sdk::pubkey::Pubkey;
        use std::str::FromStr;
        use sha2::{Sha256, Digest};
        use chrono::Utc;

        let start_time = std::time::Instant::now();
        let program_pubkey = Pubkey::from_str(program_id)?;

        // Fetch program data
        let program_data = self.fetcher.fetch_program(&program_pubkey).await?;

        // Calculate bytecode hash
        let mut hasher = Sha256::new();
        hasher.update(&program_data.executable_data);
        let bytecode_hash = format!("{:x}", hasher.finalize());

        // Tier 1: Superficial analysis (use existing analyzers)
        // TODO: Integrate with AnalyzerRegistry to run Helius, OtterSec, etc.
        let helius_identity = None; // Would come from HeliusIdentityAnalyzer
        let ottersec_verified = false; // Would come from OtterSecVerifiedAnalyzer

        if tier == AnalysisTier::Superficial {
            let analysis_time = start_time.elapsed().as_millis() as u64;
            return Ok(ProgramAnalysisResult {
                program_id: program_id.to_string(),
                bytecode_hash: Some(bytecode_hash),
                tier_used: "superficial".to_string(),
                risk_score: 50.0,
                risk_level: RiskLevel::Medium,
                is_safe: false,
                helius_identity,
                ottersec_verified,
                bytecode_analysis: None,
                suspicious_patterns: vec![],
                ai_analysis: None,
                vulnerabilities: vec![],
                recommendations: vec!["Program not verified by known sources".to_string()],
                worker_wallet_address: None,
                worker_signature: None,
                cached: false,
                analysis_time_ms: analysis_time,
                created_at: Utc::now().to_rfc3339(),
            });
        }

        // Tier 2: Deep analysis (+ disassembly + semantic)
        let disassembly = self.disassembler.disassemble(&program_data.executable_data)?;
        let semantic = self.semantic_analyzer.analyze_program(&program_data, Some(&disassembly)).await?;

        let bytecode_analysis = Some(BytecodeAnalysis {
            total_instructions: disassembly.total_instructions,
            suspicious_instruction_count: disassembly.suspicious_instruction_count,
            control_flow_graph: None, // Would be generated from semantic analysis
            complexity_score: disassembly.complexity_score,
            entropy_score: disassembly.entropy_score,
        });

        // Calculate risk score based on disassembly and semantic
        let mut risk_score = 0.0;
        risk_score += disassembly.entropy_score * 20.0; // High entropy is suspicious
        risk_score += semantic.control_flow_complexity * 30.0;
        risk_score += (disassembly.suspicious_patterns.len() * 10) as f64;
        risk_score = risk_score.min(100.0);

        let risk_level = match risk_score as u8 {
            0..=20 => RiskLevel::VeryLow,
            21..=40 => RiskLevel::Low,
            41..=60 => RiskLevel::Medium,
            61..=80 => RiskLevel::High,
            _ => RiskLevel::Critical,
        };

        if tier == AnalysisTier::Deep {
            let analysis_time = start_time.elapsed().as_millis() as u64;
            return Ok(ProgramAnalysisResult {
                program_id: program_id.to_string(),
                bytecode_hash: Some(bytecode_hash),
                tier_used: "deep".to_string(),
                risk_score,
                risk_level,
                is_safe: risk_score < 40.0,
                helius_identity,
                ottersec_verified,
                bytecode_analysis,
                suspicious_patterns: disassembly.suspicious_patterns.clone(),
                ai_analysis: None,
                vulnerabilities: vec![],
                recommendations: vec!["Deep analysis completed".to_string()],
                worker_wallet_address: None,
                worker_signature: None,
                cached: false,
                analysis_time_ms: analysis_time,
                created_at: Utc::now().to_rfc3339(),
            });
        }

        // Tier 3: AI analysis
        #[cfg(feature = "ai-analysis")]
        {
            if let Some(ref ai_analyzer) = self.ai_analyzer {
                let ai_result = ai_analyzer.analyze_program(&program_data, &disassembly, Some(&semantic)).await?;
                
                let ai_analysis = Some(AiAnalysis {
                    model_used: ai_result.model_used.clone(),
                    behavioral_analysis: ai_result.behavioral_analysis.clone(),
                    code_quality_assessment: "AI-based assessment".to_string(), // Would be expanded
                    confidence_score: ai_result.confidence_score,
                });

                let vulnerabilities = ai_result.vulnerabilities.iter().map(|v| Vulnerability {
                    severity: v.severity.clone(),
                    category: v.category.clone(),
                    description: v.description.clone(),
                    location: None,
                }).collect();

                let analysis_time = start_time.elapsed().as_millis() as u64;
                return Ok(ProgramAnalysisResult {
                    program_id: program_id.to_string(),
                    bytecode_hash: Some(bytecode_hash),
                    tier_used: "ai".to_string(),
                    risk_score: ai_result.risk_score,
                    risk_level: match ai_result.risk_level.as_str() {
                        "VeryLow" => RiskLevel::VeryLow,
                        "Low" => RiskLevel::Low,
                        "Medium" => RiskLevel::Medium,
                        "High" => RiskLevel::High,
                        _ => RiskLevel::Critical,
                    },
                    is_safe: ai_result.risk_score < 40.0,
                    helius_identity,
                    ottersec_verified,
                    bytecode_analysis,
                    suspicious_patterns: disassembly.suspicious_patterns.clone(),
                    ai_analysis,
                    vulnerabilities,
                    recommendations: ai_result.recommendations,
                    worker_wallet_address: None,
                    worker_signature: None,
                    cached: false,
                    analysis_time_ms: analysis_time,
                    created_at: Utc::now().to_rfc3339(),
                });
            }
        }

        // Fallback if AI not available but requested
        Err(anyhow::anyhow!("AI analysis requested but not available (compile with ai-analysis feature)"))
    }

    /// Enqueue analysis for worker processing
    pub async fn enqueue_analysis(
        &self,
        _program_id: &str,
        _account_id: Option<&str>,
        _tier: AnalysisTier,
        _priority: u8,
    ) -> Result<String> {
        // TODO: Enqueue to Redis work queue
        unimplemented!("Analysis enqueuing - requires Redis work queue implementation")
    }
}
