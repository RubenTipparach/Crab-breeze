# Design: occlusion-culling

## 1. How it works (Bevy 0.20, `bevy_render::occlusion_culling`)

Two-phase occlusion culling: an early depth prepass draws what was visible last frame, Bevy builds
a hierarchical depth buffer from it, a compute pass tests every mesh's bounding box against it,
and only the meshes that may be visible are drawn in the late prepass and the main passes. It needs
GPU preprocessing (on: every benchmark run reports `gpu_preprocessing: true`) and a `DepthPrepass`
on the view (the eye camera has one for the water and SSAO). Building meshes have world-space
vertices and an identity transform, so their bounding boxes are their true world boxes.

## 2. Settings

`view.occlusion_culling: true` in `world.ron`; `CB_NO_OCCLUSION=1` switches it off.

## 3. Cost

GPU: the depth pyramid and one compute pass per view, small at 1920 x 1080. Saves: vertex and
fragment work of everything hidden. Measured: city combat, interleaved, on and off
(`CB_NO_OCCLUSION=1`), same binary.

## Open questions

| question | recommendation | status |
| --- | --- | --- |
| A frame of late pop-in when turning fast | accept if a take does not show it | provisional |
