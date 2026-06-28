#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
#ifdef VERTEX_COLORS
    out.color = in.color;
#else
    out.color = vec4<f32>(0.45, 0.72, 0.35, 1.0);
#endif
    return out;
}
