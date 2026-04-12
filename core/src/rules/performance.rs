use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Performance metrics for a single rule evaluation
#[derive(Debug, Clone)]
pub struct RulePerformanceMetrics {
    pub rule_id: String,
    pub rule_name: String,
    pub evaluation_count: u64,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub avg_duration: Duration,
    pub last_evaluation: Option<Instant>,
}

impl RulePerformanceMetrics {
    pub fn new(rule_id: String, rule_name: String) -> Self {
        Self {
            rule_id,
            rule_name,
            evaluation_count: 0,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            avg_duration: Duration::ZERO,
            last_evaluation: None,
        }
    }

    pub fn record(&mut self, duration: Duration) {
        self.evaluation_count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        self.avg_duration = self.total_duration / self.evaluation_count as u32;
        self.last_evaluation = Some(Instant::now());
    }
}

/// Performance metrics for analyzer execution
#[derive(Debug, Clone)]
pub struct AnalyzerPerformanceMetrics {
    pub analyzer_name: String,
    pub execution_count: u64,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub avg_duration: Duration,
    pub last_execution: Option<Instant>,
}

impl AnalyzerPerformanceMetrics {
    pub fn new(analyzer_name: String) -> Self {
        Self {
            analyzer_name,
            execution_count: 0,
            total_duration: Duration::ZERO,
            min_duration: Duration::MAX,
            max_duration: Duration::ZERO,
            avg_duration: Duration::ZERO,
            last_execution: None,
        }
    }

    pub fn record(&mut self, duration: Duration) {
        self.execution_count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        self.avg_duration = self.total_duration / self.execution_count as u32;
        self.last_execution = Some(Instant::now());
    }
}

/// Aggregated performance metrics for the entire rule engine
#[derive(Debug, Clone)]
pub struct EnginePerformanceMetrics {
    pub total_evaluations: u64,
    pub total_duration: Duration,
    pub avg_evaluation_time: Duration,
    pub rule_metrics: HashMap<String, RulePerformanceMetrics>,
    pub analyzer_metrics: HashMap<String, AnalyzerPerformanceMetrics>,
}

impl EnginePerformanceMetrics {
    pub fn new() -> Self {
        Self {
            total_evaluations: 0,
            total_duration: Duration::ZERO,
            avg_evaluation_time: Duration::ZERO,
            rule_metrics: HashMap::new(),
            analyzer_metrics: HashMap::new(),
        }
    }

    /// Get slowest rules sorted by average duration
    pub fn slowest_rules(&self, limit: usize) -> Vec<&RulePerformanceMetrics> {
        let mut rules: Vec<&RulePerformanceMetrics> = self.rule_metrics.values().collect();
        rules.sort_by(|a, b| b.avg_duration.cmp(&a.avg_duration));
        rules.into_iter().take(limit).collect()
    }

    /// Get slowest analyzers sorted by average duration
    pub fn slowest_analyzers(&self, limit: usize) -> Vec<&AnalyzerPerformanceMetrics> {
        let mut analyzers: Vec<&AnalyzerPerformanceMetrics> =
            self.analyzer_metrics.values().collect();
        analyzers.sort_by(|a, b| b.avg_duration.cmp(&a.avg_duration));
        analyzers.into_iter().take(limit).collect()
    }

    /// Format performance report as string
    pub fn format_report(&self) -> String {
        let mut report = String::new();

        report.push_str("=== Rule Engine Performance Report ===\n\n");

        report.push_str(&format!("Total Evaluations: {}\n", self.total_evaluations));
        report.push_str(&format!("Total Duration: {:?}\n", self.total_duration));
        report.push_str(&format!(
            "Avg Evaluation Time: {:?}\n\n",
            self.avg_evaluation_time
        ));

        report.push_str("Top 10 Slowest Rules:\n");
        report.push_str("---------------------\n");
        for (i, metrics) in self.slowest_rules(10).iter().enumerate() {
            report.push_str(&format!(
                "{}. {} ({})\n   Avg: {:?} | Min: {:?} | Max: {:?} | Count: {}\n",
                i + 1,
                metrics.rule_name,
                metrics.rule_id,
                metrics.avg_duration,
                metrics.min_duration,
                metrics.max_duration,
                metrics.evaluation_count
            ));
        }

        report.push_str("\nTop 10 Slowest Analyzers:\n");
        report.push_str("-------------------------\n");
        for (i, metrics) in self.slowest_analyzers(10).iter().enumerate() {
            report.push_str(&format!(
                "{}. {}\n   Avg: {:?} | Min: {:?} | Max: {:?} | Count: {}\n",
                i + 1,
                metrics.analyzer_name,
                metrics.avg_duration,
                metrics.min_duration,
                metrics.max_duration,
                metrics.execution_count
            ));
        }

        report
    }
}

impl Default for EnginePerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance tracker for rule engine
pub struct PerformanceTracker {
    metrics: Arc<RwLock<EnginePerformanceMetrics>>,
    enabled: bool,
}

