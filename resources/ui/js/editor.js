// Editor UI JavaScript
// Handles UI interactions and communication with Rust via Ultralight

// ============================================
// JSDoc Type Definitions for Rust Communication
// ============================================

/**
 * @typedef {'light_update' | 'shadow_debug' | 'ortho_update' | 'volume_update' | 'sound_toggle' | 'create_faction' | 'create_entity_type' | 'delete_entity_type' | 'create_mode_toggle' | 'update_entity_position' | 'save_entity_state' | 'create_emitter' | 'render_emitter_preview'} EventType
 */

/**
 * @typedef {Object} LightUpdateEvent
 * @property {'light_update'} type
 * @property {Object} data
 * @property {number} data.x - Light direction X (-1 to 1)
 * @property {number} data.y - Light direction Y (-1 to 1)
 * @property {number} data.z - Light direction Z (-1 to 1)
 * @property {number} data.distance - Light distance from origin
 */

/**
 * @typedef {Object} ShadowDebugEvent
 * @property {'shadow_debug'} type
 * @property {Object} data
 * @property {boolean} data.enabled
 */

/**
 * @typedef {Object} OrthoUpdateEvent
 * @property {'ortho_update'} type
 * @property {Object} data
 * @property {number} [data.near]
 * @property {number} [data.far]
 * @property {number} [data.bounds]
 * @property {number} [data.bias]
 */

/**
 * @typedef {Object} VolumeUpdateEvent
 * @property {'volume_update'} type
 * @property {Object} data
 * @property {number} data.volume - Volume level (0 to 1)
 */

/**
 * @typedef {Object} SoundToggleEvent
 * @property {'sound_toggle'} type
 * @property {Object} data
 * @property {boolean} data.paused
 */

/**
 * @typedef {LightUpdateEvent | ShadowDebugEvent | OrthoUpdateEvent | VolumeUpdateEvent | SoundToggleEvent} RustEvent
 */

// ============================================
// State
// ============================================
/** @type {{ entityTypes: string[], factions: string[], emitterTypes: string[], emitterColors: number[][], selectedEntities: any[], createMode: boolean, renderPreview: boolean }} */
let editorState = {
    entityTypes: [],
    factions: [],
    emitterTypes: [],
    emitterColors: [],
    selectedEntities: [],
    createMode: false,
    renderPreview: false
};

// ============================================
// Section Toggle
// ============================================
function toggleSection(sectionId) {
    const section = document.getElementById(sectionId);
    const icon = section.querySelector('.toggle-icon');
    section.classList.toggle('collapsed');
    icon.textContent = section.classList.contains('collapsed') ? '▶' : '▼';
}

// ============================================
// Light Controls
// ============================================
/**
 * Send light direction and distance updates to Rust
 */
function onLightChange() {
    const dirX = parseFloat(document.getElementById('light-dir-x').value);
    const dirY = parseFloat(document.getElementById('light-dir-y').value);
    const dirZ = parseFloat(document.getElementById('light-dir-z').value);
    const distance = parseFloat(document.getElementById('light-distance').value);

    // Update display values
    document.getElementById('light-dir-x-val').textContent = dirX.toFixed(2);
    document.getElementById('light-dir-y-val').textContent = dirY.toFixed(2);
    document.getElementById('light-dir-z-val').textContent = dirZ.toFixed(2);
    document.getElementById('light-distance-val').textContent = distance.toFixed(0);

    /** @type {LightUpdateEvent} */
    const event = {
        type: 'light_update',
        data: { x: dirX, y: dirY, z: dirZ, distance: distance }
    };
    sendToRust(event);
}

/**
 * Send shadow debug toggle to Rust
 */
function onShadowDebugChange() {
    const enabled = document.getElementById('shadow-debug').checked;
    /** @type {ShadowDebugEvent} */
    const event = {
        type: 'shadow_debug',
        data: { enabled: enabled }
    };
    sendToRust(event);
}

/**
 * Send ortho/shadow settings to Rust
 */
function onOrthoChange() {
    const near = parseFloat(document.getElementById('ortho-near').value);
    const far = parseFloat(document.getElementById('ortho-far').value);
    const bounds = parseFloat(document.getElementById('ortho-bounds').value);
    const bias = parseFloat(document.getElementById('bias-scalar').value);

    // Update display values
    document.getElementById('ortho-near-val').textContent = near.toFixed(1);
    document.getElementById('ortho-far-val').textContent = far.toFixed(0);
    document.getElementById('ortho-bounds-val').textContent = bounds.toFixed(0);

    /** @type {OrthoUpdateEvent} */
    const event = {
        type: 'ortho_update',
        data: { near: near, far: far, bounds: bounds, bias: bias }
    };
    sendToRust(event);
}

