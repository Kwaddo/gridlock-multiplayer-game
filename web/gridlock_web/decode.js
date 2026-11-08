// Gridlock's browser plugin, part 9 of 9: image decoding off the main
// thread. registerDecode adds its imports. (See ../gridlock_web.js.)
//
// The browser build has no threads, so decoding a model's PNG/JPEG textures
// in wasm stalls the frame (and a game hosted in this tab) for as long as it
// takes. Instead, for the files the game asks for (gridlock_decode_ahead), a
// worker decodes the images once the file has downloaded, and only then is
// the file reported loaded. The game takes the decoded pixels
// (gridlock_decoded_size / _copy) and only uploads them. Anything the worker
// could not decode, the game decodes itself as before.
"use strict";

(function (gridlockWeb) {
    const { readString, writeU32 } = gridlockWeb;

    const KIND_GLB = 1;
    const KIND_IMAGE = 2;

    // Runs in the worker. Message in: { id, kind, bytes }. Message out:
    // { id, images: [{ index, key, width, height, rgba }] }, where `key`
    // names the image's content: GLBs often embed the same textures, which
    // are decoded once (`rgba` is absent for one already decoded).
    function workerMain() {
        const decoded = new Set(); // keys already decoded
        const canDecode = typeof createImageBitmap === "function" && typeof OffscreenCanvas === "function";

        function contentKey(bytes) {
            let fnv = 0x811c9dc5;
            let djb = 5381;
            for (let i = 0; i < bytes.length; i++) {
                fnv = Math.imul(fnv ^ bytes[i], 0x01000193);
                djb = (Math.imul(djb, 33) + bytes[i]) | 0;
            }
            return `${fnv >>> 0}:${djb >>> 0}:${bytes.length}`;
        }

        // The embedded images of a .glb, as [{ index, mime, bytes }]; only
        // those stored in its binary chunk (the game supports no others).
        function glbImages(bytes) {
            const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
            if (bytes.length < 20 || view.getUint32(0, true) !== 0x46546c67) return [];
            let json = null;
            let bin = null;
            for (let at = 12; at + 8 <= bytes.length; ) {
                const length = view.getUint32(at, true);
                const type = view.getUint32(at + 4, true);
                const data = bytes.subarray(at + 8, at + 8 + length);
                if (type === 0x4e4f534a) json = JSON.parse(new TextDecoder().decode(data));
                else if (type === 0x004e4942) bin = data;
                at += 8 + length;
            }
            if (!json || !bin) return [];
            const out = [];
            (json.images || []).forEach((image, index) => {
                const bv = json.bufferViews?.[image.bufferView];
                if (!bv || bv.buffer !== 0 || json.buffers?.[0]?.uri !== undefined) return;
                const start = bv.byteOffset || 0;
                if (start + bv.byteLength > bin.length) return;
                out.push({ index, mime: image.mimeType || "", bytes: bin.subarray(start, start + bv.byteLength) });
            });
            return out;
        }

        // RGBA pixels, or null. Images with any transparency are left to the
        // game: a canvas stores premultiplied alpha, which would round them.
        async function decode(bytes, mime) {
            const bitmap = await createImageBitmap(new Blob([bytes], { type: mime }), {
                premultiplyAlpha: "none",
                colorSpaceConversion: "none",
            });
            const { width, height } = bitmap;
            const ctx = new OffscreenCanvas(width, height).getContext("2d", { willReadFrequently: true });
            ctx.drawImage(bitmap, 0, 0);
            bitmap.close();
            const rgba = ctx.getImageData(0, 0, width, height).data;
            for (let i = 3; i < rgba.length; i += 4) {
                if (rgba[i] !== 255) return null;
            }
            return { width, height, rgba };
        }

        self.onmessage = async ({ data: { id, kind, bytes } }) => {
            const images = [];
            const transfer = [];
            try {
                // 1: a .glb; otherwise the file is the image (index 0xffffffff).
                const sources = kind === 1 ? glbImages(bytes) : [{ index: 0xffffffff, mime: "", bytes }];
                for (const source of sources) {
                    const key = contentKey(source.bytes);
                    const entry = { index: source.index, key };
                    if (canDecode && !decoded.has(key)) {
                        decoded.add(key);
                        const image = await decode(source.bytes, source.mime).catch(() => null);
                        if (image) {
                            Object.assign(entry, image);
                            transfer.push(image.rgba.buffer);
                        }
                    }
                    images.push(entry);
                }
            } catch {} // not a valid file: the game reports it
            self.postMessage({ id, images }, transfer);
        };
    }

    let worker = null; // null: not started; false: workers unavailable
    const waiting = new Map(); // message id -> resolve
    let nextId = 1;

    function decoder() {
        if (worker === null) {
            try {
                const source = `(${workerMain.toString()})();`;
                worker = new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
                worker.onmessage = ({ data }) => {
                    waiting.get(data.id)?.(data.images);
                    waiting.delete(data.id);
                };
                worker.onerror = () => {
                    // Broken worker: whatever was waiting loads undecoded.
                    for (const resolve of waiting.values()) resolve([]);
                    waiting.clear();
                    worker = false;
                };
            } catch {
                worker = false; // e.g. a strict content policy: the game decodes itself
            }
        }
        return worker || null;
    }

    // Decoded pixels by content key, until the game takes them; and which key
    // each (path, image index) holds.
    const pixels = new Map(); // key -> { width, height, rgba }
    const keyOf = new Map(); // `${path}#${index}` -> key

    function decodeAhead(path, kind, bytes) {
        const w = decoder();
        if (!w) return Promise.resolve();
        const id = nextId++;
        return new Promise((resolve) => {
            waiting.set(id, (images) => {
                for (const image of images) {
                    keyOf.set(`${path}#${image.index}`, image.key);
                    if (image.rgba) pixels.set(image.key, image);
                }
                resolve();
            });
            // A copy (moved, not copied again): the original goes to the game.
            const copy = bytes.slice();
            w.postMessage({ id, kind, bytes: copy }, [copy.buffer]);
        });
    }

    function decodedFor(pathPtr, pathLen, index) {
        const key = keyOf.get(`${readString(pathPtr, pathLen)}#${index >>> 0}`);
        return key === undefined ? null : { key, image: pixels.get(key) };
    }

    function registerDecode(env) {
        const wanted = new Map(); // path -> kind, for the next load of it

        // Decode the images in `path` ahead when it is next loaded.
        env.gridlock_decode_ahead = function (ptr, len, kind) {
            if (kind === KIND_GLB || kind === KIND_IMAGE) wanted.set(readString(ptr, len), kind);
        };

        const loadInner = env.fs_load_file;
        env.fs_load_file = function (ptr, len) {
            const path = UTF8ToString(ptr, len);
            const kind = wanted.get(path);
            if (!kind || !decoder()) return loadInner(ptr, len);
            wanted.delete(path);
            const id = FS.unique_id++;
            gridlockWeb
                .loadFile(path)
                .then((bytes) => (bytes ? decodeAhead(path, kind, bytes).then(() => bytes) : null))
                .catch(() => null)
                .then((bytes) => {
                    FS.loaded_files[id] = bytes;
                    wasm_exports.file_loaded(id);
                });
            return id;
        };

        // Image `index` of `path` (0xffffffff: the file is the image), when the
        // worker decoded it: 1, with its width and height as two u32s at outPtr.
        env.gridlock_decoded_size = function (pathPtr, pathLen, index, outPtr) {
            const found = decodedFor(pathPtr, pathLen, index);
            if (!found?.image) return 0;
            writeU32(outPtr, found.image.width);
            writeU32(outPtr + 4, found.image.height);
            return 1;
        };
        // Copy those pixels (width * height * 4 bytes) to dstPtr and forget them.
        env.gridlock_decoded_copy = function (pathPtr, pathLen, index, dstPtr) {
            const found = decodedFor(pathPtr, pathLen, index);
            if (!found?.image) return;
            new Uint8Array(wasm_memory.buffer, dstPtr, found.image.rgba.length).set(found.image.rgba);
            pixels.delete(found.key);
        };
        // Drop everything not taken (images no mesh used).
        env.gridlock_decoded_clear = function () {
            pixels.clear();
            keyOf.clear();
            wanted.clear();
        };
    }

    Object.assign(gridlockWeb, { registerDecode });
})((window.gridlockWeb = window.gridlockWeb || {}));
