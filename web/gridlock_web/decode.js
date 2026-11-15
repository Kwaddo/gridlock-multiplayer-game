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

    // The images to decode in a .glb, as [{ index, mime, bytes }]; only those
    // stored in its binary chunk (the game supports no others). Each `bytes`
    // is a copy of just that image, to move to the worker: the file itself
    // goes to the game, and copying all of it (most of it meshes and maps the
    // worker never reads) cost more than reading its JSON here.
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
        // Only the images a material takes its colour from: the game
        // samples no others (client/src/app/obj_loader.rs,
        // animated_model/parts.rs), and a model often embeds big normal
        // and roughness maps too. Any other the game asks for, it decodes.
        const used = new Set();
        for (const material of json.materials || []) {
            const infos = [
                material?.pbrMetallicRoughness?.baseColorTexture,
                material?.extensions?.KHR_materials_pbrSpecularGlossiness?.diffuseTexture,
            ];
            for (const info of infos) {
                const source = json.textures?.[info?.index]?.source;
                if (source !== undefined) used.add(source);
            }
        }
        const out = [];
        (json.images || []).forEach((image, index) => {
            if (!used.has(index)) return;
            const bv = json.bufferViews?.[image.bufferView];
            if (!bv || bv.buffer !== 0 || json.buffers?.[0]?.uri !== undefined) return;
            const start = bv.byteOffset || 0;
            if (start + bv.byteLength > bin.length) return;
            out.push({ index, mime: image.mimeType || "", bytes: bin.slice(start, start + bv.byteLength) });
        });
        return out;
    }

    // Runs in the worker. Message in: { id, sources: [{ index, mime, bytes }] }
    // (see glbImages). Messages out:
    // { pixels: { key, width, height, rgba } } for each image as soon as it
    // is decoded, then { id, images: [{ index, key }] } for the file, where
    // `key` names the image's content: GLBs often embed the same textures,
    // which are decoded once. A file's reply goes out only once each of its
    // images' pixels did (or could not be decoded), also when another file
    // was decoding that image already.
    function workerMain() {
        const decoded = new Map(); // key -> Promise, settled once its pixels went out (or not)
        const canDecode = typeof createImageBitmap === "function" && typeof OffscreenCanvas === "function";
        // Hashing natively is far faster; only a secure context has it (not
        // a plain-http LAN page), which gets the loop below. The key never
        // leaves this page.
        const subtle = self.isSecureContext ? self.crypto?.subtle : undefined;

        async function contentKey(bytes) {
            if (subtle) {
                try {
                    const digest = new Uint8Array(await subtle.digest("SHA-1", bytes));
                    let hex = "";
                    for (const b of digest) hex += (b < 16 ? "0" : "") + b.toString(16);
                    return `${hex}:${bytes.length}`;
                } catch {}
            }
            let fnv = 0x811c9dc5;
            let djb = 5381;
            for (let i = 0; i < bytes.length; i++) {
                fnv = Math.imul(fnv ^ bytes[i], 0x01000193);
                djb = (Math.imul(djb, 33) + bytes[i]) | 0;
            }
            return `${fnv >>> 0}:${djb >>> 0}:${bytes.length}`;
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

        async function decodeOut(key, source) {
            const image = await decode(source.bytes, source.mime).catch(() => null);
            if (image) self.postMessage({ pixels: { key, ...image } }, [image.rgba.buffer]);
        }

        self.onmessage = async ({ data: { id, sources } }) => {
            const images = [];
            try {
                for (const source of sources) {
                    const key = await contentKey(source.bytes);
                    images.push({ index: source.index, key });
                    if (!canDecode) continue;
                    if (!decoded.has(key)) decoded.set(key, decodeOut(key, source).catch(() => {}));
                    await decoded.get(key);
                }
            } catch {} // not a valid file: the game reports it
            self.postMessage({ id, images });
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
                    if (data.pixels) {
                        pixels.set(data.pixels.key, data.pixels);
                        return;
                    }
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
        let sources = [];
        try {
            // A .glb's images; otherwise the file is the image (index
            // 0xffffffff). Copies (moved, not copied again): the original
            // goes to the game.
            sources = kind === KIND_GLB ? glbImages(bytes) : [{ index: 0xffffffff, mime: "", bytes: bytes.slice() }];
        } catch {} // not a valid file: the game reports it
        if (!sources.length) return Promise.resolve();
        const id = nextId++;
        return new Promise((resolve) => {
            waiting.set(id, (images) => {
                // (their pixels came on their own, before this)
                for (const image of images) keyOf.set(`${path}#${image.index}`, image.key);
                resolve();
            });
            w.postMessage({ id, sources }, sources.map((source) => source.bytes.buffer));
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
