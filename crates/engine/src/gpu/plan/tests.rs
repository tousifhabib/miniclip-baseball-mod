use super::prepare::{item_for, layer_area};
use super::*;
use crate::tess::{Paint, Spread};

/// How draws treat the masks just now: the mode, by its number, and the
/// stencil value a draw is to match.
fn masks(plan: &Plan) -> (u32, u32) {
    (plan.masking.mode as u32, plan.masking.stencil)
}

#[test]
fn masks_count_up_as_one_is_set_inside_another_and_down_as_each_ends() {
    let (content, write, clear) = (0, 1, 2);
    let mut plan = Plan::new((590, 400));
    assert_eq!(masks(&plan), (content, 0));
    // The outline is drawn where there is no mask yet, and what it
    // covers is drawn where there is now one.
    plan.masking.push();
    assert_eq!(masks(&plan), (write, 0));
    plan.masking.activate();
    assert_eq!(masks(&plan), (content, 1));
    // A second inside the first.
    plan.masking.push();
    assert_eq!(masks(&plan), (write, 1));
    plan.masking.activate();
    assert_eq!(masks(&plan), (content, 2));
    // Each is taken away from where it was put, innermost first.
    plan.masking.deactivate();
    assert_eq!(masks(&plan), (clear, 2));
    plan.masking.pop();
    assert_eq!(masks(&plan), (content, 1));
    plan.masking.deactivate();
    assert_eq!(masks(&plan), (clear, 1));
    plan.masking.pop();
    assert_eq!(masks(&plan), (content, 0));
    // One end too many changes nothing.
    plan.masking.pop();
    assert_eq!(masks(&plan), (content, 0));
}

#[test]
fn a_blurred_object_gets_a_layer_of_its_own_where_masks_start_afresh() {
    let mut plan = Plan::new((590, 400));
    plan.masking.push();
    plan.masking.activate();
    // Twice over, with a box 4 wide across and 1 down.
    plan.begin_blur(4.0, 1.0, 2, [100.0, 100.0, 150.0, 140.0]);
    assert_eq!((plan.layers.len(), plan.current), (2, 1));
    // The picture spreads by 4 in all, and there is one more to spare.
    let layer = &plan.layers[1];
    assert_eq!((layer.origin, layer.size), ((95, 95), (64, 64)));
    // A box one pixel wide does nothing, so each time over is a blur
    // across and none down: a step of one pixel of the 64.
    assert_eq!(layer.blurs, [0, 1]);
    let told: Item = bytemuck::pod_read_unaligned(&plan.items[..size_of::<Item>()]);
    assert_eq!(told.paint_abcd, [1.0 / 64.0, 0.0, 0.0, 0.0]);
    assert_eq!(told.paint_t, [4.0, 0.0, 0.0, 0.0]);
    assert_eq!(masks(&plan), (0, 0));

    // When it ends, draws go where they went before, under the mask
    // that was in force there.
    assert_eq!(plan.end_blur(), Some(1));
    assert_eq!(plan.current, 0);
    assert_eq!(masks(&plan), (0, 1));
    plan.draw_layer(1);
    let step = &plan.layers[0].steps[0];
    assert!(step.texture == Texture::Layer(1));
    assert_eq!((step.item, step.stencil), (2, 1));
    // An end with nothing begun is passed over.
    assert_eq!(plan.end_blur(), None);
}

#[test]
fn nothing_is_drawn_of_a_blurred_object_that_is_out_of_view() {
    let mut plan = Plan::new((590, 400));
    assert!(!plan.out_of_view());
    plan.begin_blur(4.0, 4.0, 1, [700.0, 10.0, 800.0, 40.0]);
    assert!(plan.out_of_view());
    assert!(plan.layers[1].blurs.is_empty());
    assert!(plan.items.is_empty());
    // There is nothing of it to draw into the frame, either.
    assert_eq!(plan.end_blur(), None);
    assert!(!plan.out_of_view());
}

#[test]
fn a_layer_covers_its_object_and_the_reach_of_the_blur() {
    let area = layer_area([100.0, 100.0, 150.0, 140.0], 4.0, (0, 0), (590, 400));
    // 96 to 154 across and 96 to 144 down, each rounded up to 64.
    assert_eq!(area, Some(((96, 96), (64, 64))));
}

