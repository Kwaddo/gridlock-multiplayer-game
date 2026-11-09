"""
mixamo fbx folder -> one glb per file, run inside blender with no window:

    blender --background --python scripts/mixamo_to_glb.py -- \
        --in C:/mixamo_raw --out C:/mixamo_glb --characters Ch15.fbx Skin_Test.fbx

character files keep their mesh, skin and every texture, each texture shrunk by type
(diffuse 1024, normal 512, the rest 256 by default) so lighting can use them later,
every other file keeps just the skeleton + its animation (bones only, tiny files)
before this every animation was exported by hand with the whole character inside (~9 MB each)
"""

import argparse
import os
import sys

import bpy


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    p = argparse.ArgumentParser(description="convert mixamo fbx files to glb for gridlock")
    p.add_argument("--in", dest="src", required=True, help="folder with the .fbx files")
    p.add_argument("--out", dest="dst", required=True, help="folder to write the .glb files")
    p.add_argument("--characters", nargs="+", required=True, help="file names of the With Skin character fbx files (main + skins)")
    p.add_argument("--diffuse-size", type=int, default=1024, help="max size of base colour textures")
    p.add_argument("--normal-size", type=int, default=512, help="max size of normal maps")
    p.add_argument("--other-size", type=int, default=256, help="max size of specular, gloss, emissive and anything else")
    return p.parse_args(argv)


def empty_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def import_fbx(path):
    # keep mixamo's bone axes as they are, the game matches bones by name
    bpy.ops.import_scene.fbx(filepath=path, automatic_bone_orientation=False)


def keep_only_armature():
    # animation file, drop any mesh that came along (downloaded with skin by mistake)
    for obj in list(bpy.data.objects):
        if obj.type != "ARMATURE":
            bpy.data.objects.remove(obj, do_unlink=True)


def texture_kind(tree, node):
    # what an image feeds, base colour = diffuse, through a normal map node = normal, anything else = other
    for link in tree.links:
        if link.from_node != node:
            continue
        if link.to_node.type == "NORMAL_MAP" or link.to_socket.name == "Normal":
            return "normal"
        if link.to_socket.name == "Base Color":
            return "diffuse"
    return "other"


def shrink(img, max_size):
    w, h = img.size
    if max(w, h) > max_size:
        scale = max_size / max(w, h)
        img.scale(max(1, int(w * scale)), max(1, int(h * scale)))
        img.pack()


def compress_textures(sizes):
    # keep every map (normal, specular...) but shrink each one by what it is used for
    # before this the whole set came in at full size, up to 4096 for one specular map
    kinds = {}
    for mat in bpy.data.materials:
        if not mat.use_nodes:
            continue
        for node in mat.node_tree.nodes:
            if node.type == "TEX_IMAGE" and node.image:
                kind = texture_kind(mat.node_tree, node)
                # an image used for colour anywhere stays at the colour size
                if kinds.get(node.image.name) != "diffuse":
                    kinds[node.image.name] = kind
    for img in bpy.data.images:
        kind = kinds.get(img.name, "other")
        shrink(img, sizes[kind])
        print(f"[mixamo]   texture {img.name}: {kind} -> {img.size[0]}x{img.size[1]}")


def export_glb(path):
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", export_animations=True)


def main():
    args = parse_args()
    sizes = {"diffuse": args.diffuse_size, "normal": args.normal_size, "other": args.other_size}
    os.makedirs(args.dst, exist_ok=True)
    files = sorted(f for f in os.listdir(args.src) if f.lower().endswith(".fbx"))
    missing = [c for c in args.characters if c not in files]
    if missing:
        print(f"[mixamo] character file(s) {missing} not found in {args.src}")
        sys.exit(1)

    for name in files:
        src = os.path.join(args.src, name)
        dst = os.path.join(args.dst, os.path.splitext(name)[0] + ".glb")
        empty_scene()
        import_fbx(src)
        if name in args.characters:
            compress_textures(sizes)
            kind = "character"
        else:
            keep_only_armature()
            kind = "clip"
        export_glb(dst)
        size_mb = os.path.getsize(dst) / 1e6
        print(f"[mixamo] {kind:9} {name} -> {os.path.basename(dst)} ({size_mb:.2f} MB)")


main()
