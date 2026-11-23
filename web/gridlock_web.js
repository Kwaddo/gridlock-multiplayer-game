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
        PLUGIN_VERSION, sockets, readString, readBytes, writeU32, writeBytes, heapFloats, roomHttpUrl, iceServersFor, prewarm,
        RtcSocket, preconnect, takePreconnected, registerModels, registerDecode, keepRunningWhenHidden, startHostTicker,
        EV_PACKET, RELIABLE_MIN_BYTES,
    } = gridlockWeb;

    const requests = new Map(); // id -> { done, status, body }
    const sounds = new Map(); // id -> AudioBuffer
    let nextHandle = 1;

    // Browsers keep audio suspended until the user interacts with the page.
    let audioContext = null;
    function audio() {
        if (!audioContext) {
            audioContext = new AudioContext();
            // Kept for good (a context can be suspended again), but only asks
            // while it is not running: every key and click comes through here.
            const resume = () => audioContext.state !== "running" && audioContext.resume();
            for (const ev of ["pointerdown", "keydown", "touchstart"]) {
                window.addEventListener(ev, resume, { capture: true });
            }
        }
        return audioContext;
    }

    // ------------------------------------------------------------ socket events

    // A socket's events wait for the game to drain them (gridlock_rtc_drain).
    // Should it stop (a hidden tab without workers, say), at most EVENTS_MAX
    // stay: past that, the oldest small packet goes, as it came unreliably
    // anyway. Big packets came over the reliable channel, and peer events
    // are state: those are never dropped.
    const EVENTS_MAX = 4096;
    function capEvents(events) {
        if (events.push !== Array.prototype.push) return; // capped already
        events.push = function (ev) {
            if (this.length >= EVENTS_MAX) {
                const i = this.findIndex((e) => e.kind === EV_PACKET && e.data.length <= RELIABLE_MIN_BYTES);
                if (i >= 0) this.splice(i, 1);
            }
            return Array.prototype.push.call(this, ev);
        };
    }

    const EVENT_HEADER = 12;
    const eventHeaderBytes = new Uint8Array(EVENT_HEADER);
    const eventHeader = new DataView(eventHeaderBytes.buffer);

    // ------------------------------------------------------------ imports

    function register(importObject) {
        const env = importObject.env;

        // ---- WebGL context attributes. The loader asks for getContext("webgl")
        // with none, so the browser multisamples (antialias), keeps an alpha
        // channel and may pick the low-power GPU. The native build renders
        // without MSAA; match it. `?aa` in the URL keeps antialiasing on.

        const canvas = document.querySelector("#glcanvas");
        if (canvas) {
            const getContext = canvas.getContext.bind(canvas);
            const antialias = new URLSearchParams(location.search).has("aa");
            canvas.getContext = (type, attrs) =>
                getContext(type, { antialias, alpha: false, powerPreference: "high-performance", ...attrs });
        }

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
        // WebGL2 reads the matrices straight out of the wasm memory (no new
        // view per call); WebGL1 has no offset argument and gets a view.
        let uniformGl = null;
        let uniformWebgl2 = false;
        env.glUniformMatrix4fv = function (location, count, transpose, value) {
            if (uniformGl !== gl) {
                uniformGl = gl;
                uniformWebgl2 = typeof WebGL2RenderingContext !== "undefined" && gl instanceof WebGL2RenderingContext;
            }
            const floats = heapFloats();
            const at = value >>> 2;
            if (uniformWebgl2) gl.uniformMatrix4fv(GL.uniforms[location], !!transpose, floats, at, 16 * count);
            else gl.uniformMatrix4fv(GL.uniforms[location], !!transpose, floats.subarray(at, at + 16 * count));
        };
        // miniquad's loader copies a loaded file into wasm memory one byte at a
        // time, which for a model of several MB takes a frame or more.
        env.fs_take_buffer = function (id, ptr, max) {
            const file = FS.loaded_files[id];
            if (file) new Uint8Array(wasm_memory.buffer, ptr, max).set(file.length > max ? file.subarray(0, max) : file);
            delete FS.loaded_files[id];
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
        env.sapp_set_cursor_grab = function (grab) {
            const want = grab !== 0;
            if (want === wantLock) return;
            wantLock = want;
            if (want) lock();
            else if (document.pointerLockElement === canvas) document.exitPointerLock();
        };

        // ---- keeping the keyboard in the game
        // Browsers keep shortcuts like Ctrl+W/T/N (close or open tabs) for
        // themselves, and Ctrl+W is easy to hit while moving. No page can
        // cancel those, except through the Keyboard Lock API (Chrome, Edge,
        // Opera), which only works in fullscreen. So the click that grabs the
        // mouse for play also goes fullscreen with every key but Esc locked:
        // Ctrl shortcuts stay with the game, and Esc still gets the player out
        // (frees the mouse and leaves fullscreen; the game pauses, below).
        // Browsers without it (Firefox, Safari) get the fallbacks below: the
        // tab asks before closing during a game.

        const keyboardLock = navigator.keyboard && navigator.keyboard.lock ? navigator.keyboard : null;
        // false once the game itself leaves fullscreen (its fullscreen key),
        // so play stays windowed until it asks for fullscreen again
        let autoFullscreen = true;

        // Every key code a keyboard sends, except Escape.
        const LOCKED_KEYS = [
            ..."ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").map((c) => "Key" + c),
            ..."0123456789".split("").map((d) => "Digit" + d),
            ..."0123456789".split("").map((d) => "Numpad" + d),
            ...Array.from({ length: 12 }, (_, i) => "F" + (i + 1)),
            "Space", "Tab", "Enter", "Backspace", "Delete", "Insert", "Home", "End",
            "PageUp", "PageDown", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight",
            "Minus", "Equal", "BracketLeft", "BracketRight", "Backslash", "Semicolon",
            "Quote", "Backquote", "Comma", "Period", "Slash", "IntlBackslash",
            "NumpadAdd", "NumpadSubtract", "NumpadMultiply", "NumpadDivide",
            "NumpadDecimal", "NumpadEnter",
        ];
        const lockKeys = () => {
            if (!keyboardLock) return;
            const p = keyboardLock.lock(LOCKED_KEYS);
            if (p && p.catch) p.catch(() => {});
        };
        const enterPlay = () => {
            if (!wantLock || !keyboardLock || !autoFullscreen || document.fullscreenElement) return;
            if (!canvas.requestFullscreen) return;
            try {
                const p = canvas.requestFullscreen({ navigationUI: "hide" });
                if (p && p.then) p.then(lockKeys, () => {});
            } catch {}
        };
        document.addEventListener("fullscreenchange", () => {
            if (document.fullscreenElement) lockKeys();
            else if (keyboardLock) keyboardLock.unlock();
        });

        canvas.addEventListener("mousedown", () => {
            enterPlay(); // first: fullscreen needs the click's user activation
            lock();
        });

        // miniquad's fullscreen toggle, remembering what the game chose
        env.sapp_set_fullscreen = function (on) {
            autoFullscreen = on !== 0;
            try {
                if (on) {
                    const p = canvas.requestFullscreen();
                    if (p && p.catch) p.catch(() => {});
                } else if (document.fullscreenElement) {
                    document.exitFullscreen();
                }
            } catch {}
        };

        // Shortcuts a page may cancel (Ctrl+S, Ctrl+D, Ctrl+F, Ctrl+R, Ctrl+P...)
        // stay with the game while it is played. The key still reaches the game.
        window.addEventListener(
            "keydown",
            (e) => {
                if (wantLock && (e.ctrlKey || e.metaKey || e.altKey)) e.preventDefault();
            },
            { capture: true },
        );

        // During a game, closing the tab (Ctrl+W where nothing else can stop
        // it) asks first instead of dropping the player out of the match.
        window.addEventListener("beforeunload", (e) => {
            const inGame = [...sockets.values()].some((s) => !s.closed);
            if (!inGame) return;
            e.preventDefault();
            e.returnValue = "";
        });

        // Esc (never locked) takes pointer lock away before the game hears the
        // key: pass it on, so the game pauses as Esc does natively.
        const ESCAPE = 256; // miniquad's key code
        document.addEventListener("pointerlockchange", () => {
            if (document.pointerLockElement === canvas || !wantLock || !wasm_exports) return;
            wasm_exports.key_down(ESCAPE, 0, false);
            wasm_exports.key_up(ESCAPE, 0);
        });

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
        // `pan` is -1 (left) to 1 (right), for sounds placed around the player.
        env.gridlock_audio_play = function (id, speed, volume, pan) {
            const buffer = sounds.get(id);
            if (!buffer) return;
            const ctx = audio();
            const source = ctx.createBufferSource();
            const gain = ctx.createGain();
            source.buffer = buffer;
            source.playbackRate.value = speed;
            gain.gain.value = volume;
            let node = source.connect(gain);
            if (pan && ctx.createStereoPanner) {
                const panner = ctx.createStereoPanner();
                panner.pan.value = Math.max(-1, Math.min(1, pan));
                node = node.connect(panner);
            }
            node.connect(ctx.destination);
            source.start();
        };

        // ---- transport crate: WebRTC and fetch (transport/src/web.rs)

        // A game hosted here steps on a timer of its own (gridlock_web/tickers.js).
        env.gridlock_host_ticker_start = startHostTicker;

        env.gridlock_rtc_open = function (urlPtr, urlLen, isHost) {
            const handle = nextHandle++;
            const url = readString(urlPtr, urlLen);
            const ahead = isHost === 0 ? takePreconnected(url) : null;
            const socket = ahead || new RtcSocket(url, isHost !== 0);
            capEvents(socket.events);
            sockets.set(handle, socket);
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

        // One packet to each of n peers (u32s at peersPtr): a broadcast is one
        // call, not one per player.
        env.gridlock_rtc_send_many = function (handle, peersPtr, n, ptr, len) {
            const socket = sockets.get(handle);
            if (!socket) return;
            // A view, not a copy: a data channel's send copies what it sends
            // at once (RtcSocket.send keeps nothing).
            const bytes = new Uint8Array(wasm_memory.buffer, ptr, len);
            const peers = new Uint32Array(wasm_memory.buffer, peersPtr, n);
            for (let i = 0; i < n; i++) socket.send(peers[i], bytes);
        };

        // Moves a socket's events into wasm memory at ptr (cap bytes), as many
        // as fit, each as kind, peer and payload length (little-endian u32s)
        // then the payload (transport/src/rtc_event.rs). Returns the bytes
        // written, and writes to needOut the size of the first event left
        // (0: none left).
        env.gridlock_rtc_drain = function (handle, ptr, cap, needOut) {
            const events = sockets.get(handle)?.events ?? [];
            const size = (ev) => EVENT_HEADER + (ev.data ? ev.data.length : 0);
            let at = 0;
            let taken = 0;
            for (const ev of events) {
                if (at + size(ev) > cap) break;
                eventHeader.setUint32(0, ev.kind, true);
                eventHeader.setUint32(4, ev.peer, true);
                eventHeader.setUint32(8, size(ev) - EVENT_HEADER, true);
                writeBytes(ptr + at, eventHeaderBytes);
                if (ev.data) writeBytes(ptr + at + EVENT_HEADER, ev.data);
                at += size(ev);
                taken++;
            }
            events.splice(0, taken);
            writeU32(needOut, events.length ? size(events[0]) : 0);
            return at;
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
            // Content-Type only with a body: a GET without one is then a
            // "simple" request, which a registry on another origin answers
            // without a CORS preflight first.
            const headers = bodyLen > 0 ? { "Content-Type": "application/json" } : {};
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
