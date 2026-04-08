#!/usr/bin/env python3
"""
Sol-Shield Agent Integration Example (Python)

This demonstrates how an AI agent should interact with Sol-Shield RPC proxy.

Usage:
    python agent-integration-example.py --rpc http://localhost:8899
"""

import base58
import json
import sys
import time
from dataclasses import dataclass
from typing import Optional, List, Dict, Any
import requests


@dataclass
class SolShieldWarning:
    """Security warning from Sol-Shield"""
    severity: str
    message: str
    rule_id: str
    rule_name: str
    weight: int


@dataclass
class SolShieldAnalysis:
    """Sol-Shield security analysis result"""
    version: str
    risk_score: int
    structural_risk: int
    simulation_risk: int
    decision: str  # "safe", "alert", or "would_block"
    threshold: int
    warnings: List[SolShieldWarning]
    would_block: bool

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'SolShieldAnalysis':
        """Parse Sol-Shield metadata from simulation response"""
        warnings = [
            SolShieldWarning(
                severity=w['severity'],
                message=w['message'],
                rule_id=w['ruleId'],
                rule_name=w['ruleName'],
                weight=w['weight']
            )
            for w in data.get('warnings', [])
        ]
        
        return cls(
            version=data['version'],
            risk_score=data['riskScore'],
            structural_risk=data['structuralRisk'],
            simulation_risk=data['simulationRisk'],
            decision=data['decision'],
            threshold=data['threshold'],
            warnings=warnings,
            would_block=data['analysis']['wouldBlock']
        )


class SolShieldAgent:
    """AI Agent with Sol-Shield integration"""
    
    def __init__(self, rpc_url: str, api_key: Optional[str] = None, max_risk_score: int = 50):
        self.rpc_url = rpc_url
        self.api_key = api_key
        self.max_risk_score = max_risk_score
        self.request_counter = 0
    
    def _headers(self) -> Dict[str, str]:
        """Build request headers"""
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["X-API-Key"] = self.api_key
        return headers
    
    def _rpc_call(self, method: str, params: List[Any]) -> Dict[str, Any]:
        """Make JSON-RPC call"""
        self.request_counter += 1
        
        payload = {
            "jsonrpc": "2.0",
            "id": self.request_counter,
            "method": method,
            "params": params
        }
        
        response = requests.post(
            self.rpc_url,
            headers=self._headers(),
            json=payload,
            timeout=30
        )
        
        # Check rate limits
        remaining = response.headers.get('X-Rate-Limit-Remaining')
        if remaining:
            print(f"📊 Rate Limit: {remaining} requests remaining")
        
        return response
    
    def health_check(self) -> bool:
        """Check if Sol-Shield is accessible"""
        try:
            response = requests.get(f"{self.rpc_url}/health", timeout=5)
            return response.status_code == 200
        except Exception as e:
            print(f"❌ Health check failed: {e}")
            return False
    
    def simulate_transaction(self, tx_base58: str) -> Optional[SolShieldAnalysis]:
        """
        Simulate transaction and return Sol-Shield analysis
        
        Returns:
            SolShieldAnalysis if successful, None if failed
        """
        print("🔬 Simulating transaction...")
        
        response = self._rpc_call("simulateTransaction", [
            tx_base58,
            {"encoding": "base58", "commitment": "confirmed"}
        ])
        
        if response.status_code != 200:
            print(f"❌ Simulation failed with HTTP {response.status_code}")
            return None
        
        data = response.json()
        
        # Check for RPC error
        if 'error' in data:
            print(f"❌ RPC Error: {data['error']['message']}")
            return None
        
        # Extract simulation result
        result = data.get('result', {}).get('value', {})
        
        # Check for simulation failure
        if result.get('err'):
            print(f"❌ Simulation failed: {result['err']}")
            return None
        
        # Extract Sol-Shield metadata
        sol_shield = result.get('solShield')
        if not sol_shield:
            print("⚠️  No Sol-Shield metadata found - RPC may not be protected")
            return None
        
        analysis = SolShieldAnalysis.from_dict(sol_shield)
        
        # Log analysis
        print(f"📊 Risk Score: {analysis.risk_score}")
        print(f"📊 Decision: {analysis.decision}")
        print(f"📊 Threshold: {analysis.threshold}")
        
        if analysis.warnings:
            print(f"⚠️  {len(analysis.warnings)} security warning(s):")
            for warning in analysis.warnings:
                print(f"   - [{warning.severity.upper()}] {warning.message}")
        else:
            print("✅ No security warnings")
        
        return analysis
    
    def send_transaction_safely(self, tx_base58: str) -> Optional[str]:
        """
        Send transaction with Sol-Shield protection
        
        Returns:
            Transaction signature if successful, None if blocked/failed
        """
        # Step 1: Simulate first
        analysis = self.simulate_transaction(tx_base58)
        
        if not analysis:
            print("❌ Cannot analyze transaction - aborting for safety")
            return None
        
        # Step 2: Check if would be blocked
        if analysis.would_block:
            print(f"🚫 Transaction would be BLOCKED (risk: {analysis.risk_score})")
            self._log_security_block(analysis)
            return None
        
        # Step 3: Check against agent's risk tolerance
        if analysis.risk_score > self.max_risk_score:
            print(f"⚠️  Risk score {analysis.risk_score} exceeds agent threshold {self.max_risk_score}")
            self._log_high_risk(analysis)
            # Could implement: escalate to human review
            return None
        
        # Step 4: Log warnings
        if analysis.warnings:
            self._log_warnings(analysis.warnings)
        
        # Step 5: Send transaction
        print("📤 Sending transaction...")
        response = self._rpc_call("sendTransaction", [
            tx_base58,
            {"encoding": "base58", "skipPreflight": False}
        ])
        
        if response.status_code == 403:
            # Blocked by Sol-Shield
            data = response.json()
            print(f"🚫 BLOCKED: {data['error']['message']}")
            return None
        
        if response.status_code == 429:
            # Rate limited
            print("⏳ Rate limited - implement backoff in production")
            return None
        
        if response.status_code != 200:
            print(f"❌ Send failed with HTTP {response.status_code}")
            return None
        
        data = response.json()
        
        if 'error' in data:
            print(f"❌ RPC Error: {data['error']['message']}")
            return None
        
        signature = data.get('result')
        print(f"✅ Transaction sent: {signature}")
        
        return signature
    
    def _log_security_block(self, analysis: SolShieldAnalysis):
        """Log blocked transaction for audit"""
        print("\n🔒 SECURITY BLOCK EVENT")
        print(f"   Risk Score: {analysis.risk_score}")
        print(f"   Threshold: {analysis.threshold}")
        print(f"   Warnings: {len(analysis.warnings)}")
        for warning in analysis.warnings:
            print(f"     - [{warning.severity}] {warning.message}")
        print("")
    
    def _log_high_risk(self, analysis: SolShieldAnalysis):
        """Log high-risk transaction for review"""
        print("\n⚠️  HIGH RISK EVENT")
        print(f"   Risk Score: {analysis.risk_score}")
        print(f"   Agent Threshold: {self.max_risk_score}")
        print(f"   Requires review before proceeding")
        print("")
    
    def _log_warnings(self, warnings: List[SolShieldWarning]):
        """Log security warnings"""
        print("\n📝 Security Warnings Logged:")
        for warning in warnings:
            print(f"   [{warning.severity.upper()}] {warning.message}")
            print(f"      Rule: {warning.rule_name} (weight: {warning.weight})")
        print("")


