// Gridlock's browser plugin, part 2 of 9: wasm memory access, the open
// sockets, timers that keep running in hidden tabs, and small WebRTC
// helpers. (See ../gridlock_web.js.)
"use strict";

(function (gridlockWeb) {
    const { GATHER_MS } = gridlockWeb;

    const decoder = new TextDecoder();
    const sockets = new Map(); // handle -> RtcSocket

    function readString(ptr, len) {
        return decoder.decode(new Uint8Array(wasm_memory.buffer, ptr, len));
    }

    function readBytes(ptr, len) {
        return new Uint8Array(wasm_memory.buffer, ptr, len).slice();
    }

    // Views of the whole wasm memory, shared by the calls below so they do
    // not each make one. Growing the memory replaces its buffer (the old one
    // is detached), so they are made anew when that has happened.
    let heapBuffer = null;
    let heapU8 = null;
    let heapU32 = null;
    let heapF32 = null;
    function heap() {
        if (heapBuffer !== wasm_memory.buffer) {
            heapBuffer = wasm_memory.buffer;
            heapU8 = new Uint8Array(heapBuffer);
            heapU32 = new Uint32Array(heapBuffer);
            heapF32 = new Float32Array(heapBuffer);
        }
    }
    function heapFloats() {
        heap();
        return heapF32;
    }

    // (`ptr` is a Rust *mut u32, so 4-aligned.)
    function writeU32(ptr, value) {
        heap();
        heapU32[ptr >>> 2] = value >>> 0;
    }

    function writeBytes(ptr, bytes) {
        heap();
        heapU8.set(bytes, ptr);
    }

    // Timers in hidden tabs are throttled (to once a minute after a while),
    // which would leave a hidden host deaf to newcomers; a worker's timers are
    // not, so polling waits on those.
    let timerWorker = null;
    const timerCallbacks = new Map();
    let nextTimer = 1;
    function sleep(ms) {
        if (!timerWorker && typeof Worker !== "undefined") {
            try {
                const source = "onmessage = (e) => setTimeout(() => postMessage(e.data.id), e.data.ms);";
                timerWorker = new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
                timerWorker.onmessage = (e) => {
                    timerCallbacks.get(e.data)?.();
                    timerCallbacks.delete(e.data);
                };
            } catch {
                timerWorker = false;
            }
        }
        if (!timerWorker) return new Promise((r) => setTimeout(r, ms));
        return new Promise((r) => {
            const id = nextTimer++;
            timerCallbacks.set(id, r);
            timerWorker.postMessage({ id, ms });
        });
    }

    // The game passes wss://<registry>/room/<code>; signalling now runs over
    // https://<registry>/room/<code>/..., and ICE servers live at https://<registry>/ice.
    // The game asks for the same few rooms on every frame of the host list
    // (preconnect, prewarm), so the answers are kept (a bounded few).
    const roomHttpUrls = new Map(); // room url -> http url
    const ROOM_HTTP_URLS_MAX = 64;
    function roomHttpUrl(roomUrl) {
        const known = roomHttpUrls.get(roomUrl);
        if (known !== undefined) return known;
        const url = new URL(roomUrl);
        url.protocol = url.protocol === "wss:" ? "https:" : "http:";
        url.search = "";
        const http = url.toString().replace(/\/$/, "");
        if (roomHttpUrls.size >= ROOM_HTTP_URLS_MAX) roomHttpUrls.clear();
        roomHttpUrls.set(roomUrl, http);
        return http;
    }

    // Resolves once a connection has gathered its ICE candidates, or after
    // `ms`, whichever comes first; unless `full`, already once a server
    // reflexive candidate is in (the rest trickle). (A candidate is in the
    // local description by the time its event fires.)
    function gathered(pc, { full = false, ms = GATHER_MS } = {}) {
        if (pc.iceGatheringState === "complete") return Promise.resolve();
        return new Promise((resolve) => {
            const done = () => {
                clearTimeout(timer);
                pc.removeEventListener("icegatheringstatechange", check);
                pc.removeEventListener("icecandidate", candidate);
                resolve();
            };
            const check = () => pc.iceGatheringState === "complete" && done();
            const candidate = (e) => !full && isReflexive(e.candidate) && done();
            pc.addEventListener("icegatheringstatechange", check);
            pc.addEventListener("icecandidate", candidate);
            const timer = setTimeout(done, ms);
        });
    }

    function isReflexive(candidate) {
        return Boolean(candidate) && (candidate.type === "srflx" || / typ srflx /.test(candidate.candidate));
    }

    function randomHex(bytes) {
        // (getRandomValues, unlike randomUUID, works on a plain-http LAN page)
        return [...crypto.getRandomValues(new Uint8Array(bytes))].map((b) => b.toString(16).padStart(2, "0")).join("");
    }

    function isStunOnly(server) {
        return [].concat(server.urls).every((u) => u.startsWith("stun:"));
    }

    Object.assign(gridlockWeb, {
        sockets, readString, readBytes, writeU32, writeBytes, heapFloats, sleep, roomHttpUrl, gathered, isReflexive,
        randomHex, isStunOnly,
    });
})((window.gridlockWeb = window.gridlockWeb || {}));
