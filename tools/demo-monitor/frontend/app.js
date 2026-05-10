const API_BASE = '';

let stages = [];
let loadedRules = [];
const seenAlertIds = new Set();
const resultsByStageId = {};
const expandedStages = new Set();

document.addEventListener('DOMContentLoaded', async () => {
    await loadStages();
    await loadRules();
});

function escapeHtml(s) {
    if (s === null || s === undefined) return '';
    const d = document.createElement('div');
    d.textContent = String(s);
    return d.innerHTML;
}

function isDriftTeamStage(stageId) {
    return stageId === 1 || stageId === 2;
}

function stageTitleWithoutDate(stage) {
    const d = (stage.date || '').trim();
    let n = stage.name || '';
    if (d && n.startsWith(d)) {
        n = n.slice(d.length).replace(/^\s*[:\u2013\u2014\-]\s*/, '').trim();
    }
    return n || stage.name || '';
}

function fieldMapFromParapet(parapet) {
    const raw = parapet.analyzer_fields;
    if (!raw || typeof raw !== 'object') return {};
    return { ...raw };
}

function boolField(fields, key) {
    return fields[key] === true || fields[key] === 'true';
}

function riskLabel(score) {
    if (score >= 200) return 'CRITICAL';
    if (score >= 100) return 'HIGH';
    if (score >= 50) return 'MEDIUM';
    return 'LOW';
}

function technicalAppendixHtml(stageId, result) {
    const fields = fieldMapFromParapet(result.parapet_result);
    const p = result.parapet_result;
    const rulesHit = Array.isArray(p.rules_triggered) ? p.rules_triggered : [];
    let ruleBlock = '';
    for (const id of rulesHit) {
        const rec = ruleRecordById(id);
        ruleBlock +=
            '<p class="rule-h">' +
            escapeHtml(id) +
            '</p>';
        if (rec && rec.rule && rec.rule.conditions) {
            ruleBlock +=
                '<pre class="mono-block">' +
                escapeHtml(JSON.stringify(rec.rule.conditions, null, 2)) +
                '</pre>';
        }
    }
    return (
        '<details class="tech-fold">' +
        '<summary>Raw fields · conditions</summary>' +
        '<pre class="mono-block">' +
        escapeHtml(JSON.stringify(fields)) +
        '</pre>' +
        ruleBlock +
        '</details>'
    );
}

function ruleRecordById(ruleId) {
    return loadedRules.find(r => r.id === ruleId) || null;
}

/** Team stages: signing device vs decoded/replay. */
function compareTeamHtml(stageId, fieldsOrNull, parapetResultOrNull) {
    let leftContent;
    if (stageId === 1) {
        leftContent =
            '<p class="compare-mono">Advance nonce</p>' +
            '<p class="compare-mono">Unknown instruction</p>';
    } else {
        leftContent =
            '<p class="compare-mono">Multisig proposal</p>' +
            '<p class="compare-mono">Vault execute</p>';
    }
    const left =
        '<div class="compare-cell compare-signing">' +
        '<span class="compare-label">Device</span>' +
        leftContent +
        '</div>';
    let rightInner;
    if (!parapetResultOrNull) {
        rightInner = '<p class="muted pending-hint">Analyze</p>';
    } else {
        const fields = fieldsOrNull || {};
        const bits = [];
        if (boolField(fields, 'squads_v4:has_proposal_create')) bits.push('proposal_create');
        if (boolField(fields, 'squads_v4:has_proposal_approve')) bits.push('proposal_approve');
        if (boolField(fields, 'squads_v4:has_vault_transaction_execute')) bits.push('vault_execute');
        const p = parapetResultOrNull;
        const decoded = bits.length ? bits.join(' · ') : '—';
        rightInner =
            '<p class="compare-decode">' +
            escapeHtml(decoded) +
            '</p>' +
            '<p class="compare-verdict">' +
            escapeHtml(p.action.toUpperCase() + ' · ' + riskLabel(p.risk_score)) +
            '</p>';
    }
    const right =
        '<div class="compare-cell compare-engine">' +
        '<span class="compare-label">Parapet</span>' +
        rightInner +
        '</div>';
    return '<div class="compare-row">' + left + right + '</div>';
}

