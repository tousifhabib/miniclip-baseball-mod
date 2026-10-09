//! The colours along a gradient: 256 of them to a ramp, worked out from
//! the gradient's stops.

use bb_format as f;

use super::{Ramp, Tessellator, lerp};

impl Tessellator {
    /// The row of the ramp for these stops, adding it if it is new.
    pub(super) fn ramp(&mut self, stops: &[f::GradientStop]) -> usize {
        let mut ramp: Ramp = [[0; 4]; 256];
        for (i, entry) in ramp.iter_mut().enumerate() {
            let position = i as f64 / 255.0;
            let after = stops
                .iter()
                .position(|stop| stop.offset >= position)
                .unwrap_or(stops.len());
            let color = match (
                after.checked_sub(1).and_then(|i| stops.get(i)),
                stops.get(after),
            ) {
                (Some(a), Some(b)) => {
                    let span = (b.offset - a.offset).max(1e-9);
                    blend_color(a.color, b.color, ((position - a.offset) / span) as f32)
                }
                (Some(only), None) | (None, Some(only)) => only.color,
                (None, None) => f::Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                },
            };
            *entry = [color.r, color.g, color.b, color.a];
        }
        if let Some(&row) = self.ramp_rows.get(&ramp) {
            return row;
        }
        self.ramps.push(ramp);
        self.ramp_rows.insert(ramp, self.ramps.len() - 1);
        self.ramps.len() - 1
    }
}

/// The colour part of the way from `a` to `b`, channel by channel.
pub(super) fn blend_color(a: f::Color, b: f::Color, t: f32) -> f::Color {
    let channel = |a: u8, b: u8| lerp(f32::from(a), f32::from(b), t).round() as u8;
    f::Color {
        r: channel(a.r, b.r),
        g: channel(a.g, b.g),
        b: channel(a.b, b.b),
        a: channel(a.a, b.a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ramp_blends_between_its_stops_and_is_reused() {
        let stop = |offset: f64, v: u8| f::GradientStop {
            offset,
            color: f::Color {
                r: v,
                g: v,
                b: v,
                a: 255,
            },
        };
        let mut tessellator = Tessellator::default();
        let row = tessellator.ramp(&[stop(0.0, 0), stop(1.0, 255)]);
        assert_eq!(tessellator.ramps[row][0], [0, 0, 0, 255]);
        assert_eq!(tessellator.ramps[row][128], [128, 128, 128, 255]);
        assert_eq!(tessellator.ramps[row][255], [255, 255, 255, 255]);
        assert_eq!(tessellator.ramp(&[stop(0.0, 0), stop(1.0, 255)]), row);
        assert_eq!(tessellator.ramps.len(), 1);
    }
}
