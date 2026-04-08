pub mod basic;
pub mod canonical_tx;
pub mod core_security;
pub mod inner_instruction;
pub mod instruction_data;
pub mod instruction_padding;
pub mod program_complexity;
pub mod system_program;
pub mod token_instructions;
pub mod transaction_logs;

#[cfg(test)]
mod tests_basic;

#[cfg(test)]
mod tests_security;

#[cfg(test)]
mod tests_system;

#[cfg(test)]
mod tests_token;

pub use basic::BasicAnalyzer;
pub use canonical_tx::CanonicalTransactionAnalyzer;
pub use core_security::CoreSecurityAnalyzer;
pub use inner_instruction::InnerInstructionAnalyzer;
pub use instruction_data::InstructionDataAnalyzer;
pub use instruction_padding::InstructionPaddingAnalyzer;
pub use program_complexity::ProgramComplexityAnalyzer;
pub use system_program::SystemProgramAnalyzer;
pub use token_instructions::TokenInstructionAnalyzer;
pub use transaction_logs::TransactionLogAnalyzer;