function compareAttackerHtml(result) {
    const p = result.parapet_result;
    const b = result.baseline_result;
    return (
        '<div class="compare-row compare-row-simple">' +
        '<p><span class="muted">Baseline</span> · ' +
        escapeHtml(b.status) +
        '</p>' +
        '<p><span class="muted">Geyser (real-time)</span> · ' +
        escapeHtml(p.action.toUpperCase() + ' · ' + riskLabel(p.risk_score)) +
        '</p>' +
        '</div>'
    );
}

function verdictLineHtml(stage, result) {
    const p = result.parapet_result;
    const msg = (p.message || '').trim();
    const short =
        msg.length > 140 ? escapeHtml(msg.slice(0, 137)) + '…' : escapeHtml(msg || '—');
    const sig = stage.tx_signature || '';
    const tx =
        sig.length > 0
            ? '<a class="tx-link" href="' +
              escapeHtml('https://solscan.io/tx/' + encodeURIComponent(sig)) +
              '" target="_blank" rel="noopener">' +
              escapeHtml(sig.slice(0, 10) + '…') +
              '</a>'
            : '';
    return (
        '<div class="verdict-block">' +
        '<p class="verdict-msg">' +
        short +
        '</p>' +
        tx +
        '</div>'
    );
}

function renderTechnicalPanel(stageId) {
    const wrap = document.getElementById('stage-details-' + stageId);
    if (!wrap) return;
    const stage = stages.find(s => s.id === stageId);
    if (!stage) return;

    const open = expandedStages.has(stageId);
    wrap.className = 'stage-body' + (open ? ' is-open' : '');
    wrap.innerHTML = '';
    const chev = document.getElementById('chev-' + stageId);
    if (chev) chev.textContent = open ? '▾' : '▸';
    if (!open) return;

    const result = resultsByStageId[stageId];

    if (!result) {
        wrap.innerHTML = isDriftTeamStage(stageId)
            ? compareTeamHtml(stageId, null, null)
            : '<p class="muted pending-hint">Analyze</p>';
        return;
    }

    const fields = fieldMapFromParapet(result.parapet_result);
    const top =
        isDriftTeamStage(stageId) ? compareTeamHtml(stageId, fields, result.parapet_result) : compareAttackerHtml(result);

    wrap.innerHTML =
        top + verdictLineHtml(stage, result) + technicalAppendixHtml(stageId, result);
}

function toggleStage(stageId) {
    if (expandedStages.has(stageId)) expandedStages.delete(stageId);
    else expandedStages.add(stageId);

    const chev = document.getElementById('chev-' + stageId);
    if (chev) chev.textContent = expandedStages.has(stageId) ? '▾' : '▸';

    renderTechnicalPanel(stageId);
}

function setVerdictChip(stageId, result) {
    const el = document.getElementById('verdict-' + stageId);
    if (!el) return;
    const p = result.parapet_result;
    const a = String(p.action).toLowerCase();
    el.className =
        'verdict-chip verdict-chip--' + (a === 'pass' ? 'pass' : a === 'block' ? 'block' : 'alert');
    el.textContent = p.action.toUpperCase() + ' · ' + riskLabel(p.risk_score);
    el.hidden = false;
}

async function loadStages() {
    try {
        const response = await fetch(`${API_BASE}/api/stages`);
        stages = await response.json();
        renderStages();
    } catch (e) {
        log('critical', e.message);
    }
}

async function loadRules() {
    try {
        const response = await fetch(`${API_BASE}/api/rules`);
        loadedRules = await response.json();
        const cnt = document.getElementById('rules-count');
        const list = document.getElementById('rules-list');
        if (cnt) cnt.textContent = String(loadedRules.length);
        if (list)
            list.innerHTML = loadedRules
                .map(r => {
                    const id = escapeHtml(r.id || '');
                    const a = escapeHtml(r.rule.action);
                    return '<div class="rule-line mono-block">' + id + ' <span class="muted">·</span> ' + a + '</div>';
                })
                .join('');
    } catch (e) {
        log('critical', e.message);
    }
}