// ============================================
// Sound Controls
// ============================================
/**
 * Send volume update to Rust
 */
function onVolumeChange() {
    const volume = parseFloat(document.getElementById('master-volume').value);
    document.getElementById('volume-val').textContent = volume.toFixed(2);
    /** @type {VolumeUpdateEvent} */
    const event = {
        type: 'volume_update',
        data: { volume: volume }
    };
    console.log('[Editor] Sending volume_update:', volume);
    sendToRust(event);
}

/**
 * Toggle sound pause/play
 */
function onSoundToggle() {
    const paused = document.getElementById('sound-paused')?.checked ?? false;
    /** @type {SoundToggleEvent} */
    const event = {
        type: 'sound_toggle',
        data: { paused: paused }
    };
    sendToRust(event);
}

// ============================================
// Entity Placement
// ============================================
function onEnterCreateMode() {
    editorState.createMode = true;

    // Update UI to show we're in create mode
    const btn = document.getElementById('create-mode-btn');
    btn.textContent = '❌ Cancel Create Mode';
    btn.onclick = onExitCreateMode;
    btn.classList.remove('btn-primary');
    btn.classList.add('btn-danger');
    document.getElementById('create-mode-hint').style.display = 'block';

    sendToRust({
        type: 'create_mode_toggle',
        enabled: true,
        entityType: document.getElementById('place-entity-type').value,
        faction: document.getElementById('place-faction').value,
        weapon: document.getElementById('include-weapon').checked ? document.getElementById('place-weapon').value : null,
        baseSpeed: parseFloat(document.getElementById('place-base-speed').value)
    });
}

function onExitCreateMode() {
    editorState.createMode = false;

    // Update UI to show we're not in create mode
    const btn = document.getElementById('create-mode-btn');
    btn.textContent = '🎯 Enter Create Mode';
    btn.onclick = onEnterCreateMode;
    btn.classList.remove('btn-danger');
    btn.classList.add('btn-primary');
    document.getElementById('create-mode-hint').style.display = 'none';

    sendToRust({
        type: 'create_mode_toggle',
        enabled: false,
        entityType: null,
        faction: null,
        weapon: null,
        baseSpeed: 0
    });
}

// Called from Rust when entity is placed to reset UI
window.editorAPI = window.editorAPI || {};
window.editorAPI.onEntityPlaced = function() {
    onExitCreateMode();
};

// ============================================
// Faction Creation
// ============================================
function createFaction() {
    const name = document.getElementById('new-faction-name').value.trim();
    if (name) {
        sendToRust({ type: 'create_faction', name: name });
        document.getElementById('new-faction-name').value = '';
    }
}

// ============================================
// Entity Type Creation
// ============================================
function onHitboxTypeChange() {
    const hitboxType = document.getElementById('new-type-hitbox').value;
    const paramsDiv = document.getElementById('hitbox-params');
    
    if (hitboxType === 'Cylinder' || hitboxType === 'Pill') {
        paramsDiv.innerHTML = `
            <div class="form-row"><label>Radius</label><input type="number" id="new-type-radius" step="0.1" value="0.5"></div>
            <div class="form-row"><label>Height</label><input type="number" id="new-type-height" step="0.1" value="1.8"></div>
        `;
    } else if (hitboxType === 'BoxDim') {
        paramsDiv.innerHTML = `
            <div class="form-row"><label>Half X</label><input type="number" id="new-type-hx" step="0.1" value="0.5"></div>
            <div class="form-row"><label>Half Y</label><input type="number" id="new-type-hy" step="0.1" value="0.5"></div>
            <div class="form-row"><label>Half Z</label><input type="number" id="new-type-hz" step="0.1" value="0.5"></div>
        `;
    } else if (hitboxType === 'Sphere') {
        paramsDiv.innerHTML = `
            <div class="form-row"><label>Radius</label><input type="number" id="new-type-radius" step="0.1" value="0.5"></div>
        `;
    } else {
        paramsDiv.innerHTML = '<p style="color:var(--text-secondary);font-style:italic">No parameters needed</p>';
    }
}

function createEntityType() {
    const hitboxType = document.getElementById('new-type-hitbox').value;
    const data = {
        type: 'create_entity_type',
        entityType: document.getElementById('new-type-name').value,
        rotCorrection: document.getElementById('new-type-rot').value.split(',').map(Number),
        scaleCorrection: document.getElementById('new-type-scale').value.split(',').map(Number),
        meshPath: document.getElementById('new-type-mesh').value,
        texturePath: document.getElementById('new-type-texture').value,
        aggroRange: parseFloat(document.getElementById('new-type-aggro').value),
        totalMass: parseFloat(document.getElementById('new-type-mass').value),
        hitbox: hitboxType,
        radius: document.getElementById('new-type-radius')?.value || 0,
        height: document.getElementById('new-type-height')?.value || 0,
        hx: document.getElementById('new-type-hx')?.value || 0,
        hy: document.getElementById('new-type-hy')?.value || 0,
        hz: document.getElementById('new-type-hz')?.value || 0
    };
    sendToRust(data);
}

