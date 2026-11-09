"""
mixamo fbx -> glb for the game, run from the repo root inside blender with no window:

    blender --background --python scripts/mixamo_to_glb.py

reads   client/assets/models/New/Models/*.fbx      every file here is a character (with skin)
        client/assets/models/New/Animation/*.fbx   every file here is an animation (bones only)
writes  client/assets/models/characters/<name>.glb  mesh + skin + every texture, shrunk by type
        client/assets/models/animations/<name>.glb  just the skeleton + its animation, tiny

new skin = drop its fbx in New/Models and run again, nothing to list anywhere
files whose glb is already newer than the fbx are skipped, --force redoes everything
before this every animation was exported by hand with the whole character inside (~9 MB each)
"""

import argparse
import os
import sys

import bpy

ASSETS = os.path.join("client", "assets", "models")


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    p = argparse.ArgumentParser(description="convert mixamo fbx files to glb for gridlock")
    p.add_argument("--characters-in", default=os.path.join(ASSETS, "New", "Models"))
    p.add_argument("--animations-in", default=os.path.join(ASSETS, "New", "Animation"))
    p.add_argument("--out", default=ASSETS, help="characters/ and animations/ are made in here")
    p.add_argument("--diffuse-size", type=int, default=1024, help="max size of base colour textures")
    p.add_argument("--normal-size", type=int, default=512, help="max size of normal maps")
    p.add_argument("--other-size", type=int, default=256, help="max size of specular, gloss, emissive and anything else")
    p.add_argument("--force", action="store_true", help="convert everything again, even if up to date")
    return p.parse_args(argv)


def empty_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def import_fbx(path):
    # keep mixamo's bone axes as they are, the game matches bones by name
    if hasattr(bpy.ops.import_scene, "fbx"):
        bpy.ops.import_scene.fbx(filepath=path, automatic_bone_orientation=False)
    else:
        bpy.ops.wm.fbx_import(filepath=path)  # newer blender builds


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
        if img.size[0] == 0:
            continue  # missing or empty image, nothing to shrink
        kind = kinds.get(img.name, "other")
        shrink(img, sizes[kind])


def export_glb(path):
    bpy.ops.export_scene.gltf(filepath=path, export_format="GLB", export_animations=True)


def fbx_files(folder):
    if not os.path.isdir(folder):
        print(f"[convert] folder not found: {folder}")
        return []
    return sorted(f for f in os.listdir(folder) if f.lower().endswith(".fbx"))


def up_to_date(src, dst):
    return os.path.exists(dst) and os.path.getmtime(dst) >= os.path.getmtime(src)


def convert_folder(src_dir, dst_dir, kind, args, sizes, totals):
    os.makedirs(dst_dir, exist_ok=True)
    for name in fbx_files(src_dir):
        src = os.path.join(src_dir, name)
        dst = os.path.join(dst_dir, os.path.splitext(name)[0] + ".glb")
        if not args.force and up_to_date(src, dst):
            totals["skipped"] += 1
            totals["bytes"] += os.path.getsize(dst)
            continue
        try:
            empty_scene()
            import_fbx(src)
            if kind == "character":
                compress_textures(sizes)
            else:
                keep_only_armature()
            export_glb(dst)
        except Exception as e:  # one bad file shouldnt stop the rest
            print(f"[convert] FAILED {kind} {name}: {e}")
            totals["failed"].append(name)
            continue
        size = os.path.getsize(dst)
        totals["bytes"] += size
        totals[kind] += 1
        print(f"[convert] {kind:9} {name:32} -> {size / 1e6:6.2f} MB")


def main():
    args = parse_args()
    sizes = {"diffuse": args.diffuse_size, "normal": args.normal_size, "other": args.other_size}
    totals = {"character": 0, "animation": 0, "skipped": 0, "bytes": 0, "failed": []}
    convert_folder(args.characters_in, os.path.join(args.out, "characters"), "character", args, sizes, totals)
    convert_folder(args.animations_in, os.path.join(args.out, "animations"), "animation", args, sizes, totals)
    print(
        f"[convert] done: {totals['character']} characters, {totals['animation']} animations converted, "
        f"{totals['skipped']} already up to date, {totals['bytes'] / 1e6:.1f} MB total"
    )
    if totals["failed"]:
        print(f"[convert] failed: {', '.join(totals['failed'])}")
        sys.exit(1)


main()
