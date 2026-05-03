/**
 * Minimal FMOD Studio HTML5 bridge for learn-opengl-rs.
 * Expects global FMODModule from Firelight's fmodstudio.js (not shipped in this repo).
 * Exposes global LearnOpenglFmod for wasm (see docs/fmod_web_setup.md).
 */
(function () {
  const STUDIO_INIT_NORMAL = 0;
  const FMOD_INIT_NORMAL = 0;
  const FMOD_INIT_3D_RIGHTHANDED = 0x10;
  const STUDIO_LOAD_BANK_NORMAL = 0;
  const STUDIO_STOP_IMMEDIATE = 1;

  let studio = null;
  /** @type {Map<string, object>} FMOD EventDescription */
  const descriptions = new Map();
  /** @type {Map<string, object>} SoundType key -> 2D EventInstance */
  const active2d = new Map();
  /** @type {Map<number, object[]>} entity id -> instances (reserved; native path may use later) */
  const active3dByEntity = new Map();

  let pendingConfig = null;
  let runtimeStarting = false;
  let booted = false;
  let gestureHookInstalled = false;

  function vec(x, y, z) {
    return { x, y, z };
  }

  function make3DAttributes(position, velocity, forward, up) {
    return {
      position: vec(position.x, position.y, position.z),
      velocity: vec(velocity.x, velocity.y, velocity.z),
      forward: vec(forward.x, forward.y, forward.z),
      up: vec(up.x, up.y, up.z),
    };
  }

  function checkFmodResult(r, label) {
    const FMOD = globalThis.__LearnOpenglFmodFMOD;
    if (r !== 0 && r !== undefined && FMOD && typeof FMOD.ErrorString === "function") {
      console.warn(`[LearnOpenglFmod] ${label} ->`, r, FMOD.ErrorString(r));
    }
  }

  /**
   * Absolute URL for a bank file. Resolved with `new URL(..., location.href)` so it works when the
   * game is not at the site root (e.g. itch.io serves under `/html/<id>/` — `origin + /resources/...`
   * hits the wrong path and often returns 403).
   */
  function bankHttpUrl(config, fileName) {
    const base = (config.bankBase || "resources/fmod/Web").replace(/^\/+/, "").replace(/\/$/, "");
    const relPath = `${base}/${fileName}`.replace(/\/+/g, "/");
    return new URL(relPath, window.location.href).href;
  }

  /**
   * Register bank bytes in Emscripten MEMFS before Studio::loadBankFile (see FMOD load_banks.js example).
   * Virtual paths are /Master.bank and /Master.strings.bank.
   */
  function registerBanksInMemfs(FMOD, config) {
    if (typeof FMOD.FS_createPreloadedFile !== "function") {
      console.warn(
        "[LearnOpenglFmod] FS_createPreloadedFile missing; loadBankFile will not see HTTP banks"
      );
      return;
    }
    const names = ["Master.bank", "Master.strings.bank"];
    for (const name of names) {
      const url = bankHttpUrl(config, name);
      try {
        FMOD.FS_createPreloadedFile("/", name, url, true, false);
      } catch (e) {
        console.error("[LearnOpenglFmod] FS_createPreloadedFile failed for", name, url, e);
      }
    }
  }

  function finishBootFromConfig(FMOD, config) {
    const outSys = {};
    const cr = FMOD.Studio_System_Create(outSys);
    if (cr !== 0) {
      console.error("[LearnOpenglFmod] Studio_System_Create failed", cr);
      return;
    }
    studio = outSys.val;
    const ir = studio.initialize(
      512,
      STUDIO_INIT_NORMAL,
      FMOD_INIT_NORMAL | FMOD_INIT_3D_RIGHTHANDED,
      null
    );
    if (ir !== 0) {
      console.error("[LearnOpenglFmod] studio.initialize failed", ir);
      studio = null;
      return;
    }

    // Paths must match MEMFS names registered in preRun via FS_createPreloadedFile (FMOD HTML5 pattern).
    const bankOut = {};
    checkFmodResult(
      studio.loadBankFile("/Master.bank", STUDIO_LOAD_BANK_NORMAL, bankOut),
      "loadBankFile /Master.bank"
    );
    const stringsOut = {};
    checkFmodResult(
      studio.loadBankFile("/Master.strings.bank", STUDIO_LOAD_BANK_NORMAL, stringsOut),
      "loadBankFile /Master.strings.bank"
    );

    const sounds = config.sounds || {};
    for (const [soundKey, eventPath] of Object.entries(sounds)) {
      const evOut = {};
      const gr = studio.getEvent(eventPath, evOut);
      if (gr !== 0) {
        console.warn("[LearnOpenglFmod] getEvent failed for", soundKey, eventPath, gr);
        continue;
      }
      const desc = evOut.val;
      desc.loadSampleData();
      descriptions.set(soundKey, desc);
    }

    booted = true;
    console.log("[LearnOpenglFmod] Studio ready, banks from", config.bankBase || "resources/fmod/Web");
  }

  function startRuntimeIfNeeded() {
    if (booted || runtimeStarting) return;
    if (typeof globalThis.FMODModule !== "function") {
      return;
    }
    if (!pendingConfig) return;

    runtimeStarting = true;
    const FMOD = {};
    globalThis.__LearnOpenglFmodFMOD = FMOD;
    const config = pendingConfig;

    FMOD.preRun = function () {
      registerBanksInMemfs(FMOD, config);
    };

    FMOD.onRuntimeInitialized = function () {
      try {
        finishBootFromConfig(FMOD, config);
      } catch (e) {
        console.error("[LearnOpenglFmod] init in onRuntimeInitialized", e);
      } finally {
        runtimeStarting = false;
      }
    };

    try {
      globalThis.FMODModule(FMOD);
    } catch (e) {
      console.error("[LearnOpenglFmod] FMODModule() failed", e);
      runtimeStarting = false;
    }
  }

  function installGestureBoot() {
    if (gestureHookInstalled) return;
    gestureHookInstalled = true;
    const tryStart = () => {
      startRuntimeIfNeeded();
      if (booted) {
        document.removeEventListener("pointerdown", tryStart, true);
        document.removeEventListener("keydown", tryStart, true);
      }
    };
    document.addEventListener("pointerdown", tryStart, true);
    document.addEventListener("keydown", tryStart, true);
  }

  globalThis.LearnOpenglFmod = {
    /**
     * @param {string} configJson JSON: { bankBase?: string, sounds: Record<string, string> }
     */
    init(configJson) {
      try {
        pendingConfig = JSON.parse(configJson);
      } catch (e) {
        console.error("[LearnOpenglFmod] bad config JSON", e);
        pendingConfig = null;
        return;
      }
      if (typeof globalThis.FMODModule !== "function") {
        console.warn(
          "[LearnOpenglFmod] FMODModule missing; copy fmodstudio.js/.wasm into third_party/fmod/ (see docs/fmod_web_setup.md)"
        );
        return;
      }
      installGestureBoot();
      startRuntimeIfNeeded();
    },

    isReady() {
      return booted && studio !== null;
    },

    update() {
      if (!studio) return;
      try {
        checkFmodResult(studio.update(), "studio.update");
      } catch (e) {
        console.warn("[LearnOpenglFmod] update", e);
      }
    },

    /**
     * @param {number} px
     * @param {number} py
     * @param {number} pz
     * @param {number} fx
     * @param {number} fy
     * @param {number} fz
     * @param {number} ux
     * @param {number} uy
     * @param {number} uz
     */
    setListener(px, py, pz, fx, fy, fz, ux, uy, uz) {
      if (!studio) return;
      const pos = { x: px, y: py, z: pz };
      const vel = { x: 0, y: 0, z: 0 };
      const fwd = { x: fx, y: fy, z: fz };
      const up = { x: ux, y: uy, z: uz };
      const attrs = make3DAttributes(pos, vel, fwd, up);
      try {
        checkFmodResult(studio.setListenerAttributes(0, attrs, null), "setListenerAttributes");
      } catch (e) {
        console.warn("[LearnOpenglFmod] setListener", e);
      }
    },

    /**
     * @param {string} soundKey e.g. "Music"
     */
    play2d(soundKey) {
      if (!studio) return;
      if (active2d.has(soundKey)) return;
      const desc = descriptions.get(soundKey);
      if (!desc) {
        console.warn("[LearnOpenglFmod] unknown sound", soundKey);
        return;
      }
      const out = {};
      if (desc.createInstance(out) !== 0) return;
      const inst = out.val;
      if (inst.start() !== 0) return;
      active2d.set(soundKey, inst);
    },

    /**
     * @param {string} soundKey
     * @param {number} x
     * @param {number} y
     * @param {number} z  already converted to FMOD space from Rust
     */
    play3d(soundKey, x, y, z) {
      if (!studio) return;
      const desc = descriptions.get(soundKey);
      if (!desc) {
        console.warn("[LearnOpenglFmod] unknown sound", soundKey);
        return;
      }
      const out = {};
      if (desc.createInstance(out) !== 0) return;
      const inst = out.val;
      const attrs = make3DAttributes(
        { x, y, z },
        { x: 0, y: 0, z: 0 },
        { x: 0, y: 0, z: 1 },
        { x: 0, y: 1, z: 0 }
      );
      inst.set3DAttributes(attrs);
      if (inst.start() !== 0) return;
      inst.release();
    },

    /**
     * @param {string} soundKey
     */
    stop2d(soundKey) {
      const inst = active2d.get(soundKey);
      if (!inst) return;
      try {
        inst.stop(STUDIO_STOP_IMMEDIATE);
        inst.release();
      } catch (e) {
        console.warn("[LearnOpenglFmod] stop2d", e);
      }
      active2d.delete(soundKey);
    },

    /**
     * @param {string} soundKey
     * @param {number} linearGain
     */
    setMasterVolumeFor2d(soundKey, linearGain) {
      const inst = active2d.get(soundKey);
      if (!inst) return;
      try {
        inst.setParameterByName("main_volume", linearGain, false);
      } catch (e) {
        console.warn("[LearnOpenglFmod] setMasterVolumeFor2d", e);
      }
    },

    /** @param {number} entityId */
    cleanupEntity3d(entityId) {
      const list = active3dByEntity.get(entityId);
      if (!list) return;
      for (const inst of list) {
        try {
          inst.stop(STUDIO_STOP_IMMEDIATE);
          inst.release();
        } catch (_) {}
      }
      active3dByEntity.delete(entityId);
    },
  };
})();
