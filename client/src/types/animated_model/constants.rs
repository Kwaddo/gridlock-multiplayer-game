pub(super) const SKIN_INTERVAL: f32 = 1.0 / 30.0; // skin at 30fps max

/// The most joints the skinning shader takes. Each costs 4 of the vertex
/// shader's uniform vectors; WebGL only guarantees 128, but desktop and most
/// mobile GPUs offer 1024 or more. A GPU that cannot fit the shader falls
/// back to skinning on the CPU.
pub(super) const MAX_GPU_JOINTS: usize = 80;