#[test]
fn a_layer_is_cut_down_to_what_can_reach_the_view() {
    let area = layer_area([-500.0, 10.0, 30.0, 40.0], 4.0, (0, 0), (590, 400));
    assert_eq!(area, Some(((-4, 6), (64, 64))));
}

#[test]
fn an_object_wholly_out_of_view_gets_no_layer() {
    assert_eq!(
        layer_area([700.0, 10.0, 800.0, 40.0], 4.0, (0, 0), (590, 400)),
        None
    );
}

/// Where something is drawn, how it is tinted and where its paint sits
/// in it, with every number different so that none can be taken for
/// another.
const WORLD: Matrix = Matrix {
    a: 1.0,
    b: 2.0,
    c: 3.0,
    d: 4.0,
    tx: 5.0,
    ty: 6.0,
};
const TINT: ColorTransform = ColorTransform {
    mult: [0.1, 0.2, 0.3, 0.4],
    add: [0.5, 0.6, 0.7, 0.8],
};
const PLACED: Matrix = Matrix {
    a: 7.0,
    b: 8.0,
    c: 9.0,
    d: 10.0,
    tx: 11.0,
    ty: 12.0,
};

#[test]
fn every_paint_is_told_where_it_is_drawn_and_how_it_is_tinted() {
    let paints = [
        Paint::Solid,
        Paint::Linear {
            ramp: 7,
            matrix: PLACED,
            spread: Spread::Pad,
        },
        Paint::Radial {
            ramp: 3,
            matrix: PLACED,
            spread: Spread::Pad,
        },
        Paint::Image {
            image: 5,
            matrix: PLACED,
            smooth: true,
        },
    ];
    for paint in paints {
        let (item, _) = item_for(&paint, WORLD, TINT);
        assert_eq!(item.world_abcd, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(item.world_t, [5.0, 6.0, 0.0, 0.0]);
        assert_eq!(item.color_mult, TINT.mult);
        assert_eq!(item.color_add, TINT.add);
    }
}

#[test]
fn a_solid_paint_sits_nowhere_of_its_own_and_reads_no_row_of_the_ramps() {
    let (item, texture) = item_for(&Paint::Solid, WORLD, TINT);
    assert_eq!(item.paint_abcd, [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(item.paint_t, [0.0; 4]);
    assert_eq!(item.kind, [0; 4]);
    // The ramps are bound all the same: a draw must have some texture.
    assert!(texture == Texture::Ramps);
}

#[test]
fn a_gradient_is_told_its_row_of_the_ramps_and_how_it_goes_on_past_its_ends() {
    let ways = [(Spread::Pad, 0), (Spread::Reflect, 1), (Spread::Repeat, 2)];
    for (spread, code) in ways {
        let linear = Paint::Linear {
            ramp: 7,
            matrix: PLACED,
            spread,
        };
        let (item, texture) = item_for(&linear, WORLD, TINT);
        assert_eq!(item.paint_abcd, [7.0, 8.0, 9.0, 10.0]);
        assert_eq!(item.paint_t, [11.0, 12.0, 7.0, 0.0]);
        assert_eq!(item.kind, [1, code, 0, 0]);
        assert!(texture == Texture::Ramps);

        let radial = Paint::Radial {
            ramp: 3,
            matrix: PLACED,
            spread,
        };
        let (item, texture) = item_for(&radial, WORLD, TINT);
        assert_eq!(item.paint_abcd, [7.0, 8.0, 9.0, 10.0]);
        assert_eq!(item.paint_t, [11.0, 12.0, 3.0, 0.0]);
        assert_eq!(item.kind, [2, code, 0, 0]);
        assert!(texture == Texture::Ramps);
    }
}

#[test]
fn an_image_is_read_from_a_texture_of_its_own_smoothed_or_not() {
    for smooth in [false, true] {
        let image = Paint::Image {
            image: 5,
            matrix: PLACED,
            smooth,
        };
        let (item, texture) = item_for(&image, WORLD, TINT);
        assert_eq!(item.paint_abcd, [7.0, 8.0, 9.0, 10.0]);
        assert_eq!(item.paint_t, [11.0, 12.0, 0.0, 0.0]);
        assert_eq!(item.kind, [3, 0, 0, 0]);
        assert!(texture == Texture::Image { slot: 5, smooth });
    }
}
