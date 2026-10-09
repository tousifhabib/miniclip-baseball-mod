//! Where the parts of the view are on the stage: the pitcher, the batter,
//! the ball, the field and what stands on it.

use bb_engine::display::{Path, child_bounds};
use bb_engine::library::Library;
use bb_engine::math::Matrix;
use bb_engine::stage::Stage;

use super::at;
use crate::play::field::{Ground, reach};
use crate::play::pitch::Point;
use crate::rules::FieldRules;

/// Where the parts of the view are, found afresh for each pitch.
#[derive(Clone, Debug)]
pub(crate) struct Parts {
    pub main: Path,
    pub pitcher: Path,
    pub hitter: Path,
    pub aim: Path,
    pub aim_shadow: Path,
    pub marker: Path,
    pub ball: Path,
    pub shadow: Path,
    pub fly: Path,
    pub fly_ball: Path,
    pub fly_shadow: Option<Path>,
    pub aim_area: Path,
    pub field: Path,
    pub scoreboard: Option<Path>,
    pub strike_anim: Option<Path>,
    pub transitions: Path,
    pub next: Path,
    pub flare: Option<Path>,
    pub field_ball: Path,
    pub field_ball_inner: Path,
    pub holder: Option<Path>,
    pub fielders: Vec<Path>,
    pub umpires: Vec<Path>,
    pub field_scoreboard: Option<Path>,
    /// Fixed points of the batting view.
    pub centre_x: f32,
    pub fly_mark: Point,
    pub ground_y: f32,
    /// The box the aiming ring is kept inside: left, top, right, bottom.
    pub aim_box: [f32; 4],
    /// Fixed points of the field.
    pub home: Point,
    pub field_mark: Point,
    pub foul: (f32, f32),
    pub bases: [Point; 4],
}

impl Parts {
    /// How far across the batting view a place this far across the field
    /// is: where the art's pointer stands for a ball that comes down there.
    pub(crate) fn across_view(&self, across: f32, rules: &FieldRules) -> f32 {
        let mark = self.foul.0 + across * (self.foul.1 - self.foul.0);
        self.centre_x + (mark - self.field_mark.0) * rules.aim_share
    }

    /// The fixed points of the field that a hit is placed by.
    pub(crate) fn ground(&self, rules: &FieldRules) -> Ground {
        Ground {
            home: self.home,
            mark_y: self.field_mark.1,
            foul: self.foul,
            wall: rules.wall,
            infield: reach(self.home, self.bases[1]),
            ..Ground::default()
        }
    }

    /// Finds the parts of a batting view that has just been built.
    pub(crate) fn find(stage: &Stage, library: &Library) -> Option<Parts> {
        let main = stage.find_named(&[], "gameMain")?;
        let part = |names: &[&str]| stage.find(&main, names);
        let field = part(&["field"])?;
        let in_field = |name: &str| stage.find(&field, &[name]);
        let point = |path: Option<Path>| path.map(|path| at(stage, &path));
        let aim_box = stage
            .child(&part(&["acl"])?)
            .and_then(|child| child_bounds(child, Matrix::IDENTITY, library))?;
        let fly = part(&["ballFly"])?;
        let field_ball = in_field("ballFly")?;
        Some(Parts {
            pitcher: part(&["pitcher"])?,
            hitter: part(&["hitter"])?,
            aim: part(&["aimCircle"])?,
            aim_shadow: part(&["aimCircleShadow"])?,
            marker: part(&["ballPassesBat_marker"])?,
            ball: part(&["ballAll"])?,
            shadow: part(&["ballShadow"])?,
            fly_ball: stage.find(&fly, &["ball"])?,
            fly_shadow: stage.find(&fly, &["ballShadow"]),
            fly,
            aim_area: part(&["aimArea"])?,
            scoreboard: part(&["scoreboard"]),
            strike_anim: part(&["strikeAnim_old"]).or_else(|| part(&["strikeAnim"])),
            transitions: part(&["transitions"])?,
            next: part(&["btn_nextBall"])?,
            flare: part(&["lightFlare"]),
            field_ball_inner: stage.find(&field_ball, &["ball"])?,
            field_ball,
            holder: in_field("runnerHolder"),
            fielders: (1..=9)
                .filter_map(|number| in_field(&format!("fielder{number}")))
                .collect(),
            umpires: (1..=3)
                .filter_map(|number| in_field(&format!("umpire{number}")))
                .collect(),
            field_scoreboard: in_field("scoreboard"),
            centre_x: point(part(&["centreMarker"]))?.0,
            fly_mark: point(part(&["shadowFlyMarker"]))?,
            ground_y: point(part(&["uMarker"]))?.1,
            aim_box,
            home: point(in_field("startPointMarker"))?,
            field_mark: point(in_field("shadowFlyMarker"))?,
            // The arcade game's field has no foul lines and no bases.
            foul: (
                point(in_field("foulMarkerLeft")).map_or(f32::MIN, |at| at.0),
                point(in_field("foulMarkerRight")).map_or(f32::MAX, |at| at.0),
            ),
            bases: [1, 2, 3, 4]
                .map(|base| point(in_field(&format!("base{base}"))).unwrap_or_default()),
            field,
            main,
        })
    }
}
