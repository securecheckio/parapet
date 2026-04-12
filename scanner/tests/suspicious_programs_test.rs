use chrono::Utc;
use parapet_scanner::classifier::*;

#[test]
fn test_known_program_identification() {
    // Core Solana programs should be known
    assert!(is_known_program("11111111111111111111111111111111"));
    assert!(is_known_program(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    ));
    assert!(is_known_program(
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    ));

    // Jupiter should be known
    assert!(is_known_program(
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"
    ));

    // Random programs should not be known
    assert!(!is_known_program("UnknownProgram1234567890123456789012"));
}

#[test]
fn test_risk_score_unknown_single_occurrence() {
    let score = calculate_program_risk_score("UnknownProgram1234567890123456789012", None, 1);
    // 20 (unknown) + 5 (single occurrence) = 25
    assert_eq!(score, 25);
}

#[test]
fn test_risk_score_unknown_frequent() {
    let score = calculate_program_risk_score("UnknownProgram1234567890123456789012", None, 15);
    // 20 (unknown) - 10 (frequent) = 10
    assert_eq!(score, 10);
}

#[test]
fn test_risk_score_known_program() {
    let score = calculate_program_risk_score(
        "11111111111111111111111111111111", // System program
        None,
        5,
    );
    // Known programs don't get the +20 penalty
    assert_eq!(score, 0);
}

#[test]
fn test_threat_type_classification() {
    // High risk unknown
    let threat_type = classify_threat_type(80, None, false);
    assert_eq!(threat_type, "high_risk_unknown");

    // Low risk unknown
    let threat_type = classify_threat_type(40, None, false);
    assert_eq!(threat_type, "unknown");

    // Known program
    let threat_type = classify_threat_type(40, None, true);
    assert_eq!(threat_type, "monitored");
}

#[test]
fn test_confidence_calculation() {
    // Base confidence for unknown programs
    let confidence = calculate_confidence(25, None, 1);
    assert_eq!(confidence, 0.5);

    // Higher confidence with more occurrences
    let confidence = calculate_confidence(25, None, 10);
    assert_eq!(confidence, 0.6);

    // Higher confidence with high risk score
    let confidence = calculate_confidence(80, None, 1);
    assert_eq!(confidence, 0.6);

    // Combined bonuses
    let confidence = calculate_confidence(80, None, 10);
    assert_eq!(confidence, 0.7);
}

#[test]
fn test_analysis_summary_generation() {
    let summary =
        generate_analysis_summary("UnknownProgram1234567890123456789012", 25, None, false, 1);
    assert!(summary.contains("Unknown program"));
    assert!(summary.contains("UnknownProgram1234567890123456789012"));
    assert!(summary.contains("1 transaction"));
    assert!(summary.contains("Risk score: 25"));
}

#[test]
fn test_recommendation_generation() {
    // High risk unknown
    let rec = generate_recommendation(80, None, false);
    assert!(rec.contains("high risk score"));
    assert!(rec.contains("Investigate"));

    // Low risk unknown
    let rec = generate_recommendation(40, None, false);
    assert!(rec.contains("not in the known safe list"));

    // Known program
    let rec = generate_recommendation(10, None, true);
    assert!(rec.contains("known and generally considered safe"));
}

#[test]
fn test_create_suspicious_program() {
    let program = create_suspicious_program(
        "UnknownProgram1234567890123456789012".to_string(),
        vec!["sig1".to_string(), "sig2".to_string()],
        Utc::now(),
        None,
    );

    assert_eq!(program.program_id, "UnknownProgram1234567890123456789012");
    assert_eq!(program.occurrence_count, 2);
    assert_eq!(program.transaction_signatures.len(), 2);
    assert!(program.risk_score > 0);
    assert!(program.confidence > 0.0);
    assert!(!program.analysis_summary.is_empty());
    assert!(!program.recommendation.is_empty());
}

#[test]
fn test_suspicious_program_sorting_by_risk() {
    let mut programs = vec![
        create_suspicious_program(
            "LowRisk1234567890123456789012345678".to_string(),
            vec!["sig1".to_string(); 20], // Many occurrences = lower risk
            Utc::now(),
            None,
        ),
        create_suspicious_program(
            "HighRisk1234567890123456789012345".to_string(),
            vec!["sig1".to_string()], // Single occurrence = higher risk
            Utc::now(),
            None,
        ),
    ];

    // Before sort
    assert!(programs[0].program_id.starts_with("LowRisk"));

    // Sort by risk score (highest first)
    programs.sort_by(|a, b| b.risk_score.cmp(&a.risk_score));

    // After sort, high risk should be first
    assert!(programs[0].program_id.starts_with("HighRisk"));
    assert!(programs[0].risk_score > programs[1].risk_score);
}