function deleteEntityType() {
    const entityType = document.getElementById('delete-entity-type').value;
    if (entityType && confirm(`Delete entity type "${entityType}"?`)) {
        sendToRust({ type: 'delete_entity_type', entityType: entityType });
    }
}

// ============================================
// Generic Event Sender
// ============================================
function sendEvent(eventType) {
    sendToRust({ type: eventType });
}

// ============================================
// Particle Editor
// ============================================
function updateAlphaPowerLabel() {
    const val = parseFloat(document.getElementById('emitter-alpha-power').value);
    document.getElementById('alpha-power-val').textContent = val.toFixed(2);
}

function updateScalePowerLabel() {
    const val = parseFloat(document.getElementById('emitter-scale-power').value);
    document.getElementById('scale-power-val').textContent = val.toFixed(2);
}

function addEmitterColor() {
    const colorPicker = document.getElementById('new-color-picker');
    const alphaInput = document.getElementById('new-color-alpha');
    const hex = colorPicker.value;
    const alpha = parseFloat(alphaInput.value);

    // Convert hex to RGB
    const r = parseInt(hex.substr(1, 2), 16) / 255;
    const g = parseInt(hex.substr(3, 2), 16) / 255;
    const b = parseInt(hex.substr(5, 2), 16) / 255;

    editorState.emitterColors.push([r, g, b, alpha]);
    renderEmitterColors();
}

function removeEmitterColor(index) {
    editorState.emitterColors.splice(index, 1);
    renderEmitterColors();
}

function renderEmitterColors() {
    const container = document.getElementById('emitter-colors-list');
    container.innerHTML = editorState.emitterColors.map((c, i) => `
        <div class="form-row" style="margin-bottom:4px">
            <div style="width:20px;height:20px;background:rgba(${c[0]*255},${c[1]*255},${c[2]*255},${c[3]});border:1px solid var(--border-color);border-radius:2px"></div>
            <span style="flex:1;font-size:10px;font-family:monospace">${c.map(v => v.toFixed(2)).join(', ')}</span>
            <button class="btn btn-danger" style="padding:2px 6px" onclick="removeEmitterColor(${i})">×</button>
        </div>
    `).join('');
}

// Helper to gather current emitter form data
function gatherEmitterData() {
    const parseRange = (id) => {
        const val = document.getElementById(id).value;
        const parts = val.split(',').map(Number);
        return parts.length === 2 ? parts : [parts[0], parts[0]];
    };
    const parseVec3 = (id) => {
        const val = document.getElementById(id).value;
        return val.split(',').map(Number);
    };

    return {
        name: document.getElementById('emitter-name').value || 'preview',
        position: parseVec3('emitter-pos'),
        direction: parseVec3('emitter-dir'),
        angleRange: parseRange('emitter-angle'),
        radiusRange: parseRange('emitter-radius'),
        jitter: parseVec3('emitter-jitter'),
        gravity: parseFloat(document.getElementById('emitter-gravity').value) || -9.8,
        radialSpeed: parseRange('emitter-radial-speed'),
        upSpeed: parseRange('emitter-up-speed'),
        lifetime: parseRange('emitter-lifetime'),
        particleCount: parseInt(document.getElementById('emitter-count').value) || 10,
        pps: parseInt(document.getElementById('emitter-pps').value) || 0,
        texturePath: document.getElementById('emitter-texture').value,
        textureHasAlpha: document.getElementById('emitter-tex-alpha').checked,
        baseAlpha: parseRange('emitter-base-alpha'),
        alphaMultiplier: parseFloat(document.getElementById('emitter-alpha-mult').value) || 1.0,
        alphaPower: parseFloat(document.getElementById('emitter-alpha-power').value) || 1.0,
        baseScale: parseRange('emitter-base-scale'),
        scaleMultiplier: parseFloat(document.getElementById('emitter-scale-mult').value) || 1.0,
        scalePower: parseFloat(document.getElementById('emitter-scale-power').value) || 1.0,
        colors: editorState.emitterColors
    };
}

function onRenderPreviewChange() {
    editorState.renderPreview = document.getElementById('render-emitter-preview').checked;
    const emitterData = gatherEmitterData();
    sendToRust({
        type: 'render_emitter_preview',
        enabled: editorState.renderPreview,
        ...emitterData
    });
}

