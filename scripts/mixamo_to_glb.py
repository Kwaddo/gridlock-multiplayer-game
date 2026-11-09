"""
mixamo fbx folder -> one glb per file, run inside blender with no window:

    blender --background --python scripts/mixamo_to_glb.py -- \
        --in C:/mixamo_raw --out C:/mixamo_glb --characters Ch15.fbx Skin_Test.fbx --texture-size 1024

the character file keeps its mesh, skin and diffuse (base colour) texture only,
every other file keeps just the skeleton + its animation (bones only, tiny files)
before this every animation was exported by hand with the whole character inside (~9 MB each)
"""

import argparse
import os
import sys

import bpy

# the only texture the game reads, everything else plugged into the shader gets cut
KEEP_INPUT = "Base Color"


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    p = argparse.ArgumentParser(description="convert mixamo fbx files to glb for gridlock")
    p.add_argument("--in", dest="src", required=True, help="folder with the .fbx files")
    p.add_argument("--out", dest="dst", required=True, help="folder to write the .glb files")
    p.add_argument("--characters", nargs="+", required=True, help="file names of the With Skin character fbx files (main + skins)")
    p.add_argument("--texture-size", type=int, default=1024, help="max diffuse size in pixels")
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


def strip_textures(max_size):
    # unplug every texture except base colour, the game is unlit so the rest is dead weight
    for mat in bpy.data.materials:
        if not mat.use_nodes:
            continue
        tree = mat.node_tree
        for link in list(tree.links):
            if link.from_node.type == "TEX_IMAGE" and link.to_socket.name != KEEP_INPUT:
                tree.links.remove(link)
        for node in list(tree.nodes):
            if node.type == "TEX_IMAGE" and not any(l.from_node == node for l in tree.links):
                tree.nodes.remove(node)
    # images nobody uses anymore, so the exporter doesnt write them
    for img in list(bpy.data.images):
        if img.users == 0:
            bpy.data.images.remove(img)
    # shrink the diffuse if its bigger than asked
    for img in bpy.data.images:
        w, h = img.size
        if max(w, h) > max_size:
            scale = max_size / max(w, h)
            img.scale(max(1, int(w * scale)), max(1, int(h * scale)))
            img.pack()


def export_glb(path):
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", export_animations=True)


def main():
    args = parse_args()
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
            strip_textures(args.texture_size)
            kind = "character"
        else:
            keep_only_armature()
            kind = "clip"
        export_glb(dst)
        size_mb = os.path.getsize(dst) / 1e6
        print(f"[mixamo] {kind:9} {name} -> {os.path.basename(dst)} ({size_mb:.2f} MB)")


main()
