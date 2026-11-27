//! Optional native Rust GPU compute backend (`--features gpu-wgpu`, docs/swarm/S5.md).
//!
//! Runs batched dense projections (`Y = X @ W^T + b` with optional ReLU, tanh-approx GELU, or accumulation)
//! directly on Metal (Apple Silicon), Vulkan (Linux/Windows/Android), or DX12 (Windows) via WGSL compute shaders,
//! requiring zero Python or CUDA runtime dependencies.

use std::sync::{Arc, OnceLock};
use wgpu::util::DeviceExt;

const MATMUL_WGSL: &str = r#"
struct Params {
    rows: u32,
    inp: u32,
    out_dim: u32,
    flags: u32, // bit 0: do_act, bit 1: gelu, bit 2: add_to_y
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> x_buf: array<f32>;
@group(0) @binding(2) var<storage, read> w_buf: array<f32>;
@group(0) @binding(3) var<storage, read> b_buf: array<f32>;
@group(0) @binding(4) var<storage, read_write> y_buf: array<f32>;

fn act(v: f32, flags: u32) -> f32 {
    if ((flags & 1u) == 0u) {
        return v;
    }
    if ((flags & 2u) != 0u) {
        let k: f32 = 0.7978846;
        return 0.5 * v * (1.0 + tanh(k * (v + 0.044715 * v * v * v)));
    }
    return max(v, 0.0);
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let o = gid.x;
    let r = gid.y;
    if (r >= params.rows || o >= params.out_dim) {
        return;
    }
    var acc: f32 = b_buf[o];
    let x_base = r * params.inp;
    let w_base = o * params.inp;
    for (var k: u32 = 0u; k < params.inp; k = k + 1u) {
        acc = acc + x_buf[x_base + k] * w_buf[w_base + k];
    }
    let idx = r * params.out_dim + o;
    let val = act(acc, params.flags);
    if ((params.flags & 4u) != 0u) {
        y_buf[idx] = y_buf[idx] + val;
    } else {
        y_buf[idx] = val;
    }
}
"#;

pub struct WgpuContext {
    pub adapter_name: String,
    pub backend: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

static GLOBAL_CTX: OnceLock<Result<Arc<WgpuContext>, String>> = OnceLock::new();

impl WgpuContext {
    /// Initializes the process-wide high-performance `wgpu` GPU adapter and compute pipeline.
    pub fn global() -> Result<Arc<WgpuContext>, String> {
        GLOBAL_CTX.get_or_init(|| Self::init().map(Arc::new)).clone()
    }

    /// Detects the available native GPU label (e.g. `wgpu:Apple M4 (Metal)`), if any.
    pub fn detect_label() -> Option<String> {
        Self::global().ok().map(|c| format!("wgpu:{} ({})", c.adapter_name, c.backend))
    }

    pub fn init() -> Result<WgpuContext, String> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| "no wgpu hardware GPU adapter found".to_string())?;

        let info = adapter.get_info();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("playmat-ai-wgpu"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|e| format!("wgpu request_device: {e}"))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("playmat-ai-matmul"),
            source: wgpu::ShaderSource::Wgsl(MATMUL_WGSL.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("matmul-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("matmul-pl"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("matmul-pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Ok(WgpuContext {
            adapter_name: info.name,
            backend: format!("{:?}", info.backend),
            device,
            queue,
            pipeline,
            bind_group_layout,
        })
    }

    /// Executes batched `Y = act(X @ W^T + b)` (`X: [rows, inp]`, `W: [out, inp]`, `b: [out]`) on GPU.
    #[allow(clippy::too_many_arguments)]
    pub fn matmul_rows(
        &self,
        x: &[f32],
        w: &[f32],
        b: &[f32],
        rows: usize,
        inp: usize,
        out: usize,
        y: &mut [f32],
        do_act: bool,
        gelu: bool,
        add_to_y: bool,
    ) -> Result<(), String> {
        if rows == 0 || out == 0 {
            return Ok(());
        }
        let flags: u32 = (do_act as u32) | ((gelu as u32) << 1) | ((add_to_y as u32) << 2);
        let params: [u32; 4] = [rows as u32, inp as u32, out as u32, flags];

        let params_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params"),
            contents: bytemuck::cast_slice(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let x_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("x"),
            contents: bytemuck::cast_slice(x),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let w_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("w"),
            contents: bytemuck::cast_slice(w),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let b_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("b"),
            contents: bytemuck::cast_slice(b),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let y_bytes = std::mem::size_of_val(y) as u64;
        let y_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("y"),
            contents: bytemuck::cast_slice(y),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("y-readback"),
            size: y_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("matmul-bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: x_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: w_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: b_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 4, resource: y_buf.as_entire_binding() },
            ],
        });

        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("matmul-enc") });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("matmul-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups((out as u32).div_ceil(16), (rows as u32).div_ceil(16), 1);
        }
        enc.copy_buffer_to_buffer(&y_buf, 0, &staging, 0, y_bytes);
        self.queue.submit(Some(enc.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv().map_err(|e| format!("wgpu readback recv: {e}"))?.map_err(|e| format!("wgpu map_async: {e:?}"))?;

        let data = slice.get_mapped_range();
        let out_floats: &[f32] = bytemuck::cast_slice(&data);
        y.copy_from_slice(out_floats);
        drop(data);
        staging.unmap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::Linear;

    #[test]
    fn wgpu_matmul_matches_cpu_within_float_tolerance() {
        let Ok(ctx) = WgpuContext::global() else {
            // Headless CI runners without a GPU adapter skip cleanly.
            return;
        };
        let (rows, inp, out) = (16usize, 64usize, 48usize);
        let x: Vec<f32> = (0..rows * inp).map(|i| ((i % 29) as f32 - 14.0) * 0.05).collect();
        let w: Vec<f32> = (0..out * inp).map(|i| ((i % 31) as f32 - 15.0) * 0.03).collect();
        let b: Vec<f32> = (0..out).map(|i| (i as f32 - 20.0) * 0.01).collect();
        let lin = Linear::new(w.clone(), b.clone(), out, inp);

        for (do_act, gelu) in [(false, false), (true, false), (true, true)] {
            let mut y_cpu = vec![0.0f32; rows * out];
            for r in 0..rows {
                let mut row_out = Vec::new();
                // Compute CPU reference directly
                for o in 0..out {
                    let mut s = lin.b[o];
                    for k in 0..inp {
                        s += x[r * inp + k] * lin.w[o * inp + k];
                    }
                    let v = if !do_act {
                        s
                    } else if gelu {
                        const K: f32 = 0.797_884_6;
                        0.5 * s * (1.0 + (K * (s + 0.044_715 * s * s * s)).tanh())
                    } else {
                        s.max(0.0)
                    };
                    row_out.push(v);
                }
                y_cpu[r * out..(r + 1) * out].copy_from_slice(&row_out);
            }
            let mut y_gpu = vec![0.0f32; rows * out];
            ctx.matmul_rows(&x, &w, &b, rows, inp, out, &mut y_gpu, do_act, gelu, false).unwrap();
            let max_diff = y_cpu.iter().zip(&y_gpu).map(|(a, b)| (a - b).abs()).fold(0.0f32, f32::max);
            assert!(
                max_diff < 1e-4,
                "wgpu ({} / {}) parity mismatch (do_act={do_act}, gelu={gelu}): max_diff={max_diff}",
                ctx.adapter_name,
                ctx.backend
            );
        }
    }
}
