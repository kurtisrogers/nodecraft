#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}

struct VoxelLighting {
    sun_dir: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> material: VoxelLighting;

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
#ifdef VERTEX_COLORS
    let base = in.color;
#else
    let base = vec4<f32>(0.45, 0.72, 0.35, 1.0);
#endif
    let day = material.sun_dir.w;
    let night = 1.0 - day;
    let n = normalize(in.world_normal);
    let sun = normalize(material.sun_dir.xyz);
    let shade = clamp(dot(n, sun) * 0.5 + 0.5, 0.0, 1.0);
    let light = mix(0.28 + night * 0.18, 0.34 + shade * 0.78, day);
    var rgb = base.rgb * light;
    if base.r > 0.78 && base.g < 0.42 && base.b < 0.25 {
        let glow = 0.55 + 0.45 * (1.0 - day * 0.35);
        rgb += vec3<f32>(0.65, 0.22, 0.03) * glow;
    }
    out.color = vec4<f32>(rgb, base.a);
    return out;
}
