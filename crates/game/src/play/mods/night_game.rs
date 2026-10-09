//! The night game: a mod that darkens the stadium and leaves the players,
//! the ball and the scoreboards lit.
//!
//! The stadium is one picture behind everything else in each view, so
//! darkening it is a matter of tinting that picture. The art builds the
//! views afresh for every pitch, so it is done again every frame. A home
//! run flashes the lights: for a moment the picture goes from dark to
//! brighter than day and back, several times over.

use bb_engine::display::Path;
use bb_engine::math::ColorTransform;
use bb_engine::stage::Stage;

use crate::art;
use crate::rules::NightRules;

/// The mod, in play: the lights, and a flash of them still to come.
pub(crate) struct NightGame {
    rules: NightRules,
    /// A home run has just been hit, which the lights have yet to flash
    /// for.
    home_run: bool,
    /// Frames of a flash still to come.
    flash: u32,
}

impl NightGame {
    pub fn new(rules: &NightRules) -> NightGame {
        NightGame {
            rules: rules.clone(),
            home_run: false,
            flash: 0,
        }
    }

    /// A home run has been hit: the lights flash for it, from the next
    /// frame.
    pub fn a_home_run_was_hit(&mut self) {
        self.home_run = true;
    }

    /// How the stadium is lit this frame. A flash that is under way moves
    /// on by a frame.
    pub fn lighting(&mut self) -> ColorTransform {
        if std::mem::take(&mut self.home_run) {
            self.flash = self.rules.flash_time;
        }
        let lighting = lighting(self.flash, &self.rules);
        self.flash = self.flash.saturating_sub(1);
        lighting
    }
}

/// The stadium as it is by day.
pub(crate) const DAY: ColorTransform = ColorTransform {
    mult: [1.0, 1.0, 1.0, 1.0],
    add: [0.0, 0.0, 0.0, 0.0],
};

/// How the stadium is lit, with this many frames of a flash still to come.
fn lighting(flash: u32, rules: &NightRules) -> ColorTransform {
    let [red, green, blue] = rules.dark;
    // It begins lit, and is lit and dark by turns from there.
    let lit = flash > 0 && ((flash - 1) / rules.flash_every.max(1)).is_multiple_of(2);
    if lit {
        ColorTransform {
            add: [rules.glare, rules.glare, rules.glare, 0.0],
            ..DAY
        }
    } else {
        ColorTransform {
            mult: [red, green, blue, 1.0],
            ..DAY
        }
    }
}

/// Lights the stadium in whichever views of the game are on the stage: the
/// batting view, and the view of the field inside it.
pub(crate) fn light(lighting: ColorTransform, stage: &mut Stage) {
    let Some(main) = stage.find_named(&[], "gameMain") else {
        return;
    };
    let mut views = vec![main.clone()];
    views.extend(stage.find(&main, &["field"]));
    let mut backdrops: Vec<Path> = Vec::new();
    for view in views {
        let Some(clip) = stage.clip(&view) else {
            continue;
        };
        for (&depth, child) in &clip.children {
            if art::BACKDROPS.contains(&child.symbol) && child.color != lighting {
                let mut path = view.clone();
                path.push(depth);
                backdrops.push(path);
            }
        }
    }
    for path in backdrops {
        if let Some(backdrop) = stage.child_mut(&path) {
            backdrop.set_color(lighting);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rules;

    #[test]
    fn the_stadium_is_dark_but_for_a_flash_that_goes_on_and_off() {
        let rules = Rules::default().night;
        let dark = lighting(0, &rules);
        assert!(dark.mult[..3].iter().all(|&share| share < 0.6));
        assert_eq!(dark.add, [0.0; 4]);
        // Through a flash it is lit and dark by turns, and ends dark.
        let through: Vec<bool> = (0..=rules.flash_time)
            .rev()
            .map(|left| lighting(left, &rules) != dark)
            .collect();
        let changes = through.windows(2).filter(|pair| pair[0] != pair[1]).count();
        assert!(changes >= 5, "{changes}");
        assert!(!through[through.len() - 1]);
        // Lit, it is brighter than by day.
        let lit = lighting(rules.flash_time, &rules);
        assert!(lit.add[0] > 0.0 && lit.mult[0] == 1.0);
    }
}
