// Browser networking for Gridlock, as a miniquad plugin (no wasm-bindgen).
//
// WebRTC: one data channel per peer, negotiated through the HTTP signalling
// mailboxes of the Gridlock registry (registry/dcs/index.js). The host
// publishes offers made ahead ("spares"); a player answers one and sends the
// answer with its join request (or, without a spare, sends its own offer
// with it). Players only ever connect to the host. ICE
// servers (STUN, plus TURN relay credentials when configured) come from the
// registry's /ice route, fetched ahead of time.
//
// HTTP: fetch() for the host list.
//
// Rust drives everything by polling (transport/src/web.rs). Load this file
// after mq_js_bundle.js and before calling load().
//
// The plugin's parts are the modules in gridlock_web/, loaded before this
// file in the order index.html lists them; each adds what it defines to
// window.gridlockWeb. This file registers the plugin.
"use strict";

(function (gridlockWeb) {
    const {
        PLUGIN_VERSION, sockets, readString, readBytes, writeU32, writeBytes, roomHttpUrl, iceServersFor, prewarm,
        RtcSocket, preconnect, takePreconnected, registerModels, registerDecode,
    } = gridlockWeb;

    const requests = new Map(); // id -> { done, status, body }
    const sounds = new Map(); // id -> AudioBuffer
    let nextHandle = 1;

    // Browsers keep audio suspended until the user interacts with the page.
    let audioContext = null;
    function audio() {
        if (!audioContext) {
            audioContext = new AudioContext();
            const resume = () => audioContext.resume();
            for (const ev of ["pointerdown", "keydown", "touchstart"]) {
                window.addEventListener(ev, resume, { capture: true });
            }
        }
        return audioContext;
    }

    // ------------------------------------------------------------ hidden tabs

    // The game advances once per animation frame, and browsers stop animation
    // frames in hidden tabs, which would freeze a game hosted in one (players
    // could connect but nobody would answer). While hidden with a game open,
    // a worker's ticks run the frames instead: worker timers and the messages
    // they post keep going in the background. Those frames only keep the game
    // and its connections going, without drawing (see platform::page_hidden),
    // so 20 a second is plenty.
    const HIDDEN_FRAME_MS = 50;
    let hiddenTicker = null;

    function keepRunningWhenHidden() {
        if (hiddenTicker || typeof Worker === "undefined") return;
        const source = `setInterval(() => postMessage(0), ${HIDDEN_FRAME_MS});`;
        try {
            hiddenTicker = new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
        } catch {
            return; // no workers (e.g. a strict content policy): the game pauses when hidden
        }
        hiddenTicker.onmessage = () => {
            const inGame = [...sockets.values()].some((s) => !s.closed);
            if (document.hidden && inGame && wasm_exports) wasm_exports.frame();
        };
    }

    // ------------------------------------------------------------ imports

    function register(importObject) {
        const env = importObject.env;

        // ---- models: file loading and download progress (gridlock_web/models.js)

        registerModels(env);

        // ---- image decoding in a worker, after file loading (gridlock_web/decode.js)

        registerDecode(env);

        // ---- platform crate: clocks and randomness (platform/src/lib.rs)

        env.gridlock_now_ms = () => performance.now();
        env.gridlock_page_hidden = () => (document.hidden ? 1 : 0);

        // miniquad's loader uploads only the first matrix of a uniform array
        // (it ignores `count`), which the GPU skinning shader's joint array
        // needs whole (client/src/types/animated_model.rs).
        env.glUniformMatrix4fv = function (location, count, transpose, value) {
            const data = new Float32Array(wasm_memory.buffer, value, 16 * count);
            gl.uniformMatrix4fv(GL.uniforms[location], !!transpose, data);
        };
        env.gridlock_force_cpu_skinning = () => (new URLSearchParams(location.search).has("cpuskin") ? 1 : 0);
        env.gridlock_unix_ms = () => Date.now();
        env.gridlock_random_fill = function (ptr, len) {
            // getRandomValues fills at most 65536 bytes per call.
            for (let off = 0; off < len; off += 65536) {
                crypto.getRandomValues(new Uint8Array(wasm_memory.buffer, ptr + off, Math.min(65536, len - off)));
            }
        };

        // ---- platform::storage: settings and presets in localStorage

        const STORAGE_PREFIX = "gridlock:";
        const encoder = new TextEncoder();
        let storageTaken = null;
        env.gridlock_storage_get = function (keyPtr, keyLen, lenOut) {
            let value = null;
            try {
                value = localStorage.getItem(STORAGE_PREFIX + readString(keyPtr, keyLen));
            } catch {} // storage disabled (e.g. some private modes)
            if (value === null) return 0;
            storageTaken = encoder.encode(value);
            writeU32(lenOut, storageTaken.length);
            return 1;
        };
        env.gridlock_storage_take = function (ptr) {
            if (storageTaken) writeBytes(ptr, storageTaken);
            storageTaken = null;
        };
        env.gridlock_storage_set = function (keyPtr, keyLen, valuePtr, valueLen) {
            try {
                localStorage.setItem(STORAGE_PREFIX + readString(keyPtr, keyLen), readString(valuePtr, valueLen));
                return 1;
            } catch {
                return 0; // quota exceeded or storage disabled
            }
        };

        // ---- platform::page_setting: window.GRIDLOCK_<name>, set by the page
        // (the local server sets GRIDLOCK_REGISTRY). Bytes via gridlock_storage_take.

        env.gridlock_page_setting = function (namePtr, nameLen, lenOut) {
            const value = window["GRIDLOCK_" + readString(namePtr, nameLen)];
            if (typeof value !== "string") return 0;
            storageTaken = encoder.encode(value);
            writeU32(lenOut, storageTaken.length);
            return 1;
        };

        // ---- pointer lock (replaces miniquad's version)
        // The game re-asserts the grab often. Browsers grant pointer lock only from
        // a user gesture and reject rapid repeats, so remember what the game wants,
        // try once when that changes, and otherwise lock on the next click.

        let wantLock = false;
        const lock = () => {
            if (wantLock && document.pointerLockElement !== canvas) {
                try {
                    const p = canvas.requestPointerLock();
                    if (p && p.catch) p.catch(() => {});
                } catch {}
            }
        };
        canvas.addEventListener("mousedown", lock);
        env.sapp_set_cursor_grab = function (grab) {
            const want = grab !== 0;
            if (want === wantLock) return;
            wantLock = want;
            if (want) lock();
            else if (document.pointerLockElement === canvas) document.exitPointerLock();
        };

        // ---- client audio (client/src/audio.rs): Web Audio, with pitch via playbackRate

        env.gridlock_audio_load = function (ptr, len) {
            const id = nextHandle++;
            const bytes = readBytes(ptr, len);
            audio()
                .decodeAudioData(bytes.buffer)
                .then((buffer) => sounds.set(id, buffer))
                .catch((e) => console.warn("gridlock_web: could not decode sound", e));
            return id;
        };

        // Plays nothing until decoding finishes (it is fast for short effects).
        env.gridlock_audio_play = function (id, speed, volume) {
            const buffer = sounds.get(id);
            if (!buffer) return;
            const ctx = audio();
            const source = ctx.createBufferSource();
            const gain = ctx.createGain();
            source.buffer = buffer;
            source.playbackRate.value = speed;
            gain.gain.value = volume;
            source.connect(gain).connect(ctx.destination);
            source.start();
        };

        // ---- transport crate: WebRTC and fetch (transport/src/web.rs)

        env.gridlock_rtc_open = function (urlPtr, urlLen, isHost) {
            const handle = nextHandle++;
            const url = readString(urlPtr, urlLen);
            const ahead = isHost === 0 ? takePreconnected(url) : null;
            sockets.set(handle, ahead || new RtcSocket(url, isHost !== 0));
            keepRunningWhenHidden();
            return handle;
        };

        env.gridlock_rtc_preconnect = function (urlPtr, urlLen) {
            try {
                preconnect(readString(urlPtr, urlLen));
            } catch {}
        };

        env.gridlock_rtc_prewarm = function (urlPtr, urlLen) {
            try {
                prewarm(roomHttpUrl(readString(urlPtr, urlLen)));
            } catch {}
        };

        env.gridlock_rtc_send = function (handle, peer, ptr, len) {
            const socket = sockets.get(handle);
            if (socket) socket.send(peer, readBytes(ptr, len));
        };

        // Returns the next event kind (0 = none) and writes its peer and payload
        // length; the payload is then copied out with gridlock_rtc_take.
        env.gridlock_rtc_next = function (handle, peerOut, lenOut) {
            const socket = sockets.get(handle);
            const ev = socket && socket.events.shift();
            if (!ev) return 0;
            socket.taken = ev.data;
            writeU32(peerOut, ev.peer);
            writeU32(lenOut, ev.data ? ev.data.length : 0);
            return ev.kind;
        };

        env.gridlock_rtc_take = function (handle, ptr) {
            const socket = sockets.get(handle);
            if (socket && socket.taken) writeBytes(ptr, socket.taken);
            if (socket) socket.taken = null;
        };

        env.gridlock_rtc_close = function (handle) {
            const socket = sockets.get(handle);
            if (socket) socket.shutdown();
            sockets.delete(handle);
        };

        env.gridlock_http_start = function (methodPtr, methodLen, urlPtr, urlLen, bodyPtr, bodyLen, authPtr, authLen) {
            const id = nextHandle++;
            const url = readString(urlPtr, urlLen);
            // The game asking a registry for its host list is about to join or
            // host through it: have its ICE servers ready by then.
            if (/\/servers\/?$/.test(url)) iceServersFor(url);
            const req = { done: false, status: 0, body: new Uint8Array(0) };
            requests.set(id, req);
            const headers = { "Content-Type": "application/json" };
            if (authLen > 0) headers.Authorization = readString(authPtr, authLen);
            const method = readString(methodPtr, methodLen);
            fetch(url, {
                method,
                headers,
                body: bodyLen > 0 ? readBytes(bodyPtr, bodyLen) : undefined,
            })
                .then(async (resp) => {
                    req.body = new Uint8Array(await resp.arrayBuffer());
                    req.status = resp.status;
                })
                .catch(() => {
                    req.status = 0; // network error
                })
                .finally(() => {
                    req.done = true;
                });
            return id;
        };

        // 0 while in flight; 1 when done, with status (0 = network error) and body length written.
        env.gridlock_http_poll = function (id, statusOut, lenOut) {
            const req = requests.get(id);
            if (!req || !req.done) return 0;
            writeU32(statusOut, req.status);
            writeU32(lenOut, req.body.length);
            return 1;
        };

        env.gridlock_http_take = function (id, ptr) {
            const req = requests.get(id);
            if (req) writeBytes(ptr, req.body);
            requests.delete(id);
        };

        env.gridlock_http_cancel = function (id) {
            requests.delete(id);
        };
    }

    // Debugging aid: how each WebRTC connection is routed. From the browser console:
    //   await gridlockConnections()  ->  [{ peer, role, path: "direct" | "stun" | "relay", remote }]
    window.gridlockConnections = async function () {
        const result = [];
        for (const socket of sockets.values()) {
            for (const [peer, p] of socket.peers) {
                const stats = await p.pc.getStats();
                let pair = null;
                stats.forEach((s) => {
                    if (s.type === "candidate-pair" && s.nominated && s.state === "succeeded") pair = s;
                });
                const local = pair && stats.get(pair.localCandidateId);
                const remote = pair && stats.get(pair.remoteCandidateId);
                const types = [local?.candidateType, remote?.candidateType];
                const path = !pair ? "none" : types.includes("relay") ? "relay" : types.includes("srflx") || types.includes("prflx") ? "stun" : "direct";
                result.push({ peer, role: socket.isHost ? "host" : "player", path, remote: remote ? `${remote.address}:${remote.port}` : null });
            }
        }
        return result;
    };

    miniquad_add_plugin({
        name: "gridlock_web",
        version: PLUGIN_VERSION,
        register_plugin: register,
    });
})(window.gridlockWeb);
