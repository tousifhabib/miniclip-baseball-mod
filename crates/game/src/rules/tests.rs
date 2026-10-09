use super::layers::BUILT_IN;
use super::plate_mods::GoldenRules;
use super::shapes::{Area, Levels, Span};
use super::*;
use crate::settings::Difficulty;

#[test]
fn the_built_in_rules_are_sound() {
    let rules = Rules::layered(&[]).unwrap();
    assert_eq!(rules.game.outs, 3);
    assert_eq!(*rules.game.runs_down.at(Difficulty::Hard), 3);
}

#[test]
fn butterfingers_has_a_chance_for_each_of_its_levels() {
    let chance = Rules::default().butterfingers.chance;
    assert_eq!(chance.count(), 5);
    assert_eq!(chance.at(1), Some(20));
    assert_eq!(chance.at(5), Some(100));
    // A level there is none of is the nearest there is.
    assert_eq!(chance.at(0), Some(20));
    assert_eq!(chance.at(9), Some(100));
    let none = Levels::<u32>::default();
    assert_eq!((none.count(), none.at(3)), (0, None));
}

#[test]
fn heat_takes_time_off_a_pitch_up_to_the_most_there_can_be() {
    let rules = Rules::default().heat;
    assert_eq!(rules.time(0), 1.0);
    assert!((rules.time(1) - 0.94).abs() < 1e-6);
    assert!((rules.time(8) - 0.52).abs() < 1e-6);
    assert_eq!(rules.time(8), rules.time(30));
    let span = Span { low: 35, high: 49 };
    assert_eq!(span.times(0.5), Span { low: 18, high: 25 });
    assert_eq!(span.times(0.0), Span { low: 1, high: 1 });
}

#[test]
fn a_pitcher_tires_between_his_fresh_pitches_and_the_ones_that_spend_him() {
    let rules = Rules::default();
    let arm = &rules.tired_arm;
    assert_eq!(arm.tired(0), 0.0);
    assert_eq!(arm.tired(arm.fresh), 0.0);
    assert_eq!(arm.tired(arm.spent), 1.0);
    assert_eq!(arm.tired(arm.spent + 50), 1.0);
    let half = arm.tired((arm.fresh + arm.spent) / 2);
    assert!((half - 0.5).abs() < 0.05, "{half}");
    // Fresh, he pitches as he always did.
    let usual = rules.pitch.at(Difficulty::Medium);
    assert_eq!(arm.pitch(usual, 0.0), *usual);
    // Spent, he is slower, and aims at more than the strike zone
    // about the same middle.
    let spent = arm.pitch(usual, 1.0);
    assert_eq!(
        spent.speed.low,
        (usual.speed.low as f32 * arm.slow).round() as u32
    );
    let middle = |area: &Area| (area.x + area.width / 2.0, area.y + area.height / 2.0);
    assert_eq!(middle(&spent.target), middle(&usual.target));
    assert_eq!(spent.target.width, usual.target.width * arm.wild);
    assert_eq!(spent.target.height, usual.target.height * arm.wild);
    assert_eq!(spent.window, usual.window);
}

#[test]
fn a_rally_makes_a_run_worth_one_more_for_each_batter_up_to_the_most() {
    let rules = Rules::default().rally;
    assert_eq!(rules.worth(0), 1);
    assert_eq!(rules.worth(1), 2);
    assert_eq!(rules.worth(rules.most), rules.most + 1);
    assert_eq!(rules.worth(rules.most + 7), rules.most + 1);
}

#[test]
fn every_fifth_pitch_is_gold() {
    let rules = Rules::default().golden;
    let gold: Vec<u32> = (0..=16).filter(|&number| rules.is_gold(number)).collect();
    assert_eq!(gold, [5, 10, 15]);
    let never = GoldenRules { every: 0, ..rules };
    assert!(!never.is_gold(5));
}

#[test]
fn a_level_past_the_last_is_the_last_and_a_list_of_none_has_no_number() {
    assert_eq!(Levels::from(vec![1.0, 2.0]).at(9), Some(2.0));
    assert_eq!(Levels::<f32>::default().at(1), None);
}

#[test]
fn a_layer_changes_only_what_it_names() {
    let rules = Rules::layered(&[("a mod", "[match.runs_down]\nhard = 5\n")]).unwrap();
    assert_eq!(*rules.game.runs_down.at(Difficulty::Hard), 5);
    assert_eq!(*rules.game.runs_down.at(Difficulty::Easy), 1);
    assert_eq!(rules.game.outs, 3);
}

#[test]
fn the_last_layer_to_name_a_number_wins() {
    let rules = Rules::layered(&[
        ("first", "[match]\nouts = 1\n"),
        ("second", "[match]\nouts = 2\n"),
    ])
    .unwrap();
    assert_eq!(rules.game.outs, 2);
}

#[test]
fn a_number_the_game_does_not_have_is_refused_by_name() {
    let error = Rules::layered(&[("typo.toml", "[match]\nouts_allowed = 4\n")]).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("typo.toml"), "{message}");
    assert!(message.contains("outs_allowed"), "{message}");
}

#[test]
fn a_value_of_the_wrong_kind_is_refused() {
    let error = Rules::layered(&[("wrong.toml", "[match]\nouts = \"three\"\n")]).unwrap_err();
    assert!(format!("{error:#}").contains("wrong.toml"));
}

#[test]
fn a_file_that_is_not_toml_is_refused_with_its_name() {
    let error = Rules::layered(&[("broken.toml", "[match\nouts = 3")]).unwrap_err();
    assert!(format!("{error:#}").contains("broken.toml"));
}

#[test]
fn a_mistake_is_blamed_on_the_layer_that_made_it() {
    let error = Rules::layered(&[
        ("good.toml", "[match]\nouts = 4\n"),
        ("bad.toml", "[match]\nnonsense = 1\n"),
    ])
    .unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("bad.toml"), "{message}");
    assert!(!message.contains("good.toml"), "{message}");
}

#[test]
fn the_built_in_rules_laid_over_themselves_change_nothing() {
    let rules = Rules::layered(&[("themselves", BUILT_IN)]).unwrap();
    assert_eq!(rules, Rules::default());
}

#[test]
fn a_number_the_game_cannot_be_played_by_is_refused_and_named() {
    let wrong = |layer: &str| {
        let error = Rules::layered(&[("a mod", layer)]).expect_err("rules to be refused");
        format!("{error:#}")
    };
    assert_eq!(
        wrong("[pitch.hard.speed]\nlow = 80\nhigh = 40\n"),
        "in a mod: `pitch.hard.speed` has its low above its high"
    );
    assert_eq!(
        wrong("[pitch.easy.speed]\nlow = 0\n"),
        "in a mod: `pitch.easy.speed` has to be at least 1"
    );
    assert_eq!(
        wrong("[field]\nwall = 0.0\n"),
        "in a mod: `field.wall` has to be more than nought"
    );
    assert_eq!(
        wrong("[shift]\nmost = -0.1\n"),
        "in a mod: `shift.most` cannot be less than nought"
    );
    assert_eq!(
        wrong("[field]\ngravity = nan\n"),
        "in a mod: `field.gravity` is not a number the game can do sums with"
    );
    assert_eq!(
        wrong("[moon]\nslow = [1.5, inf]\n"),
        "in a mod: `moon.slow` is not a number the game can do sums with"
    );
}
