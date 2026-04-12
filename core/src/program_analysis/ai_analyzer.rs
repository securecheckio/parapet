// AI-powered program analyzer using OpenAI-compatible endpoints
// Works with OpenAI, Anthropic, Nano GPT, Groq, and other compatible providers

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::time::Instant;

use super::disassembler::DisassemblyResult;
use super::semantic::SemanticAnalysisResult;
use super::types::ProgramData;

/// AI analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalysisResult {
    pub program_id: String,
    pub risk_score: f64, // 0.0 to 100.0
    pub risk_level: String,
    pub behavioral_analysis: String,
    pub vulnerabilities: Vec<AiVulnerability>,
    pub recommendations: Vec<String>,
    pub confidence_score: f64,
    pub model_used: String,
    pub analysis_timestamp: DateTime<Utc>,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiVulnerability {
    pub severity: String,
    pub category: String,
    pub description: String,
    pub remediation: String,
}

/// AI provider configuration (OpenAI-compatible)
#[derive(Debug, Clone)]
pub struct AiProviderConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: usize,
    pub temperature: f64,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        // Check for provider-specific env vars, fallback to OpenAI
        let provider = env::var("AI_PROVIDER").unwrap_or_else(|_| "openai".to_string());

        match provider.as_str() {
            "nano-gpt" => Self {
                api_key: env::var("NANO_GPT_API_KEY").unwrap_or_default(),
                base_url: env::var("NANO_GPT_BASE_URL")
                    .unwrap_or_else(|_| "https://api.nano-gpt.com/v1".to_string()),
                model: env::var("NANO_GPT_MODEL").unwrap_or_else(|_| "glm-4-flash".to_string()),
                max_tokens: 4000,
                temperature: 0.1,
            },
            "anthropic" | "claude" => Self {
                api_key: env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
                base_url: env::var("ANTHROPIC_BASE_URL")
                    .unwrap_or_else(|_| "https://api.anthropic.com/v1".to_string()),
                model: env::var("ANTHROPIC_MODEL")
                    .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string()),
                max_tokens: 4000,
                temperature: 0.1,
            },
            "groq" => Self {
                api_key: env::var("GROQ_API_KEY").unwrap_or_default(),
                base_url: env::var("GROQ_BASE_URL")
                    .unwrap_or_else(|_| "https://api.groq.com/openai/v1".to_string()),
                model: env::var("GROQ_MODEL")
                    .unwrap_or_else(|_| "llama-3.1-70b-versatile".to_string()),
                max_tokens: 4000,
                temperature: 0.1,
            },
            _ => Self {
                api_key: env::var("OPENAI_API_KEY").unwrap_or_default(),
                base_url: env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                model: env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
                max_tokens: 4000,
                temperature: 0.1,
            },
        }
    }
}

/// AI analyzer using OpenAI-compatible endpoints
pub struct AiAnalyzer {
    config: AiProviderConfig,
    http_client: Client,
}

