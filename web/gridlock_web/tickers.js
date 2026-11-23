// Gridlock's browser plugin: what keeps the game going apart from animation
// frames, in hidden tabs and for a game hosted in this tab. (See
// ../gridlock_web.js.)
"use strict";

(function (gridlockWeb) {
    const { sockets } = gridlockWeb;

    // A worker whose timer posts a message every `ms`, calling `onTick` with
    // each; null where workers are unavailable (e.g. a strict content policy).
    // Worker timers are not tied to animation frames, nor throttled in hidden
    // tabs. Each message is a task of its own: `onTick` never runs inside a
    // frame, nor a frame inside it.
    const tickerUrls = new Map(); // ms -> blob URL
    function startTicker(ms, onTick) {
        try {
            let url = tickerUrls.get(ms);
            if (!url) {
                url = URL.createObjectURL(new Blob([`setInterval(() => postMessage(0), ${ms});`], { type: "text/javascript" }));
                tickerUrls.set(ms, url);
            }
            const worker = new Worker(url);
            worker.onmessage = onTick;
            return worker;
        } catch {
            return null;
        }
    }

    // ------------------------------------------------------------ hidden tabs

    // The game advances once per animation frame, and browsers stop animation
    // frames in hidden tabs, which would cut the player off (no keepalives).
    // While hidden with a game open, a worker's ticks run the frames instead:
    // worker timers and the messages they post keep going in the background.
    // Those frames only keep the game and its connections going, without
    // drawing (see platform::page_hidden), so 20 a second is plenty. (A game
    // hosted here has a ticker of its own besides, below.)
    // The worker only runs while the tab is hidden (from the first game on):
    // a visible tab has its animation frames.
    const HIDDEN_FRAME_MS = 50;
    let hiddenTicker = null;
    let watchingVisibility = false;

    function keepRunningWhenHidden() {
        if (typeof Worker === "undefined") return;
        if (!watchingVisibility) {
            watchingVisibility = true;
            document.addEventListener("visibilitychange", updateHiddenTicker);
        }
        updateHiddenTicker();
    }

    function updateHiddenTicker() {
        if (!document.hidden) {
            if (hiddenTicker) hiddenTicker.terminate();
            hiddenTicker = null;
            return;
        }
        // (Without workers the game pauses when hidden.)
        hiddenTicker ??= startTicker(HIDDEN_FRAME_MS, () => {
            if (!document.hidden || !wasm_exports) return;
            let inGame = false;
            for (const s of sockets.values()) if (!s.closed) inGame = true;
            if (inGame) wasm_exports.frame();
        });
    }

    // ------------------------------------------------------------ hosting

    // A game hosted in this tab keeps its own clock, apart from the frames
    // (client/src/net/hosting/web.rs): a host drawing 20 frames a second, or
    // stalled on a big asset or a shader compile, would otherwise hand every
    // player a world that runs at 20 Hz, or freezes. This worker steps the
    // server every few ms, tab visible or hidden; the server ticks at 60 Hz
    // out of those steps and handles packets as they come in between them.
    // Messages that queued up behind a long frame cost one tick, then next to
    // nothing (no catching up). Without workers only the frames step it, as
    // they still do besides.
    const HOST_STEP_MS = 4;
    let hostTicker = null;

    function hostStep() {
        if (!wasm_exports) return;
        let hosting = 0;
        try {
            hosting = wasm_exports.gridlock_host_tick();
        } catch (e) {
            console.error("gridlock_web: the hosted server failed", e);
        }
        // Nothing hosted any more (or the game crashed): stop until the next host.
        if (!hosting && hostTicker) {
            hostTicker.terminate();
            hostTicker = null;
        }
    }

    // Called by LocalHost::start (through gridlock_host_ticker_start).
    function startHostTicker() {
        hostTicker ??= startTicker(HOST_STEP_MS, hostStep);
    }

    Object.assign(gridlockWeb, { keepRunningWhenHidden, startHostTicker });
})((window.gridlockWeb = window.gridlockWeb || {}));
