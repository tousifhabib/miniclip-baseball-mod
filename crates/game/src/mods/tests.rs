use std::collections::BTreeSet;

use proptest::prelude::*;

use super::chosen::Mods;
use super::page::ModsPage;
use super::*;

#[test]
fn every_mod_is_known_by_its_own_key() {
    for which in Mod::ALL {
        assert_eq!(Mod::from_key(which.key()), Some(which));
    }
    let keys: BTreeSet<&str> = Mod::ALL.iter().map(|each| each.key()).collect();
    assert_eq!(keys.len(), Mod::ALL.len());
    assert_eq!(Mod::from_key("no_such_mod"), None);
}

#[test]
fn the_list_has_as_many_pages_as_it_takes_and_every_mod_is_on_one() {
    let rules = Rules::default();
    let pages = ModsPage::pages(&rules);
    let listed: Vec<Mod> = pages.iter().flatten().copied().collect();
    assert_eq!(listed, Mod::ALL);
    for page in &pages {
        assert!(!page.is_empty());
        let room: f32 = page
            .iter()
            .map(|&which| ModsPage::room_for(which, &rules))
            .sum();
        assert!(room <= ModsPage::ROOM, "{page:?}");
    }
    // The first four fit on a page between them.
    assert_eq!(pages[0].len(), 4);
}

#[test]
fn a_mod_is_off_until_it_is_switched_on() {
    let mut mods = Mods::default();
    assert!(!mods.is_on(Mod::TimingIndicator));
    assert!(mods.toggle(Mod::TimingIndicator));
    assert!(mods.is_on(Mod::TimingIndicator));
    assert!(!mods.toggle(Mod::TimingIndicator));
    assert!(!mods.is_on(Mod::TimingIndicator));
}

#[test]
fn a_setting_is_at_its_usual_level_until_it_is_set_and_keeps_it_while_off() {
    let rules = Rules::default();
    let mut mods = Mods::default();
    assert_eq!(Mod::Butterfingers.setting(), Some("HOW OFTEN"));
    assert_eq!(Mod::Butterfingers.levels(&rules), 5);
    assert_eq!(mods.level(Mod::Butterfingers), 3);
    assert_eq!(Mod::Butterfingers.level_words(3, &rules), "60%");
    mods.set_level(Mod::Butterfingers, 5);
    assert_eq!(mods.level(Mod::Butterfingers), 5);
    assert!(!mods.is_on(Mod::Butterfingers));
    mods.toggle(Mod::Butterfingers);
    mods.toggle(Mod::Butterfingers);
    assert_eq!(mods.level(Mod::Butterfingers), 5);
    // A mod with no setting has no levels.
    assert_eq!(Mod::LonePitcher.setting(), None);
    assert_eq!(Mod::LonePitcher.levels(&rules), 0);
}

#[test]
fn a_setting_put_higher_than_it_goes_is_brought_back_to_the_most_there_is() {
    let rules = Rules::default();
    let mut mods = Mods::default();
    mods.set_level(Mod::Butterfingers, 9);
    mods.set_level(Mod::MoonBall, 2);
    mods.keep_within(&rules);
    let most = Mod::Butterfingers.levels(&rules);
    assert_eq!(most, 5);
    assert_eq!(mods.level(Mod::Butterfingers), most);
    // One that was within what there is stays where it was put.
    assert_eq!(mods.level(Mod::MoonBall), 2);
    // Rules with fewer levels bring it down further.
    let fewer = Rules::layered(&[("fewer", "[butterfingers]\nchance = [10, 50]\n")]).unwrap();
    mods.keep_within(&fewer);
    assert_eq!(mods.level(Mod::Butterfingers), 2);
}

#[test]
fn the_choice_comes_back_as_it_was_saved() {
    let file = std::env::temp_dir().join(format!("bb-mods-{}.toml", std::process::id()));
    let mut mods = Mods::default();
    mods.set(Mod::TimingIndicator, true);
    mods.set(Mod::Butterfingers, true);
    mods.set_level(Mod::Butterfingers, 4);
    mods.save(&file).unwrap();
    assert_eq!(Mods::load(&file), mods);
    mods.set(Mod::Butterfingers, false);
    mods.levels.clear();
    // A mod the game does not have is passed over.
    std::fs::write(&file, "on = [\"long_gone\", \"timing_indicator\"]").unwrap();
    assert_eq!(Mods::load(&file), mods);
    // No file, or nonsense in it, leaves every mod off.
    std::fs::write(&file, "not a table at all [").unwrap();
    assert_eq!(Mods::load(&file), Mods::default());
    std::fs::remove_file(&file).unwrap();
    assert_eq!(Mods::load(&file), Mods::default());
}

proptest! {
    // Each case writes a file and reads it back.
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn any_choice_of_mods_and_levels_comes_back_as_it_was_saved(
        // For each mod, whether it is on, and the level its setting has
        // been put to if it has been put to one. A mod with no setting
        // keeps a level all the same.
        chosen in prop::collection::vec(
            (any::<bool>(), prop::option::of(any::<u8>())),
            Mod::ALL.len(),
        ),
    ) {
        let mut mods = Mods::default();
        for (which, (on, level)) in Mod::ALL.into_iter().zip(chosen) {
            mods.set(which, on);
            if let Some(level) = level {
                mods.set_level(which, level);
            }
        }
        let name = format!("bb-mods-any-{}.toml", std::process::id());
        let file = std::env::temp_dir().join(name);
        mods.save(&file).unwrap();
        let loaded = Mods::load(&file);
        std::fs::remove_file(&file).unwrap();
        prop_assert_eq!(loaded, mods);
    }
}