impl PerformanceTracker {
    pub fn new(enabled: bool) -> Self {
        Self {
            metrics: Arc::new(RwLock::new(EnginePerformanceMetrics::new())),
            enabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Start timing a rule evaluation
    pub fn start_rule(&self, rule_id: &str) -> Option<RuleTimer> {
        if !self.enabled {
            return None;
        }
        Some(RuleTimer {
            rule_id: rule_id.to_string(),
            start: Instant::now(),
        })
    }

    /// Start timing an analyzer execution
    pub fn start_analyzer(&self, analyzer_name: &str) -> Option<AnalyzerTimer> {
        if !self.enabled {
            return None;
        }
        Some(AnalyzerTimer {
            analyzer_name: analyzer_name.to_string(),
            start: Instant::now(),
        })
    }

    /// Record rule evaluation time
    pub async fn record_rule(&self, rule_id: String, rule_name: String, duration: Duration) {
        if !self.enabled {
            return;
        }

        let mut metrics = self.metrics.write().await;
        let rule_metrics = metrics
            .rule_metrics
            .entry(rule_id.clone())
            .or_insert_with(|| RulePerformanceMetrics::new(rule_id, rule_name));
        rule_metrics.record(duration);
    }

    /// Record analyzer execution time
    pub async fn record_analyzer(&self, analyzer_name: String, duration: Duration) {
        if !self.enabled {
            return;
        }

        let mut metrics = self.metrics.write().await;
        let analyzer_metrics = metrics
            .analyzer_metrics
            .entry(analyzer_name.clone())
            .or_insert_with(|| AnalyzerPerformanceMetrics::new(analyzer_name));
        analyzer_metrics.record(duration);
    }

    /// Record total evaluation time
    pub async fn record_evaluation(&self, duration: Duration) {
        if !self.enabled {
            return;
        }

        let mut metrics = self.metrics.write().await;
        metrics.total_evaluations += 1;
        metrics.total_duration += duration;
        metrics.avg_evaluation_time = metrics.total_duration / metrics.total_evaluations as u32;
    }

    /// Get current metrics snapshot
    pub async fn get_metrics(&self) -> EnginePerformanceMetrics {
        self.metrics.read().await.clone()
    }

    /// Reset all metrics
    pub async fn reset(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = EnginePerformanceMetrics::new();
    }

    /// Get formatted performance report
    pub async fn get_report(&self) -> String {
        let metrics = self.metrics.read().await;
        metrics.format_report()
    }
}

impl Clone for PerformanceTracker {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
            enabled: self.enabled,
        }
    }
}

/// Timer for rule evaluation
pub struct RuleTimer {
    rule_id: String,
    start: Instant,
}

impl RuleTimer {
    pub fn finish(self, tracker: &PerformanceTracker, rule_name: String) -> Duration {
        let duration = self.start.elapsed();
        let rule_id = self.rule_id;
        let tracker = tracker.clone();

        // Spawn async task to record metrics without blocking
        tokio::spawn(async move {
            tracker.record_rule(rule_id, rule_name, duration).await;
        });

        duration
    }
}

/// Timer for analyzer execution
pub struct AnalyzerTimer {
    analyzer_name: String,
    start: Instant,
}

impl AnalyzerTimer {
    pub fn finish(self, tracker: &PerformanceTracker) -> Duration {
        let duration = self.start.elapsed();
        let analyzer_name = self.analyzer_name;
        let tracker = tracker.clone();

        // Spawn async task to record metrics without blocking
        tokio::spawn(async move {
            tracker.record_analyzer(analyzer_name, duration).await;
        });

        duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_performance_tracker() {
        let tracker = PerformanceTracker::new(true);

        // Record some rule evaluations
        tracker
            .record_rule(
                "rule1".to_string(),
                "Test Rule 1".to_string(),
                Duration::from_millis(10),
            )
            .await;
        tracker
            .record_rule(
                "rule1".to_string(),
                "Test Rule 1".to_string(),
                Duration::from_millis(20),
            )
            .await;
        tracker
            .record_rule(
                "rule2".to_string(),
                "Test Rule 2".to_string(),
                Duration::from_millis(5),
            )
            .await;

        // Record some analyzer executions
        tracker
            .record_analyzer("analyzer1".to_string(), Duration::from_millis(15))
            .await;
        tracker
            .record_analyzer("analyzer1".to_string(), Duration::from_millis(25))
            .await;

        // Record total evaluations
        tracker.record_evaluation(Duration::from_millis(50)).await;
        tracker.record_evaluation(Duration::from_millis(60)).await;

        let metrics = tracker.get_metrics().await;

        assert_eq!(metrics.total_evaluations, 2);
        assert_eq!(metrics.rule_metrics.len(), 2);
        assert_eq!(metrics.analyzer_metrics.len(), 1);

        let rule1 = metrics.rule_metrics.get("rule1").unwrap();
        assert_eq!(rule1.evaluation_count, 2);
        assert_eq!(rule1.avg_duration, Duration::from_millis(15));
    }

    #[tokio::test]
    async fn test_disabled_tracker() {
        let tracker = PerformanceTracker::new(false);

        tracker
            .record_rule(
                "rule1".to_string(),
                "Test Rule".to_string(),
                Duration::from_millis(10),
            )
            .await;

        let metrics = tracker.get_metrics().await;
        assert_eq!(metrics.total_evaluations, 0);
        assert_eq!(metrics.rule_metrics.len(), 0);
    }

    #[test]
    fn test_slowest_rules() {
        let mut metrics = EnginePerformanceMetrics::new();

        let mut rule1 = RulePerformanceMetrics::new("rule1".to_string(), "Fast Rule".to_string());
        rule1.record(Duration::from_millis(5));

        let mut rule2 = RulePerformanceMetrics::new("rule2".to_string(), "Slow Rule".to_string());
        rule2.record(Duration::from_millis(50));

        metrics.rule_metrics.insert("rule1".to_string(), rule1);
        metrics.rule_metrics.insert("rule2".to_string(), rule2);

        let slowest = metrics.slowest_rules(1);
        assert_eq!(slowest[0].rule_id, "rule2");
    }
}
