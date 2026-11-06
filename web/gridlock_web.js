// Browser networking for Gridlock, as a miniquad plugin (no wasm-bindgen).
//
// WebRTC: one data channel per peer, negotiated through the HTTP signalling
// mailboxes of the Gridlock registry (registry/dcs/index.js). The host is the
// offerer for every player; players only ever connect to the host. ICE servers
// (STUN, plus TURN relay credentials when configured) come from the registry's
// /ice route.
//
// HTTP: fetch() for the host list.
//
// Rust drives everything by polling (transport/src/web.rs). Load this file
// after mq_js_bundle.js and before calling load().
"use strict";

(function () {
    const PLUGIN_VERSION = 3;

    // Event kinds shared with transport/src/web.rs.
    const EV_PEER_JOINED = 1;
    const EV_PEER_LEFT = 2;
    const EV_PACKET = 3;
    const EV_CLOSED = 4;

    const FORCE_RELAY = new URLSearchParams(location.search).has("relay");
    // ?nodirect makes the direct attempt fail, to check the fallback to TURN.
    const NO_DIRECT = new URLSearchParams(location.search).has("nodirect");
    const STUN_FALLBACK = [{ urls: ["stun:stun.cloudflare.com:3478", "stun:stun.l.google.com:19302"] }];
    const POLL_HOST_MS = 2000;
    const POLL_CONNECTING_MS = 500;
    const POLL_CONNECTED_MS = 1000; // a connected player only checks whether to resume
    const HOST_KEEP_MS = 20000; // how often the host renews its claim on the room
    // A player not connected yet re-announces itself this often, so a
    // signalling write the registry lost (it has no atomic updates) only
    // costs a few seconds: the host resends its offer, the player its answer.
    const HELLO_MS = 6000;
    // An offer or answer goes out once its ICE candidates are gathered (one
    // write per side, nothing trickled), or after this long regardless.
    const GATHER_MS = 2500;
    const SEND_BATCH_MS = 50;
    const DISCONNECT_GRACE_MS = 3000;
    // TURN is the last resort: a connection first tries direct paths only
    // (STUN), and the host starts over with the relay when no direct path
    // opened this long after the answer arrived, or the attempt failed.
    const DIRECT_ATTEMPT_MS = 4000;

    const decoder = new TextDecoder();
    const sockets = new Map(); // handle -> RtcSocket
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

    function readString(ptr, len) {
        return decoder.decode(new Uint8Array(wasm_memory.buffer, ptr, len));
    }

    function readBytes(ptr, len) {
        return new Uint8Array(wasm_memory.buffer, ptr, len).slice();
    }

    function writeU32(ptr, value) {
        new Uint32Array(wasm_memory.buffer, ptr, 1)[0] = value >>> 0;
    }

    function writeBytes(ptr, bytes) {
        new Uint8Array(wasm_memory.buffer, ptr, bytes.length).set(bytes);
    }

    // ------------------------------------------------------------ WebRTC

    class RtcSocket {
        constructor(roomUrl, isHost) {
            this.isHost = isHost;
            this.events = [];
            this.taken = null; // payload of the event last returned by next()
            this.peers = new Map(); // small id -> { remoteId, pc, channel, pending }
            this.byRemote = new Map(); // signalling id -> small id
            this.nextPeer = 1;
            this.closed = false;
            this.me = null; // { id, token } from the room's join
            this.outbox = []; // signals waiting for the next send
            this.flushing = false;
            this.start(roomUrl);
        }

        // Signalling goes through HTTP mailboxes on the registry (registry/dcs/index.js):
        // join the room, then poll it for messages and post signals to it.
        async start(roomUrl) {
            this.room = roomHttpUrl(roomUrl);
            this.iceServers = await fetchIceServers(this.room);
            if (this.closed) return;
            try {
                const resp = await fetch(this.room + "/join" + (this.isHost ? "?host" : ""), { method: "POST" });
                if (!resp.ok) throw new Error("join refused: " + resp.status);
                this.me = await resp.json();
            } catch (e) {
                console.warn("gridlock_web: could not join room", e);
                return this.shutdown();
            }
            if (this.closed) return this.leave();
            this.onPageHide = () => this.leave();
            window.addEventListener("pagehide", this.onPageHide);
            this.poll();
        }

        async poll() {
            let lastKeep = Date.now();
            let lastHello = Date.now();
            const seen = new Set(); // ids of messages already handled
            while (!this.closed) {
                // A connected player needs no signalling: if the host goes,
                // the WebRTC connection itself reports it.
                const connected = !this.isHost && [...this.peers.values()].some((p) => p.open);
                if (!connected) {
                    try {
                        let q = `?id=${encodeURIComponent(this.me.id)}&token=${encodeURIComponent(this.me.token)}`;
                        if (this.isHost) {
                            q += "&host";
                            if (Date.now() - lastKeep >= HOST_KEEP_MS) {
                                q += "&keep";
                                lastKeep = Date.now();
                            }
                        }
                        const resp = await fetch(this.room + "/poll" + q, { cache: "no-store" });
                        // A host whose claim was lost claims the room again;
                        // players already connected are not affected.
                        if (resp.status === 410 && this.isHost && (await this.reclaim())) continue;
                        // 410: the host left; 403/404: this room is gone for us.
                        if (resp.status === 410 || resp.status === 403 || resp.status === 404) return this.shutdown();
                        if (resp.ok) {
                            for (const msg of await resp.json()) {
                                // Messages carry ids (repeated until they expire);
                                // a registry without ids sends each one once.
                                if (this.closed || (msg.id !== undefined && seen.has(msg.id))) continue;
                                if (msg.id !== undefined) seen.add(msg.id);
                                this.onSignal(msg);
                            }
                        }
                        if (!this.isHost && Date.now() - lastHello >= HELLO_MS) {
                            lastHello = Date.now();
                            this.post("/hello", {});
                        }
                    } catch {} // network blip: try again on the next poll
                }
                await sleep(this.pollDelay(connected));
            }
        }

        // Poll fast while a connection is being negotiated; an idle host polls
        // just often enough to notice newcomers.
        pollDelay(connected) {
            if ([...this.peers.values()].some((p) => !p.open)) return POLL_CONNECTING_MS;
            if (this.isHost) return POLL_HOST_MS;
            return connected ? POLL_CONNECTED_MS : POLL_CONNECTING_MS;
        }

        async reclaim() {
            try {
                const resp = await fetch(this.room + "/join?host", { method: "POST" });
                if (!resp.ok) return false;
                this.me = await resp.json();
                return true;
            } catch {
                return false;
            }
        }

        post(path, fields) {
            const body = JSON.stringify({ id: this.me.id, token: this.me.token, host: this.isHost, ...fields });
            return fetch(this.room + path, { method: "POST", body }).catch(() => {});
        }

        signal(msg) {
            if (!msg.Signal || this.closed) return;
            this.outbox.push(msg.Signal);
            // ICE candidates come in bursts; send each burst as one request.
            if (!this.flushing) {
                this.flushing = true;
                setTimeout(() => this.flush(), SEND_BATCH_MS);
            }
        }

        async flush() {
            const signals = this.outbox.splice(0);
            try {
                if (signals.length && this.me && !this.closed) {
                    await fetch(this.room + "/send", {
                        method: "POST",
                        body: JSON.stringify({ id: this.me.id, token: this.me.token, host: this.isHost, signals }),
                    });
                }
            } catch {}
            this.flushing = this.outbox.length > 0;
            if (this.flushing) setTimeout(() => this.flush(), SEND_BATCH_MS);
        }

        leave() {
            if (!this.me || this.left) return;
            this.left = true;
            const body = JSON.stringify({ id: this.me.id, token: this.me.token, host: this.isHost });
            // sendBeacon still goes out while the tab is closing.
            if (!navigator.sendBeacon(this.room + "/leave", body)) {
                fetch(this.room + "/leave", { method: "POST", body, keepalive: true }).catch(() => {});
            }
        }

        onSignal(msg) {
            if (msg.NewPeer) {
                // Heard again from a player already being offered to: the offer
                // (or its answer) went missing, so send the offer again.
                const known = this.peers.get(this.byRemote.get(msg.NewPeer));
                if (known && !known.open) this.sendDescription(known);
                else if (!known) this.connect(msg.NewPeer, true);
            } else if (msg.PeerLeft) {
                const id = this.byRemote.get(msg.PeerLeft);
                if (id !== undefined) this.drop(id);
            } else if (msg.Signal) {
                const { sender, data } = msg.Signal;
                let id = this.byRemote.get(sender);
                // The host offers to every player it learned of through NewPeer;
                // a signal from anyone else is a leftover of a dropped peer.
                if (id === undefined && this.isHost) return;
                // A newer offer (the host retrying through the relay) replaces
                // an attempt that never opened.
                const gen = data?.gen ?? 0;
                if (id !== undefined && data?.sdp?.type === "offer" && gen > this.peers.get(id).gen && !this.peers.get(id).open) {
                    this.discard(id);
                    id = undefined;
                }
                id ??= this.connect(sender, false, gen);
                this.onPeerSignal(this.peers.get(id), data);
            }
            // IdAssigned needs no action: we never address ourselves.
        }

        // `gen` counts attempts for this peer: 0 is direct only, 1 adds the
        // TURN relay. Signals carry it, so ones from an abandoned attempt are
        // ignored.
        connect(remoteId, offerer, gen = 0) {
            const id = this.nextPeer++;
            const relay = gen > 0 || FORCE_RELAY;
            const pc = new RTCPeerConnection({
                iceServers: relay ? this.iceServers : this.iceServers.filter(isStunOnly),
                // ?relay in the page URL forces every connection through TURN (for testing the relay).
                iceTransportPolicy: FORCE_RELAY || (NO_DIRECT && !relay) ? "relay" : "all",
            });
            const peer = { remoteId, pc, channel: null, pending: [], open: false, gen, offerer };
            this.peers.set(id, peer);
            this.byRemote.set(remoteId, id);

            pc.onconnectionstatechange = () => {
                const state = pc.connectionState;
                if (state === "failed" && this.retryWithRelay(id)) return;
                if (state === "failed" || state === "closed") this.drop(id);
                // A closed or crashed browser shows up as "disconnected" within
                // seconds, but "failed" only after ~30 s; brief network blips
                // recover well within this grace.
                if (state === "disconnected") {
                    setTimeout(() => {
                        if (pc.connectionState === "disconnected") this.drop(id);
                    }, DISCONNECT_GRACE_MS);
                }
            };

            if (offerer) {
                // Game traffic is UDP-like: unordered, no retransmits.
                this.attach(id, pc.createDataChannel("gridlock", { ordered: false, maxRetransmits: 0 }));
                pc.createOffer()
                    .then((offer) => pc.setLocalDescription(offer))
                    .then(() => gathered(pc))
                    .then(() => this.sendDescription(peer))
                    .catch(() => this.drop(id));
            } else {
                pc.ondatachannel = (e) => this.attach(id, e.channel);
            }
            return id;
        }

        // The host's direct attempt failed: start over with the TURN relay.
        retryWithRelay(id) {
            const peer = this.peers.get(id);
            if (!peer || !peer.offerer || peer.open || peer.gen > 0 || FORCE_RELAY || this.closed) return false;
            this.discard(id);
            this.connect(peer.remoteId, true, 1);
            return true;
        }

        // Forget a peer that never opened, without telling the game.
        discard(id) {
            const peer = this.peers.get(id);
            if (!peer) return;
            this.peers.delete(id);
            this.byRemote.delete(peer.remoteId);
            try {
                peer.pc.close();
            } catch {}
        }

        // Our offer or answer, with every ICE candidate gathered so far in it.
        sendDescription(peer) {
            if (!peer.pc.localDescription || this.closed) return;
            this.signal({ Signal: { receiver: peer.remoteId, data: { sdp: peer.pc.localDescription.toJSON(), gen: peer.gen } } });
        }

        async onPeerSignal(peer, data) {
            if (!peer || (data.gen ?? 0) !== peer.gen) return;
            const pc = peer.pc;
            try {
                if (data.sdp) {
                    // The same offer again: our answer went missing; resend it.
                    if (data.sdp.type === "offer" && pc.remoteDescription) {
                        if (pc.localDescription) this.sendDescription(peer);
                        return;
                    }
                    // An answer we already have (the host resent its offer).
                    if (data.sdp.type === "answer" && pc.remoteDescription) return;
                    await pc.setRemoteDescription(data.sdp);
                    for (const c of peer.pending) await pc.addIceCandidate(c);
                    peer.pending = [];
                    if (data.sdp.type === "offer") {
                        await pc.setLocalDescription(await pc.createAnswer());
                        await gathered(pc);
                        this.sendDescription(peer);
                    } else if (peer.offerer && peer.gen === 0) {
                        // The answer is in: give the direct attempt a few seconds.
                        const id = this.byRemote.get(peer.remoteId);
                        setTimeout(() => {
                            if (this.peers.get(id) === peer && !peer.open) this.retryWithRelay(id);
                        }, DIRECT_ATTEMPT_MS);
                    }
                } else if (data.candidate) {
                    // Candidates can arrive before the description they belong to.
                    if (pc.remoteDescription) await pc.addIceCandidate(data.candidate);
                    else peer.pending.push(data.candidate);
                }
            } catch (e) {
                console.warn("gridlock_web: signalling error", e);
            }
        }

        attach(id, channel) {
            const peer = this.peers.get(id);
            if (!peer) return;
            peer.channel = channel;
            channel.binaryType = "arraybuffer";
            channel.onopen = () => {
                peer.open = true;
                this.events.push({ kind: EV_PEER_JOINED, peer: id, data: null });
            };
            channel.onmessage = (e) => {
                if (e.data instanceof ArrayBuffer) {
                    this.events.push({ kind: EV_PACKET, peer: id, data: new Uint8Array(e.data) });
                }
            };
            channel.onclose = () => this.drop(id);
        }

        send(id, bytes) {
            const peer = this.peers.get(id);
            if (peer && peer.open && peer.channel.readyState === "open") {
                try {
                    peer.channel.send(bytes);
                } catch {
                    // send buffer full: drop the packet, like UDP would
                }
            }
        }

        drop(id) {
            const peer = this.peers.get(id);
            if (!peer) return;
            this.peers.delete(id);
            this.byRemote.delete(peer.remoteId);
            try {
                peer.pc.close();
            } catch {}
            if (peer.open) this.events.push({ kind: EV_PEER_LEFT, peer: id, data: null });
        }

        shutdown() {
            if (this.closed) return;
            this.closed = true;
            for (const id of [...this.peers.keys()]) this.drop(id);
            if (this.onPageHide) window.removeEventListener("pagehide", this.onPageHide);
            this.leave();
            this.events.push({ kind: EV_CLOSED, peer: 0, data: null });
        }
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
    function roomHttpUrl(roomUrl) {
        const url = new URL(roomUrl);
        url.protocol = url.protocol === "wss:" ? "https:" : "http:";
        url.search = "";
        return url.toString().replace(/\/$/, "");
    }

    // Resolves once a connection has gathered its ICE candidates, or after
    // GATHER_MS, whichever comes first.
    function gathered(pc) {
        if (pc.iceGatheringState === "complete") return Promise.resolve();
        return new Promise((resolve) => {
            const done = () => {
                pc.removeEventListener("icegatheringstatechange", check);
                resolve();
            };
            const check = () => pc.iceGatheringState === "complete" && done();
            pc.addEventListener("icegatheringstatechange", check);
            setTimeout(done, GATHER_MS);
        });
    }

    function isStunOnly(server) {
        return [].concat(server.urls).every((u) => u.startsWith("stun:"));
    }

    async function fetchIceServers(room) {
        try {
            const url = new URL(room);
            url.pathname = "/ice";
            const resp = await fetch(url);
            const body = await resp.json();
            // An empty list is deliberate: the local server (registry/local/) has
            // no STUN to offer offline, and LAN peers connect without one.
            if (Array.isArray(body.iceServers)) return body.iceServers;
        } catch {}
        return STUN_FALLBACK;
    }

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
            sockets.set(handle, new RtcSocket(readString(urlPtr, urlLen), isHost !== 0));
            keepRunningWhenHidden();
            return handle;
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
            const req = { done: false, status: 0, body: new Uint8Array(0) };
            requests.set(id, req);
            const headers = { "Content-Type": "application/json" };
            if (authLen > 0) headers.Authorization = readString(authPtr, authLen);
            const method = readString(methodPtr, methodLen);
            fetch(readString(urlPtr, urlLen), {
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
})();
