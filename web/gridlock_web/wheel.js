// Mouse wheel in lines or pages, as pixels. miniquad's loader (mq_js_bundle.js,
// vendored) passes `deltaY` on as it comes, which is pixels in most browsers
// (about 100 a notch) but lines in Firefox with a mouse (3 a notch): there
// scrolling lists crawled at 3 px a notch. Such events are turned into pixels
// here and handed to the game in their place; pixel events pass untouched.
"use strict";

(function () {
    // Pixels for one line: 3 lines a notch, about the 100 px other browsers send.
    const LINE_PX = 33;

    window.addEventListener(
        "wheel",
        (e) => {
            if (e.deltaMode === 0 || typeof wasm_exports === "undefined" || !wasm_exports) return;
            if (!(e.target instanceof HTMLCanvasElement) || e.target.id !== "glcanvas") return;
            const k = e.deltaMode === 1 ? LINE_PX : window.innerHeight;
            // In capture on the window, before the loader's own listener on the canvas: it never sees this one.
            e.preventDefault();
            e.stopImmediatePropagation();
            wasm_exports.mouse_wheel(-e.deltaX * k, -e.deltaY * k);
        },
        { capture: true, passive: false },
    );
})();