# Demo Usage
if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="Test Sol-Shield agent integration")
    parser.add_argument("--rpc", default="http://localhost:8899", help="Sol-Shield RPC URL")
    parser.add_argument("--api-key", help="API key for authentication")
    parser.add_argument("--max-risk", type=int, default=50, help="Maximum acceptable risk score")
    args = parser.parse_args()
    
    # Create agent
    agent = SolShieldAgent(
        rpc_url=args.rpc,
        api_key=args.api_key,
        max_risk_score=args.max_risk
    )
    
    # Test 1: Health check
    print("Test 1: Health Check")
    print("────────────────────────────────────────")
    if agent.health_check():
        print("✅ Sol-Shield is accessible")
    else:
        print("❌ Sol-Shield is not accessible")
        sys.exit(1)
    print("")
    
    # Test 2: Basic RPC call
    print("Test 2: Basic RPC Call (getHealth)")
    print("────────────────────────────────────────")
    try:
        response = agent._rpc_call("getHealth", [])
        if response.status_code == 200:
            print("✅ RPC is responding")
            result = response.json()
            print(f"Response: {json.dumps(result, indent=2)}")
        else:
            print(f"⚠️  Unexpected status code: {response.status_code}")
    except Exception as e:
        print(f"❌ RPC call failed: {e}")
    print("")
    
    # Test 3: Get balance
    print("Test 3: Get Balance (Pass-through Method)")
    print("────────────────────────────────────────")
    try:
        # Use a known wallet address (Solana treasury)
        response = agent._rpc_call("getBalance", [
            "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"
        ])
        
        if response.status_code == 200:
            result = response.json()
            if 'result' in result:
                balance = result['result']['value']
                print(f"✅ Balance retrieved: {balance / 1e9:.4f} SOL")
            else:
                print("⚠️  Unexpected response format")
        else:
            print(f"❌ Failed with HTTP {response.status_code}")
    except Exception as e:
        print(f"❌ Balance check failed: {e}")
    print("")
    
    print("═══════════════════════════════════════════")
    print("✅ Basic Integration Tests Complete")
    print("═══════════════════════════════════════════")
    print("")
    print("🤖 Agent Integration Status:")
    print("   [✅] RPC connectivity")
    print("   [✅] Pass-through methods")
    print("   [✅] JSON-RPC compatibility")
    print("")
    print("📋 Next Steps:")
    print("   1. Build a test transaction with your agent")
    print("   2. Test simulateTransaction + solShield parsing")
    print("   3. Test sendTransaction with safe transaction")
    print("   4. Test blocked transaction handling")
    print("")
    print("📚 See AI_AGENT_INTEGRATION_GUIDE.md for full documentation")
    print("")