impl AiAnalyzer {
    pub fn new(config: AiProviderConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
        }
    }

    /// Analyze a program using AI
    pub async fn analyze_program(
        &self,
        program_data: &ProgramData,
        disassembly: &DisassemblyResult,
        semantic_analysis: Option<&SemanticAnalysisResult>,
    ) -> Result<AiAnalysisResult> {
        let start_time = Instant::now();
        info!(
            "🤖 Starting AI analysis for program: {}",
            program_data.address
        );

        // Build analysis prompt
        let prompt = self.build_analysis_prompt(program_data, disassembly, semantic_analysis);

        // Call AI provider
        let response = self.call_openai_compatible_api(&prompt).await?;

        // Parse response
        let result = self.parse_ai_response(&response, program_data)?;

        let processing_time = start_time.elapsed().as_millis() as u64;
        info!("✅ AI analysis completed in {}ms", processing_time);

        Ok(AiAnalysisResult {
            processing_time_ms: processing_time,
            ..result
        })
    }

    fn build_analysis_prompt(
        &self,
        program_data: &ProgramData,
        disassembly: &DisassemblyResult,
        semantic_analysis: Option<&SemanticAnalysisResult>,
    ) -> String {
        let mut prompt = format!(
            r#"Analyze this Solana BPF program for security risks and malicious behavior.

Program ID: {}
Executable: {}
Upgradeable: {}
Authority: {}
Data Size: {} bytes

Disassembly Analysis:
- Total Instructions: {}
- Suspicious Patterns: {}
- Entropy Score: {:.2}
- Complexity Score: {:.2}
"#,
            program_data.address,
            program_data.is_executable,
            program_data.is_upgradeable,
            program_data
                .authority
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
            program_data.executable_data.len(),
            disassembly.total_instructions,
            disassembly.suspicious_patterns.join(", "),
            disassembly.entropy_score,
            disassembly.complexity_score
        );

        if let Some(semantic) = semantic_analysis {
            prompt.push_str(&format!(
                r#"
Semantic Analysis:
- Control Flow Complexity: {:.2}
- Data Flow Risks: {}
- Syscall Patterns: {}
"#,
                semantic.control_flow_complexity,
                semantic.data_flow_risks.join(", "),
                semantic.syscall_patterns.join(", ")
            ));
        }

        prompt.push_str(
            r#"

Provide a security analysis in JSON format:
{
  "risk_score": 0-100,
  "risk_level": "VeryLow|Low|Medium|High|Critical",
  "behavioral_analysis": "detailed description of program behavior",
  "vulnerabilities": [
    {
      "severity": "Low|Medium|High|Critical",
      "category": "category name",
      "description": "what the vulnerability is",
      "remediation": "how to fix it"
    }
  ],
  "recommendations": ["recommendation 1", "recommendation 2"],
  "confidence_score": 0.0-1.0
}
"#,
        );

        prompt
    }

    async fn call_openai_compatible_api(&self, prompt: &str) -> Result<String> {
        if self.config.api_key.is_empty() {
            return Err(anyhow!("AI provider API key not set"));
        }

        let url = format!("{}/chat/completions", self.config.base_url);

        let payload = json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a security expert specializing in Solana program analysis. Analyze programs for malicious behavior, vulnerabilities, and security issues."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
        });

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to call AI provider: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!(
                "AI provider returned error {}: {}",
                status,
                error_text
            ));
        }

        let response_json: Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse AI response: {}", e))?;

        // Extract content from OpenAI-compatible response
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("No content in AI response"))?
            .to_string();

        Ok(content)
    }

    fn parse_ai_response(
        &self,
        response: &str,
        program_data: &ProgramData,
    ) -> Result<AiAnalysisResult> {
        // Try to extract JSON from response (might be wrapped in markdown)
        let json_str = if response.contains("```json") {
            response
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
                .trim()
        } else if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
                .trim()
        } else {
            response.trim()
        };

        let parsed: Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow!("Failed to parse AI JSON response: {}", e))?;

        let risk_score = parsed["risk_score"].as_f64().unwrap_or(50.0);
        let risk_level = parsed["risk_level"]
            .as_str()
            .unwrap_or("Medium")
            .to_string();
        let behavioral_analysis = parsed["behavioral_analysis"]
            .as_str()
            .unwrap_or("No analysis provided")
            .to_string();
        let confidence_score = parsed["confidence_score"].as_f64().unwrap_or(0.5);

        let mut vulnerabilities = Vec::new();
        if let Some(vulns) = parsed["vulnerabilities"].as_array() {
            for vuln in vulns {
                vulnerabilities.push(AiVulnerability {
                    severity: vuln["severity"].as_str().unwrap_or("Medium").to_string(),
                    category: vuln["category"].as_str().unwrap_or("Unknown").to_string(),
                    description: vuln["description"].as_str().unwrap_or("").to_string(),
                    remediation: vuln["remediation"].as_str().unwrap_or("").to_string(),
                });
            }
        }

        let mut recommendations = Vec::new();
        if let Some(recs) = parsed["recommendations"].as_array() {
            for rec in recs {
                if let Some(s) = rec.as_str() {
                    recommendations.push(s.to_string());
                }
            }
        }

        Ok(AiAnalysisResult {
            program_id: program_data.address.to_string(),
            risk_score,
            risk_level,
            behavioral_analysis,
            vulnerabilities,
            recommendations,
            confidence_score,
            model_used: self.config.model.clone(),
            analysis_timestamp: Utc::now(),
            processing_time_ms: 0, // Will be set by caller
        })
    }
}
