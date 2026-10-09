//! The mods: changes to the game that the player switches on and off from
//! the menu, and the page of the menu they are listed on.

mod chosen;
mod page;

use crate::rules::Rules;
pub use chosen::Mods;
pub use page::{Asked, ModsPage};

/// One change to the game. To add a mod, add it here and to [`Mod::ALL`],
/// and give it a file in `play/mods/`: the menu lists whatever is in `ALL`,
/// by what its file says of it. The README's "A mod" has the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mod {
    /// A bar in the batting view that shows when to swing.
    TimingIndicator,
    /// Only the pitcher goes after a ball that has been hit. The rest of
    /// the side stands where it is, and nobody throws the ball on.
    LonePitcher,
    /// Every ball the bat meets is a home run, and the better the swing was
    /// timed the further it goes.
    ZingerHit,
    /// Fielders let the ball go: they drop catches, fumble pick-ups, and
    /// fail to hold throws at the bases. How often is set on the menu.
    Butterfingers,
    /// Pitches sway from side to side on the way in, and the marker shows
    /// only roughly where they will cross.
    Knuckleball,
    /// Every run scored makes the next pitch faster, and every strike
    /// slows the pitches down again.
    HeatCheck,
    /// Each pitch is a fastball, a slow change-up or a big curve, by
    /// chance, and nothing shows which until it leaves his hand.
    MysteryPitch,
    /// Before a pitch the player clicks a spot on the outfield, and a hit
    /// that comes down near it is worth runs on top.
    CalledShot,
    /// Each hit in a row widens the timing window for the next swing,
    /// and a strike takes it back to what it was.
    HotBat,
    /// One strike puts a batter out, and every run counts for two.
    SuddenDeath,
    /// Every fifth pitch is a golden ball: runs scored off it count for
    /// three, and a strike on it puts the batter out.
    GoldenBall,
    /// The ball keeps most of its speed when it bounces, comes off the
    /// wall like a ball off a cushion, and is kept in by the foul lines
    /// too. How much it keeps is set on the menu.
    PinballPark,
    /// A hit floats: it goes where it would have gone, taking several
    /// times as long to get there. How many times is set on the menu.
    MoonBall,
    /// Runners go round the bases several times as fast, and can be
    /// sent on at any time while the ball is in play, in the air or not.
    /// How fast is set on the menu.
    TurboRunners,
    /// The stadium is darkened, with the players and the ball left lit,
    /// and a home run flashes the lights.
    NightGame,
    /// The fielders stand where the last few balls were hit, so that
    /// hitting the same way every time stops paying.
    TheShift,
    /// The pitcher tires as his pitches mount up: they come slower and
    /// miss the strike zone more, until a fresh pitcher comes in for him.
    TiredArm,
    /// A click on the little field while the pitcher winds up sends a
    /// runner for the next base, and the catcher throws to put him out.
    StolenBases,
    /// The outfield wall has signs on it, one of them lit, and a ball that
    /// strikes one is worth runs on top.
    HitTheSign,
    /// Each batter in a row who reaches base makes the runs that follow
    /// worth one more, until somebody is out.
    Rally,
    /// With two out and a runner on second or third, every run counts
    /// for two.
    Clutch,
    /// Holding the space bar slows the pitch as it comes to the plate, for
    /// as long as a meter lasts that hits fill up again.
    BulletTime,
    /// The batter bats left-handed, from the other side of the plate, and
    /// the pitches are thrown to him as they were to a right-hander.
    Southpaw,
}

/// What the menu and the files know a mod by. Each mod says its own, in
/// its file in `play/mods/`.
pub(crate) struct About {
    /// The name it is saved under, and asked for by on the command line.
    pub(crate) key: &'static str,
    /// What the menu calls it.
    pub(crate) name: &'static str,
    /// What the menu says it does.
    pub(crate) does: &'static str,
    /// Its setting, if it has one besides being on or off.
    pub(crate) setting: Option<Setting>,
}

/// A mod's setting: a level, counted from 1.
pub(crate) struct Setting {
    /// What the menu calls it.
    pub(crate) name: &'static str,
    /// The level it is at until it is set to another.
    pub(crate) usual: u8,
    /// How many levels the rules give it.
    pub(crate) levels: fn(&Rules) -> u8,
    /// What the menu says a level comes to.
    pub(crate) words: fn(u8, &Rules) -> String,
}

impl Mod {
    /// Every mod, in the order the menu lists them.
    pub const ALL: [Mod; 23] = [
        Mod::TimingIndicator,
        Mod::LonePitcher,
        Mod::ZingerHit,
        Mod::Butterfingers,
        Mod::Knuckleball,
        Mod::HeatCheck,
        Mod::MysteryPitch,
        Mod::CalledShot,
        Mod::HotBat,
        Mod::SuddenDeath,
        Mod::GoldenBall,
        Mod::PinballPark,
        Mod::MoonBall,
        Mod::TurboRunners,
        Mod::NightGame,
        Mod::TheShift,
        Mod::TiredArm,
        Mod::StolenBases,
        Mod::HitTheSign,
        Mod::Rally,
        Mod::Clutch,
        Mod::BulletTime,
        Mod::Southpaw,
    ];

    fn about_it(self) -> &'static About {
        crate::play::mods::about(self)
    }

    /// The name the mod is saved under, and asked for by on the command
    /// line.
    pub fn key(self) -> &'static str {
        self.about_it().key
    }

    /// The mod with this key.
    pub fn from_key(key: &str) -> Option<Mod> {
        Mod::ALL.into_iter().find(|each| each.key() == key)
    }

    /// What the menu calls it.
    pub fn name(self) -> &'static str {
        self.about_it().name
    }

    /// What the menu says it does.
    pub fn about(self) -> &'static str {
        self.about_it().does
    }

    fn its_setting(self) -> Option<&'static Setting> {
        self.about_it().setting.as_ref()
    }

    /// What the menu calls the mod's setting, if it has one besides being
    /// on or off.
    pub fn setting(self) -> Option<&'static str> {
        self.its_setting().map(|setting| setting.name)
    }

    /// The level the mod's setting is at until it is set to another.
    pub fn usual_level(self) -> u8 {
        self.its_setting().map_or(1, |setting| setting.usual)
    }

    /// How many levels the mod's setting has. None, for a mod with no
    /// setting.
    pub fn levels(self, rules: &Rules) -> u8 {
        self.its_setting()
            .map_or(0, |setting| (setting.levels)(rules))
    }

    /// What the menu says a level of the mod's setting comes to.
    pub fn level_words(self, level: u8, rules: &Rules) -> String {
        self.its_setting()
            .map_or_else(String::new, |setting| (setting.words)(level, rules))
    }
}

#[cfg(test)]
mod tests;
