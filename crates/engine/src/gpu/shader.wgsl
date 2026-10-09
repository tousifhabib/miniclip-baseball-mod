// Draws the engine's meshes: where each corner goes on the screen, and what
// colour each pixel is, from the corners' own colour, a gradient, an image
// or a finished layer. And the blur a layer is put through.

struct Globals {
    // Pixels to clip space: clip = pixel * view.xy + view.zw.
    view: vec4<f32>,
    // x: the least half-width a stroke may have, in pixels.
    limits: vec4<f32>,
};

struct Item {
    world_abcd: vec4<f32>,
    world_t: vec4<f32>,
    color_mult: vec4<f32>,
    color_add: vec4<f32>,
    // For a blur: xy is one pixel along the blur, in texture coordinates.
    paint_abcd: vec4<f32>,
    // xy: translation. z: the paint's row in the ramp texture.
    // For a blur: x is the width of the box, in pixels.
    paint_t: vec4<f32>,
    // x: 0 solid, 1 linear, 2 radial, 3 image, 4 layer.
    // y: 0 pad, 1 reflect, 2 repeat.
    kind: vec4<u32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var<uniform> item: Item;
@group(2) @binding(0) var paint_texture: texture_2d<f32>;
@group(2) @binding(1) var paint_sampler: sampler;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) paint: vec2<f32>,
};

@vertex
fn vs(
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) half_width: f32,
    @location(3) color: vec4<f32>,
) -> VertexOut {
    let m = item.world_abcd;
    let centre = vec2<f32>(
        m.x * position.x + m.z * position.y + item.world_t.x,
        m.y * position.x + m.w * position.y + item.world_t.y,
    );

    // A stroke is drawn as if by a round pen on the screen: its width is the
    // same in every direction, however its shape is stretched or skewed, and
    // never less than one pixel. So the vertex moves out from the centre
    // line on the screen, not in the shape's own coordinates. A direction
    // that is square to a line stays square to it when turned by the
    // inverse transpose of the transform, which is what this is.
    var pixel = centre;
    let turned = vec2<f32>(
        m.w * normal.x - m.y * normal.y,
        m.x * normal.y - m.z * normal.x,
    );
    if dot(turned, turned) > 0.0 {
        let scale = sqrt(abs(m.x * m.w - m.y * m.z));
        let half = max(half_width * scale, globals.limits.x);
        pixel = centre + normalize(turned) * length(normal) * half;
    }
    // Where the vertex is in the shape, for working out its paint.
    let local = position + normal * half_width;

    let p = item.paint_abcd;
    var out: VertexOut;
    out.clip = vec4<f32>(pixel * globals.view.xy + globals.view.zw, 0.0, 1.0);
    out.color = color;
    out.paint = vec2<f32>(
        p.x * local.x + p.z * local.y + item.paint_t.x,
        p.y * local.x + p.w * local.y + item.paint_t.y,
    );
    return out;
}

@fragment
fn fs(in: VertexOut) -> @location(0) vec4<f32> {
    var c = in.color;
    let kind = item.kind.x;
    if kind == 4u {
        // A finished layer: already transformed and multiplied by alpha.
        return textureSampleLevel(paint_texture, paint_sampler, in.paint, 0.0);
    }
    if kind == 3u {
        // Images are stored multiplied by alpha, so that smoothing does not
        // drag colour in from clear pixels.
        let texel = textureSampleLevel(paint_texture, paint_sampler, in.paint, 0.0);
        let rgb = select(vec3<f32>(0.0), texel.rgb / texel.a, texel.a > 0.0);
        c = vec4<f32>(rgb, texel.a * in.color.a);
    } else if kind != 0u {
        var t = in.paint.x;
        if kind == 2u {
            t = length(in.paint);
        }
        if item.kind.y == 1u {
            t = 1.0 - abs(t - 2.0 * floor(t / 2.0) - 1.0);
        } else if item.kind.y == 2u {
            t = fract(t);
        }
        t = clamp(t, 0.0, 1.0);
        let rows = f32(textureDimensions(paint_texture).y);
        let uv = vec2<f32>(t * (255.0 / 256.0) + 0.5 / 256.0, (item.paint_t.z + 0.5) / rows);
        c = textureSampleLevel(paint_texture, paint_sampler, uv, 0.0);
    }
    c = clamp(c * item.color_mult + item.color_add, vec4<f32>(0.0), vec4<f32>(1.0));
    return vec4<f32>(c.rgb * c.a, c.a);
}

struct BlurOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// One triangle that covers the whole target.
@vertex
fn vs_blur(@builtin(vertex_index) index: u32) -> BlurOut {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var out: BlurOut;
    out.clip = vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(corner.x, 1.0 - corner.y);
    return out;
}

// Flash's blur: the average of a row of pixels `width` wide. A width that is
// not a whole odd number gives the two end pixels part weight.
@fragment
fn fs_blur(in: BlurOut) -> @location(0) vec4<f32> {
    let radius = (item.paint_t.x - 1.0) / 2.0;
    let reach = i32(ceil(radius));
    var sum = vec4<f32>(0.0);
    var total = 0.0;
    for (var i = -reach; i <= reach; i++) {
        let weight = clamp(radius + 0.5 - abs(f32(i)), 0.0, 1.0);
        let uv = in.uv + item.paint_abcd.xy * f32(i);
        sum += textureSampleLevel(paint_texture, paint_sampler, uv, 0.0) * weight;
        total += weight;
    }
    return sum / max(total, 1e-6);
}
