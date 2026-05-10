// API Base URL
const API_BASE = '';

// State
let stages = [];
let currentResults = null;
let seenAlertIds = new Set();

// Initialize
document.addEventListener('DOMContentLoaded', async () => {
    await loadStages();
    await loadRules();
    startConsoleTime();
});

// Load stages from API
async function loadStages() {
    try {
        const response = await fetch(`${API_BASE}/api/stages`);
        stages = await response.json();
        renderStages();
        log('system', 'Loaded ' + stages.length + ' demo stages');
    } catch (error) {
        log('critical', 'Failed to load stages: ' + error.message);
    }
}

// Load rules from API
async function loadRules() {
    try {
        const response = await fetch(`${API_BASE}/api/rules`);
        const rules = await response.json();
        renderRules(rules);
        log('system', 'Loaded ' + rules.length + ' security rules');
    } catch (error) {
        log('critical', 'Failed to load rules: ' + error.message);
    }
}

// Render rules list
function renderRules(rules) {
    const container = document.getElementById('rules-list');
    const countEl = document.getElementById('rules-count');
    
    countEl.textContent = rules.length;
    
    container.innerHTML = rules.map(rule => `
        <div class="rule-item">
            <div class="rule-header-row">
                <div class="rule-name">${rule.name}</div>
                <div class="rule-action ${rule.rule.action}">${rule.rule.action}</div>
            </div>
            <div class="rule-description">${rule.description}</div>
            <div class="rule-conditions">
                <div class="conditions-label">Conditions:</div>
                <pre class="conditions-code">${JSON.stringify(rule.rule.conditions, null, 2)}</pre>
            </div>
            <div class="rule-message">${rule.rule.message}</div>
        </div>
    `).join('');
}

// Toggle rules section
function toggleRules() {
    const list = document.getElementById('rules-list');
    const toggle = document.getElementById('rules-toggle');
    
    if (list.style.display === 'none') {
        list.style.display = 'block';
        toggle.classList.add('expanded');
    } else {
        list.style.display = 'none';
        toggle.classList.remove('expanded');
    }
}

// Render stage cards
function renderStages() {
    const container = document.getElementById('stages-container');
    container.innerHTML = stages.map(stage => {
        // Show Solscan link for all real on-chain transactions
        const txSig = stage.tx_signature || '';
        const showLink = txSig && txSig.length > 0;
        const shortSig = txSig ? `${txSig.slice(0, 8)}...${txSig.slice(-8)}` : '';
        const solscanUrl = txSig ? `https://solscan.io/tx/${txSig}` : '';
        
        return `
        <div class="stage-card" id="stage-${stage.id}" onclick="fireStage(${stage.id})">
            <div class="stage-header">
                <div class="stage-number">${stage.id}</div>
                <div class="stage-date">${stage.date}</div>
            </div>
            <div class="stage-name">${stage.name}</div>
            <div class="stage-description">${stage.description}</div>
            ${showLink ? `
            <div class="stage-tx-link">
                <a href="${solscanUrl}" target="_blank" onclick="event.stopPropagation();" title="${txSig}">
                    ${shortSig} ↗
                </a>
            </div>
            ` : ''}
            <div class="stage-footer">
                <button class="btn-fire" onclick="fireStage(${stage.id}); event.stopPropagation();">
                    FIRE EVENT
                </button>
                <span class="stage-badge" id="badge-${stage.id}" style="display: none;"></span>
            </div>
        </div>
    `;
    }).join('');
}

// Fire stage event
async function fireStage(stageId) {
    const stageCard = document.getElementById(`stage-${stageId}`);
    const stage = stages.find(s => s.id === stageId);
    
    if (!stage) return;
    
    // Visual feedback
    stageCard.classList.add('loading');
    log('info', `Firing Stage ${stageId}: ${stage.name}`);
    
    try {
        // Call API
        const response = await fetch(`${API_BASE}/api/stage/fire`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ stage_id: stageId })
        });
        
        if (!response.ok) {
            throw new Error('API request failed');
        }
        
        const result = await response.json();
        
        // Update UI
        stageCard.classList.remove('loading');
        stageCard.classList.add('fired');
        
        // Show badge after firing (only for alerts/blocks)
        const badge = document.getElementById(`badge-${stageId}`);
        if (badge && result.parapet_result.action) {
            const action = result.parapet_result.action.toLowerCase();
            if (action === 'alert' || action === 'block') {
                badge.textContent = result.parapet_result.action.toUpperCase();
                badge.className = `stage-badge badge-${result.parapet_result.action}`;
                badge.style.display = 'inline-block';
            } else {
                badge.style.display = 'none';
            }
        }
        
        // Show results
        displayResults(result);
        
        // Log to console
        logStageResult(result);
        
    } catch (error) {
        stageCard.classList.remove('loading');
        log('critical', `Stage ${stageId} failed: ${error.message}`);
    }
}