function saveEmitter() {
    const data = gatherEmitterData();
    data.type = 'save_emitter';
    sendToRust(data);
}

// ============================================
// Data Updates from Rust
// ============================================
function updatePlayerData(data) {
    if (data.position) {
        document.getElementById('player-pos').textContent =
            `${data.position[0].toFixed(1)}, ${data.position[1].toFixed(1)}, ${data.position[2].toFixed(1)}`;
    }
    if (data.state) document.getElementById('player-state').textContent = data.state;
    if (data.attackState) document.getElementById('player-attack-state').textContent = data.attackState;
    if (data.animation) document.getElementById('player-animation').textContent = data.animation;
}

function updateEntityTypes(types) {
    editorState.entityTypes = types;
    const selects = ['place-entity-type', 'place-weapon', 'delete-entity-type'];
    selects.forEach(id => {
        const select = document.getElementById(id);
        if (select) {
            select.innerHTML = types.map(t => `<option value="${t}">${t}</option>`).join('');
        }
    });
}

function updateFactions(factions) {
    editorState.factions = factions;
    const select = document.getElementById('place-faction');
    if (select) {
        select.innerHTML = factions.map(f => `<option value="${f}">${f}</option>`).join('');
    }
}

function updateEmitterTypes(types) {
    editorState.emitterTypes = types;
    const select = document.getElementById('emitter-type-select');
    if (select) {
        select.innerHTML = types.map(t => `<option value="${t}">${t}</option>`).join('');
    }
}

function updateSelectedEntities(entities) {
    editorState.selectedEntities = entities;
    const container = document.getElementById('selected-entities');
    if (entities.length === 0) {
        container.innerHTML = '<p style="color:var(--text-secondary);font-style:italic">No entities selected</p>';
    } else {
        container.innerHTML = entities.map(e => `
            <div style="margin-bottom:8px;padding:8px;background:var(--bg-secondary);border-radius:4px">
                <div class="data-row">
                    <span class="data-label">ID</span>
                    <span class="data-value">${e.id}</span>
                </div>
                <div class="data-row">
                    <span class="data-label">Type</span>
                    <span class="data-value">${e.type}</span>
                </div>
                <div class="form-row">
                    <label>Position</label>
                    <input type="text" value="${e.position.join(',')}" onchange="updateEntityPosition(${e.id}, this.value)">
                </div>
            </div>
        `).join('');
    }
}

function updateEntityPosition(entityId, posStr) {
    const pos = posStr.split(',').map(Number);
    sendToRust({ type: 'update_entity_position', entityId: entityId, position: pos });
}

function updateLightValues(data) {
    if (data.dirX !== undefined) document.getElementById('light-dir-x').value = data.dirX;
    if (data.dirY !== undefined) document.getElementById('light-dir-y').value = data.dirY;
    if (data.dirZ !== undefined) document.getElementById('light-dir-z').value = data.dirZ;
    if (data.distance !== undefined) document.getElementById('light-distance').value = data.distance;
    if (data.shadowDebug !== undefined) document.getElementById('shadow-debug').checked = data.shadowDebug;
    if (data.orthoNear !== undefined) document.getElementById('ortho-near').value = data.orthoNear;
    if (data.orthoFar !== undefined) document.getElementById('ortho-far').value = data.orthoFar;
    if (data.orthoBounds !== undefined) document.getElementById('ortho-bounds').value = data.orthoBounds;
    if (data.biasScalar !== undefined) document.getElementById('bias-scalar').value = data.biasScalar;
    // Update labels
    onLightChange();
}

// ============================================
// Communication with Rust (Polling Approach)
// ============================================

/**
 * Pending events queue - Rust will poll this via evaluate_script
 * @type {string[]}
 */
window.__rustPendingEvents = [];

/**
 * Send an event to Rust by adding it to the pending events queue.
 * Rust will call drainEvents() to retrieve them.
 * @param {RustEvent} data - The event to send
 */
function sendToRust(data) {
    window.__rustPendingEvents.push(JSON.stringify(data));
}

/**
 * Called by Rust to drain all pending events.
 * Returns a JSON array of event strings, then clears the queue.
 * @returns {string} JSON array of pending events
 */
function drainEvents() {
    const events = JSON.stringify(window.__rustPendingEvents);
    window.__rustPendingEvents = [];
    return events;
}

// Expose functions globally for Rust to call
window.editorAPI = {
    updatePlayerData,
    updateEntityTypes,
    updateFactions,
    updateEmitterTypes,
    updateSelectedEntities,
    updateLightValues,
    drainEvents
};

// Initialize on load
document.addEventListener('DOMContentLoaded', () => {
    console.log('[Editor] UI initialized');
});

