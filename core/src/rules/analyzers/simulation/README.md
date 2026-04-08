# Simulation Analyzers

Analyzers that require simulation results. Only available when using `simulateTransaction` or analyzing confirmed transactions.

## Available Analyzers

### 1. SimulationBalanceAnalyzer

Analyzes SOL balance changes from simulation.

**Fields:**

- `simulation_balance:sol_change` - Net SOL change (lamports)
- `simulation_balance:sol_increase` - SOL gained
- `simulation_balance:sol_decrease` - SOL lost
- `simulation_balance:has_sol_drain` - Large SOL outflow detected

**Risks:** Unexpected balance drains, hidden SOL transfers

### 2. SimulationTokenAnalyzer

Analyzes token balance changes from simulation.

**Fields:**

- `simulation_token:tokens_changed` - Array of tokens with balance changes
- `simulation_token:token_count` - Number of tokens affected
- `simulation_token:has_token_drain` - Token balance decrease detected
- `simulation_token:largest_change_pct` - Largest percentage change

**Risks:** Hidden token transfers, unexpected token movements

### 3. SimulationCpiAnalyzer

Analyzes CPI patterns from simulation results.

**Fields:**

- `simulation_cpi:cpi_count` - Number of CPIs executed
- `simulation_cpi:cpi_depth` - Maximum CPI nesting depth (use with `>` operator in rules)
- `simulation_cpi:programs_called` - Array of programs called via CPI

**Risks:** Deep CPI chains, CPI-based exploits

### 4. SimulationComputeAnalyzer

Analyzes compute unit usage.

**Fields:**

- `simulation_compute:units_consumed` - Compute units used
- `simulation_compute:units_requested` - Compute units requested
- `simulation_compute:efficiency_pct` - Usage efficiency percentage
- `simulation_compute:has_high_usage` - Near compute limit

**Risks:** Compute exhaustion attacks, resource abuse

### 5. SimulationFailureAnalyzer

Analyzes simulation failures.

**Fields:**

- `simulation_failure:failed` - Simulation failed
- `simulation_failure:error_type` - Type of error
- `simulation_failure:error_message` - Error message
- `simulation_failure:is_intentional` - Likely intentional failure

**Risks:** Intentional failure patterns, error-based exploits

### 6. SimulationLogAnalyzer

Analyzes logs from simulation.

**Fields:**

- `simulation_logs:log_count` - Number of log entries
- `simulation_logs:has_errors` - Error logs present
- `simulation_logs:suspicious_patterns` - Array of suspicious patterns
- `simulation_logs:program_logs` - Logs by program

**Risks:** Log-based threat detection, suspicious program behavior

## Usage

Simulation analyzers are automatically available when:

1. Using `simulateTransaction` RPC method
2. Analyzing confirmed transactions with metadata
3. Scanner analyzing historical transactions

## Performance

**Latency:** <1ms (pure analysis of simulation results, no external calls)

**Note:** Simulation itself adds ~100-500ms latency depending on transaction complexity.

## Example Rules

**Block Large SOL Drains:**

```json
{
  "action": "block",
  "conditions": {
    "all": [
      {"field": "simulation_balance:has_sol_drain", "operator": "equals", "value": true},
      {"field": "simulation_balance:sol_decrease", "operator": "greater_than", "value": 1000000000}
    ]
  },
  "message": "Large SOL drain detected in simulation"
}
```

**Alert on Deep CPIs:**

```json
{
  "action": "alert",
  "conditions": {
    "field": "simulation_cpi:cpi_depth",
    "operator": "greater_than",
    "value": 3
  },
  "message": "Deep CPI chain detected (>3 levels)"
}
```

**Block High Compute Usage:**

```json
{
  "action": "block",
  "conditions": {
    "field": "simulation_compute:units_consumed",
    "operator": "greater_than",
    "value": 1200000
  },
  "message": "Compute limit exceeded"
}
```

## Limitations

- Not available for pre-send analysis without simulation
- Adds latency (simulation time)
- Requires RPC with simulation support
- May have different results than actual execution

## Testing

Unit tests: `src/rules/analyzers/simulation/tests.rs`
Integration tests: `tests/integration/simulation_analyzers.rs`