// Display results
function displayResults(result) {
    currentResults = result;
    const container = document.getElementById('results-container');
    const stageName = document.getElementById('results-stage-name');
    
    stageName.textContent = result.stage.name;
    
    // Baseline result
    const baselineDiv = document.getElementById('baseline-result');
    const statusClass = getStatusClass(result.baseline_result.status);
    
    baselineDiv.innerHTML = `
        <div class="result-status ${statusClass}">
            <span>Status: ${result.baseline_result.status}</span>
        </div>
        <div class="result-detail">
            <strong>Detection:</strong> None - Transaction executed on-chain
        </div>
        <div class="result-detail">
            <strong>Response Time:</strong> N/A (No monitoring)
        </div>
    `;
    
    // Parapet result
    const parapetDiv = document.getElementById('parapet-result');
    const parapetStatusClass = getParapetStatusClass(result.parapet_result.action);
    
    // Update subtitle based on stage type
    const isPreSigning = result.stage.id === 1;
    const subtitleEl = document.getElementById('parapet-subtitle');
    if (subtitleEl) {
        subtitleEl.textContent = isPreSigning ? 'RPC Pre-Signing Analysis' : 'On-Chain Monitoring (Geyser/Helius)';
    }
    
    parapetDiv.innerHTML = `
        <div class="result-status ${parapetStatusClass}">
            <span>${result.parapet_result.action.toUpperCase()}</span>
        </div>
        <div class="result-detail" style="padding: 0.75rem; background: rgba(59, 130, 246, 0.1); border-radius: 0.375rem; border: 1px solid rgba(59, 130, 246, 0.3); margin-bottom: 0.75rem;">
            ${result.parapet_result.message}
        </div>
        <div class="result-detail">
            <strong>Risk Score:</strong> ${result.parapet_result.risk_score}/100
        </div>
        <div class="result-detail">
            <strong>Rules Triggered:</strong> ${result.parapet_result.rules_triggered.map(r => r.split('-').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ')).join(', ') || 'None'}
        </div>
        <div class="result-detail">
            <strong>Analysis Time:</strong> ${result.parapet_result.analysis_time_ms.toFixed(2)}ms
        </div>
    `;
    
    // Talking points
    const pointsList = document.getElementById('talking-points-list');
    pointsList.innerHTML = result.stage.talking_points
        .map(point => `<li>${point}</li>`)
        .join('');
    
    // Show container
    container.style.display = 'block';
    container.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
}

// Helper: Get status class
function getStatusClass(status) {
    if (status.includes('Executed')) return 'status-executed';
    if (status.includes('Signed')) return 'status-executed';
    return 'status-executed';
}

// Helper: Get Parapet status class
function getParapetStatusClass(action) {
    if (action === 'alert') return 'status-alert';
    return 'status-pass';
}

// Clear results
function clearResults() {
    document.getElementById('results-container').style.display = 'none';
    currentResults = null;
}

// Log to console
function log(level, message) {
    const output = document.getElementById('console-output');
    const timestamp = new Date().toISOString().substr(11, 8);
    const line = document.createElement('div');
    line.className = `console-line ${level}`;
    line.textContent = `[${timestamp}] ${message}`;
    output.appendChild(line);
    output.scrollTop = output.scrollHeight;
}

// Log stage result
function logStageResult(result) {
    const stage = result.stage;
    const parapet = result.parapet_result;
    const baseline = result.baseline_result;
    
    log('info', '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
    log('info', `Stage ${stage.id}: ${stage.name}`);
    log('system', `Baseline: ${baseline.status}`);
    
    if (parapet.action === 'alert') {
        log('alert', `⚠️ ALERT: Risk ${parapet.risk_score}/100 - ${parapet.rules_triggered.length} rules triggered`);
        log('alert', parapet.message);
        
        // Show Squads V4 analyzer output if available
        if (parapet.analyzer_fields) {
            const fields = parapet.analyzer_fields;
            
            // Check for Squads V4 transaction
            if (fields['squads_v4:has_vault_transaction_execute'] === true) {
                log('info', '');
                log('info', '📋 DECODED TRANSACTION ANALYSIS:');
                log('critical', '  • Program: Squads V4 Multisig');
                log('critical', '  • Instruction: vault_transaction_execute');
                log('alert', '    Executes a pre-approved transaction from the vault');
                log('alert', '    Contains embedded instructions (e.g., admin transfer to unknown address)');
                
                if (fields['squads_v4:sets_timelock_to_zero'] === true) {
                    log('critical', '  • Sets timelock to: ZERO (removes governance delay!)');
                }
                
                log('info', '  ℹ️ Hardware wallet shows: "AdvanceNonceAccount + Unknown Instruction"');
                log('info', '  ✅ Parapet decoded: Squads vault execution (high risk when pre-signed)');
            }
        }
    } else {
        log('info', `PASS: Risk ${parapet.risk_score}/100 - Transaction safe`);
    }
    
    log('system', `Analysis completed in ${parapet.analysis_time_ms.toFixed(2)}ms`);
    log('info', '━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━');
}

// Clear console
function clearConsole() {
    const output = document.getElementById('console-output');
    output.innerHTML = '<div class="console-line system">Console cleared. Ready for next stage...</div>';
    // Clear seen alerts when console is cleared
    seenAlertIds.clear();
}

// Start console time display
function startConsoleTime() {
    setInterval(() => {
        const now = new Date().toISOString();
        // Update would go here if needed
    }, 1000);
}

// Poll for alerts (optional - for real-time monitoring)
async function pollAlerts() {
    try {
        const response = await fetch(`${API_BASE}/api/alerts`);
        const alerts = await response.json();
        
        // Process only new alerts (not already seen)
        alerts.forEach(alert => {
            // Create unique ID from stage_id + timestamp
            const alertId = `${alert.stage_id}-${alert.timestamp}`;
            
            if (!seenAlertIds.has(alertId)) {
                seenAlertIds.add(alertId);
                
                if (alert.alert_type === 'attack_detected') {
                    log('critical', alert.message);
                } else if (alert.alert_type === 'nonce_creation_detected') {
                    log('alert', alert.message);
                } else if (alert.alert_type === 'coordinated_attack') {
                    log('critical', alert.message);
                }
            }
        });
    } catch (error) {
        // Silently fail
    }
}

// Don't poll alerts automatically - only show alerts when user fires events
// Alerts will be generated by the fire stage API and logged via logStageResult
// setInterval(pollAlerts, 5000);
