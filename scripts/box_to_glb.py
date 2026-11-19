"""
cardboard box models -> glb for the game, run from the repo root inside blender with no window:

    blender --background --python scripts/box_to_glb.py

reads   model_source/cardboard/...   (git ignored)
writes  client/assets/models/props/cardboard_box_<name>.glb

every box comes out the same size as the code-built one (1.05 x 0.72 x 0.75 m, long side along x,
bottom centre at the origin) so the game draws them all the same way, and only the colour texture
is kept since the game ignores the rest
"""

import os
import sys

import bpy
from mathutils import Vector

W, H, D = 1.05, 0.72, 0.75
TEXTURE_SIZE = 512
OUT = os.path.join("client", "assets", "models", "props")
SRC = os.path.join("model_source", "cardboard")

BOXES = [
    ("lowpoly", os.path.join(SRC, "lowpoly_cardboard_box", "source", "Cardboard Box.fbx"),
     os.path.join(SRC, "lowpoly_cardboard_box", "textures", "Box_Diffuse.png")),
    ("old", os.path.join(SRC, "old_cardboard_box", "source", "model", "model.dae"),
     os.path.join(SRC, "old_cardboard_box", "textures", "DefaultMaterial_albedo.jpeg")),
]


def empty_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)


def load(path):
    if path.endswith(".fbx"):
        bpy.ops.import_scene.fbx(filepath=path)
    else:
        load_dae(path)


# blender 5 dropped collada, this one is a single plain mesh (one index per corner) so read it by hand
def load_dae(path):
    import xml.etree.ElementTree as ET

    ns = {"c": "http://www.collada.org/2005/11/COLLADASchema"}
    root = ET.parse(path).getroot()

    def floats(suffix, stride):
        arr = root.find(f".//c:float_array[@id='meshId0-{suffix}-array']", ns)
        vals = [float(x) for x in arr.text.split()]
        return [tuple(vals[i:i + stride]) for i in range(0, len(vals), stride)]

    positions = floats("positions", 3)
    uvs = floats("tex0", 2)
    poly = root.find(".//c:polylist", ns)
    counts = [int(x) for x in poly.find("c:vcount", ns).text.split()]
    idx = [int(x) for x in poly.find("c:p", ns).text.split()]
    faces, at = [], 0
    for n in counts:
        faces.append(idx[at:at + n])
        at += n

    mesh = bpy.data.meshes.new("box")
    mesh.from_pydata(positions, [], faces)
    uv = mesh.uv_layers.new()
    for poly_ in mesh.polygons:
        for li in poly_.loop_indices:
            uv.data[li].uv = uvs[mesh.loops[li].vertex_index]
    obj = bpy.data.objects.new("box", mesh)
    bpy.context.scene.collection.objects.link(obj)


def one_mesh():
    meshes = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    for o in bpy.context.scene.objects:
        o.select_set(o in meshes)
    bpy.context.view_layer.objects.active = meshes[0]
    if len(meshes) > 1:
        bpy.ops.object.join()
    obj = bpy.context.view_layer.objects.active
    # drop empties/armatures/cameras the file came with
    for o in list(bpy.context.scene.objects):
        if o is not obj:
            bpy.data.objects.remove(o, do_unlink=True)
    obj.parent = None
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    return obj


def fit(obj):
    def size():
        xs = [v.co.x for v in obj.data.vertices]
        ys = [v.co.y for v in obj.data.vertices]
        zs = [v.co.z for v in obj.data.vertices]
        return Vector((min(xs), min(ys), min(zs))), Vector((max(xs), max(ys), max(zs)))

    lo, hi = size()
    # long side along x
    if hi.y - lo.y > hi.x - lo.x:
        obj.rotation_euler = (0, 0, 1.5707963)
        bpy.ops.object.transform_apply(rotation=True)
        lo, hi = size()
    ext = hi - lo
    centre = Vector(((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, lo.z))
    for v in obj.data.vertices:
        p = v.co - centre
        # blender is z up; the game's y up comes out of the gltf exporter
        v.co = Vector((p.x * W / ext.x, p.y * D / ext.y, p.z * H / ext.z))


def colour_only(obj, texture_path):
    img = bpy.data.images.load(os.path.abspath(texture_path))
    if max(img.size) > TEXTURE_SIZE:
        img.scale(TEXTURE_SIZE, TEXTURE_SIZE)
    mat = bpy.data.materials.new("cardboard")
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    bsdf = next(n for n in nodes if n.type == "BSDF_PRINCIPLED")
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = img
    mat.node_tree.links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    obj.data.materials.clear()
    obj.data.materials.append(mat)


def main():
    os.makedirs(OUT, exist_ok=True)
    for name, model, texture in BOXES:
        empty_scene()
        load(model)
        obj = one_mesh()
        fit(obj)
        colour_only(obj, texture)
        tris = sum(len(p.vertices) - 2 for p in obj.data.polygons)
        out = os.path.join(OUT, f"cardboard_box_{name}.glb")
        bpy.ops.export_scene.gltf(
            filepath=out,
            export_format="GLB",
            use_selection=False,
            export_yup=True,
            export_apply=True,
            export_image_format="JPEG",
            export_materials="EXPORT",
        )
        print(f"BOX {name}: {tris} triangles ({tris * 3} indices) -> {out}")


main()