function renderStages() {
    const container = document.getElementById('stages-container');
    const ordered = [...stages].sort((a, b) => a.id - b.id);
    const p1 = ordered.filter(s => isDriftTeamStage(s.id));
    const p2 = ordered.filter(s => !isDriftTeamStage(s.id));

    let html = '<p class="phase-label">Team</p>';
    p1.forEach(s => {
        html += renderStage(s);
    });
    html += '<p class="phase-label phase-label-spaced">Attacker</p>';
    p2.forEach(s => {
        html += renderStage(s);
    });

    container.innerHTML = html;
    ordered.forEach(s => {
        if (expandedStages.has(s.id)) renderTechnicalPanel(s.id);
        if (resultsByStageId[s.id]) setVerdictChip(s.id, resultsByStageId[s.id]);
    });
}

function renderStage(stage) {
    const team = isDriftTeamStage(stage.id);
    const title = escapeHtml(stageTitleWithoutDate(stage));
    const date = escapeHtml(stage.date || '');
    const txSig = stage.tx_signature || '';
    const txShort = txSig ? escapeHtml(txSig.slice(0, 6) + '…') : '';
    const txUrl =
        txSig.length > 0
            ? 'https://solscan.io/tx/' + encodeURIComponent(txSig)
            : '';

    const txHtml =
        txSig.length > 0
            ? `<a href="${escapeHtml(txUrl)}" target="_blank" rel="noopener" class="tx-link-plain" onclick="event.stopPropagation()">${txShort}</a>`
            : '';

    const exp = expandedStages.has(stage.id) ? '▾' : '▸';

    return (
        `<section class="stage ${team ? 'stage--team' : 'stage--attacker'}" id="stage-${stage.id}">` +
        `<button type="button" class="stage-head" onclick="toggleStage(${stage.id})" aria-expanded="${expandedStages.has(stage.id)}">` +
        `<time class="stage-date">${date}</time>` +
        `<h2 class="stage-title">${title}</h2>` +
        `<span class="stage-chev" id="chev-${stage.id}">${exp}</span>` +
        `</button>` +
        `<div class="stage-toolbar">` +
        `<button type="button" class="btn-analyze" onclick="event.stopPropagation(); fireStage(${stage.id})">Analyze</button>` +
        txHtml +
        `<span class="verdict-chip" id="verdict-${stage.id}" hidden></span>` +
        `</div>` +
        `<div id="stage-details-${stage.id}" class="stage-body"></div>` +
        `</section>`
    );
}

async function fireStage(stageId) {
    const wrap = document.getElementById('stage-' + stageId);
    if (!wrap) return;
    expandedStages.add(stageId);
    const chev = document.getElementById('chev-' + stageId);
    if (chev) chev.textContent = '▾';
    renderTechnicalPanel(stageId);
    wrap.classList.add('is-busy');

    try {
        const response = await fetch(`${API_BASE}/api/stage/fire`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ stage_id: stageId }),
        });
        if (!response.ok) throw new Error('HTTP ' + response.status);
        const result = await response.json();
        resultsByStageId[stageId] = result;
        wrap.classList.remove('is-busy');
        wrap.classList.add('has-result');
        setVerdictChip(stageId, result);
        renderTechnicalPanel(stageId);
        logStageResult(result);
    } catch (e) {
        wrap.classList.remove('is-busy');
        log('critical', e.message);
    }
}

function log(level, message) {
    const output = document.getElementById('console-output');
    if (!output) return;
    const t = new Date().toISOString().substr(11, 8);
    const line = document.createElement('div');
    line.className = 'console-line ' + level;
    line.textContent = '[' + t + '] ' + message;
    output.appendChild(line);
    output.scrollTop = output.scrollHeight;
}

function logStageResult(result) {
    const s = result.stage;
    const p = result.parapet_result;
    const rs = Array.isArray(p.rules_triggered) ? p.rules_triggered.join(', ') : '';
    log('system', `${s.id} ${p.action} ${p.risk_score}ms:${p.analysis_time_ms.toFixed(0)}`);
    if (rs) log('alert', rs);
}

function clearConsole() {
    const output = document.getElementById('console-output');
    if (output) output.innerHTML = '<div class="console-line system">cleared</div>';
    seenAlertIds.clear();
}
