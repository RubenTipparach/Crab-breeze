"""PBR material wiring shared by the Blender generators (mech_kit.py, build_cockpit.py).

Every custom texture ships as full PBR maps (owner, 2026-10-24: "I want normal maps, speculat maps and
ao for all our custom textures. Aim for that pbr triple a goodness that bevy provides"): base colour,
a tangent-space normal map, and an occlusion-roughness-metal map packed the glTF way (R occlusion,
G roughness, B metal), which Bevy's StandardMaterial reads as `occlusion_texture` and
`metallic_roughness_texture` from the one image.

This module is the one place a Principled BSDF is wired to those maps so the glTF exporter writes them:
roughness and metal through a Separate Color node on the ORM image, occlusion through the exporter's
"glTF Material Output" node group (Blender has no occlusion socket of its own).
"""
import bpy


def _gltf_output_group():
    g = bpy.data.node_groups.get("glTF Material Output")
    if g is None:
        g = bpy.data.node_groups.new("glTF Material Output", "ShaderNodeTree")
        s = g.interface.new_socket("Occlusion", in_out="INPUT", socket_type="NodeSocketFloat")
        s.default_value = 1.0
        s.min_value, s.max_value = 0.0, 1.0
    return g


def image(nt, img, uv_map=None, colour=False):
    """An image texture node on `img` (a bpy image), Non-Color unless `colour`, on the named UV map."""
    N, L = nt.nodes, nt.links
    t = N.new("ShaderNodeTexImage")
    t.image = img
    t.image.colorspace_settings.name = "sRGB" if colour else "Non-Color"
    if uv_map:
        uv = N.new("ShaderNodeUVMap")
        uv.uv_map = uv_map
        L.new(uv.outputs[0], t.inputs[0])
    return t


def orm(nt, bsdf, img, uv_map=None, occlusion=True):
    """Wire an ORM image: G to roughness, B to metal, R to occlusion (exported as occlusionTexture)."""
    N, L = nt.nodes, nt.links
    t = image(nt, img, uv_map)
    sep = N.new("ShaderNodeSeparateColor")
    L.new(t.outputs[0], sep.inputs[0])
    L.new(sep.outputs[1], bsdf.inputs["Roughness"])
    L.new(sep.outputs[2], bsdf.inputs["Metallic"])
    if occlusion:
        go = N.new("ShaderNodeGroup")
        go.node_tree = _gltf_output_group()
        L.new(sep.outputs[0], go.inputs["Occlusion"])
    return t


def normal(nt, bsdf, img, uv_map=None, strength=1.0):
    """Wire a tangent-space normal map image to the BSDF's normal."""
    N, L = nt.nodes, nt.links
    t = image(nt, img, uv_map)
    nm = N.new("ShaderNodeNormalMap")
    if uv_map:
        nm.uv_map = uv_map
    nm.inputs["Strength"].default_value = strength
    L.new(t.outputs[0], nm.inputs["Color"])
    L.new(nm.outputs[0], bsdf.inputs["Normal"])
    return t
