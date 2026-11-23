// Gridlock's browser plugin, part 8 of 9: models from GRIDLOCK_MODELS_URL,
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
    // Models downloaded but still being written to Cache Storage.
    const storing = new Map(); // path -> Promise

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
            await storing.get(path);
            if (cache && (await cache.match(modelUrl(path)))) return;
        }
        await loadModel(path);
    }

    // The game loading models itself: stop starting preloads.
    function pausePreload() {
        if (preloadPaused) return;
        preloadPaused = true;
        // The queued preloads will never start now; drop them from the
        // count so the title screen's progress note does not stay up.
        preloadTotal -= preloadQueue.length;
        preloadQueue.length = 0;
    }

    // A file the game loads, as fs_load_file would: models from
    // GRIDLOCK_MODELS_URL (when set), everything else from the page. Null if
    // it could not be loaded.
    function loadFile(path) {
        if (MODELS_URL && path.startsWith(MODELS_PREFIX)) {
            pausePreload();
            return loadModel(path).catch(() => null);
        }
        return fetch(path)
            .then((resp) => (resp.ok ? resp.arrayBuffer().then((b) => new Uint8Array(b)) : null))
            .catch(() => null);
    }

    async function fetchModel(path) {
        const url = modelUrl(path);
        const cache = await openModelsCache();
        await storing.get(path); // a copy still being written is read back, not downloaded again
        const cached = cache && (await cache.match(url));
        if (cached) {
            const bytes = new Uint8Array(await cached.arrayBuffer());
            downloads.set(path, { received: bytes.length, total: bytes.length });
            return bytes;
        }
        const resp = await fetch(url);
        if (!resp.ok) return null;
        // The cache gets a copy of the response as it streams in; the model
        // is used without waiting for that write (the next read of it waits).
        if (cache) {
            const stored = cache
                .put(url, resp.clone())
                .catch(() => {}) // quota full: still use this copy
                .finally(() => storing.get(path) === stored && storing.delete(path));
            storing.set(path, stored);
        }
        const progress = { received: 0, total: Number(resp.headers.get("Content-Length")) || 0 };
        downloads.set(path, progress);
        // Read in chunks so the loading screen can show how far along it is,
        // straight into one buffer when the size is known. (Content-Length is
        // the compressed size of a compressed response, which a page on
        // another origin can not tell: then the chunks are put together.)
        let buffer = progress.total > 0 ? new Uint8Array(progress.total) : null;
        const chunks = [];
        const reader = resp.body.getReader();
        for (;;) {
            const { done, value } = await reader.read();
            if (done) break;
            if (buffer && progress.received + value.length <= buffer.length) {
                buffer.set(value, progress.received);
            } else {
                if (buffer) chunks.push(buffer.subarray(0, progress.received));
                buffer = null;
                chunks.push(value);
            }
            progress.received += value.length;
        }
        let bytes;
        if (buffer) {
            bytes = progress.received === buffer.length ? buffer : buffer.slice(0, progress.received);
        } else {
            bytes = new Uint8Array(progress.received);
            let at = 0;
            for (const c of chunks) {
                bytes.set(c, at);
                at += c.length;
            }
        }
        progress.total = bytes.length;
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
                loadFile(path).then((bytes) => {
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

    Object.assign(gridlockWeb, { registerModels, loadFile });
})((window.gridlockWeb = window.gridlockWeb || {}));
