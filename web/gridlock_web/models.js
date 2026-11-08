// Gridlock's browser plugin, part 8 of 8: models from GRIDLOCK_MODELS_URL,
// through Cache Storage, and their preloading. registerModels adds their
// imports. (See ../gridlock_web.js.)
"use strict";

(function (gridlockWeb) {
    const { writeU32 } = gridlockWeb;

    // Models are too big to ship with the page, so build_web.sh can host them
    // elsewhere and set GRIDLOCK_MODELS_URL (plus a version) in index.html. They
    // are downloaded once and kept in Cache Storage; a new version starts a new
    // cache and drops the old one. Every other file loads from the page as usual.
    const MODELS_PREFIX = "assets/models/";
    const MODELS_URL = window.GRIDLOCK_MODELS_URL || "";
    const MODELS_CACHE = "gridlock-models-" + (window.GRIDLOCK_MODELS_VERSION || "dev");

    let modelsCache = null;
    function openModelsCache() {
        if (!modelsCache) {
            modelsCache = (async () => {
                if (!("caches" in window)) return null;
                try {
                    for (const name of await caches.keys()) {
                        if (name.startsWith("gridlock-models-") && name !== MODELS_CACHE) caches.delete(name);
                    }
                    navigator.storage?.persist?.();
                    return await caches.open(MODELS_CACHE);
                } catch {
                    return null; // private windows can refuse storage; download every time
                }
            })();
        }
        return modelsCache;
    }

    // Bytes received so far per model path, for the loading screen
    // (gridlock_download_received / gridlock_download_total).
    const downloads = new Map(); // path -> { received, total }
    // One download per model at a time: the game asking for a model that is
    // being preloaded waits for that download instead of starting another.
    const inflight = new Map(); // path -> Promise<Uint8Array | null>

    function modelUrl(path) {
        const file = path.slice(MODELS_PREFIX.length).split("/").map(encodeURIComponent).join("/");
        return MODELS_URL.replace(/\/$/, "") + "/" + file + "?v=" + (window.GRIDLOCK_MODELS_VERSION || "dev");
    }

    function loadModel(path) {
        if (!inflight.has(path)) {
            const download = fetchModel(path).finally(() => inflight.delete(path));
            inflight.set(path, download);
        }
        return inflight.get(path);
    }

    // ---- preloading: from the title screen on, download the models the game
    // will need into Cache Storage (gridlock_preload), so joining a game only
    // reads them from disk. Stops starting new downloads once the game loads
    // models itself, so the two do not compete for bandwidth.
    const PRELOAD_PARALLEL = 3;
    const preloadQueue = [];
    let preloadActive = 0;
    let preloadDone = 0;
    let preloadTotal = 0;
    let preloadPaused = false;

    function pumpPreload() {
        while (!preloadPaused && preloadActive < PRELOAD_PARALLEL && preloadQueue.length) {
            const path = preloadQueue.shift();
            preloadActive++;
            preloadModel(path)
                .catch(() => {})
                .finally(() => {
                    preloadActive--;
                    preloadDone++;
                    pumpPreload();
                });
        }
    }

    // Like loadModel, but a model already cached is not read back.
    async function preloadModel(path) {
        if (!inflight.has(path)) {
            const cache = await openModelsCache();
            if (cache && (await cache.match(modelUrl(path)))) return;
        }
        await loadModel(path);
    }

    async function fetchModel(path) {
        const url = modelUrl(path);
        const cache = await openModelsCache();
        const cached = cache && (await cache.match(url));
        if (cached) {
            const bytes = new Uint8Array(await cached.arrayBuffer());
            downloads.set(path, { received: bytes.length, total: bytes.length });
            return bytes;
        }
        const resp = await fetch(url);
        if (!resp.ok) return null;
        const progress = { received: 0, total: Number(resp.headers.get("Content-Length")) || 0 };
        downloads.set(path, progress);
        // Read in chunks so the loading screen can show how far along it is.
        const chunks = [];
        const reader = resp.body.getReader();
        for (;;) {
            const { done, value } = await reader.read();
            if (done) break;
            chunks.push(value);
            progress.received += value.length;
        }
        const bytes = new Uint8Array(progress.received);
        let at = 0;
        for (const c of chunks) {
            bytes.set(c, at);
            at += c.length;
        }
        progress.total = bytes.length;
        if (cache) {
            const headers = { "Content-Type": resp.headers.get("Content-Type") || "application/octet-stream" };
            try {
                await cache.put(url, new Response(bytes, { headers }));
            } catch {} // quota full: still use this copy
        }
        return bytes;
    }

    function registerModels(env) {
        // ---- file loading: models from GRIDLOCK_MODELS_URL, through Cache Storage

        if (MODELS_URL) {
            const loadFromPage = env.fs_load_file;
            env.fs_load_file = function (ptr, len) {
                const path = UTF8ToString(ptr, len);
                if (!path.startsWith(MODELS_PREFIX)) return loadFromPage(ptr, len);
                const id = FS.unique_id++;
                if (!preloadPaused) {
                    preloadPaused = true;
                    // The queued preloads will never start now; drop them from the
                    // count so the title screen's progress note does not stay up.
                    preloadTotal -= preloadQueue.length;
                    preloadQueue.length = 0;
                }
                loadModel(path)
                    .catch(() => null)
                    .then((bytes) => {
                        FS.loaded_files[id] = bytes;
                        wasm_exports.file_loaded(id);
                    });
                return id;
            };
        }

        // ---- client: download progress of one model, for the loading screen
        // (client/src/app/asset_loader.rs). -1 when it is not being downloaded.

        env.gridlock_preload = function (ptr, len) {
            const path = UTF8ToString(ptr, len);
            if (!MODELS_URL || preloadPaused || !path.startsWith(MODELS_PREFIX) || preloadQueue.includes(path)) return;
            preloadQueue.push(path);
            preloadTotal++;
            pumpPreload();
        };
        // Preloaded models so far and in all, as two u32s at out_ptr.
        env.gridlock_preload_status = function (outPtr) {
            writeU32(outPtr, preloadDone);
            writeU32(outPtr + 4, preloadTotal);
        };

        env.gridlock_download_received = function (ptr, len) {
            return downloads.get(UTF8ToString(ptr, len))?.received ?? -1;
        };
        env.gridlock_download_total = function (ptr, len) {
            return downloads.get(UTF8ToString(ptr, len))?.total ?? -1;
        };
    }

    Object.assign(gridlockWeb, { registerModels });
})((window.gridlockWeb = window.gridlockWeb || {}));
