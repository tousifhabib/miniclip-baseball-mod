//! The shift mod: the fielders stand where the ball has been going.

mod common;

use bb_engine::display::child_bounds;
use bb_game::art;
use bb_game::mods::Mod;
use bb_game::play::field::Ground;
use bb_game::rules::Rules;
use bb_game::script::Script;
use common::{long_match, next, number, pitch, said, state};

/// The middle of the batting view, which a hit straight up the field goes
/// to.
const MIDDLE: f32 = 295.0;
/// How far to the side of straight a hit is sent, in pixels of the batting
/// view, to come down well to the left or the right and still be fair.
const LEFT: f32 = -260.0;
const RIGHT: f32 = 240.0;
/// The fielders who roam, and two who do not: the pitcher and the man at
/// first.
const ROAMERS: [&str; 4] = ["fielder1", "fielder2", "fielder4", "fielder5"];
const STAY: [&str; 2] = ["fielder3", "fielder6"];

/// A match long enough for any number of hits, with the timing bar up.
fn game(shift: bool) -> Option<Script> {
    let mut mods = vec![Mod::TimingIndicator];
    if shift {
        mods.push(Mod::TheShift);
    }
    long_match(1, &mods)
}

/// Waits for the pitcher to stand ready for the next pitch, and returns how
/// things stand then.
fn ready(script: &mut Script) -> String {
    for _ in 0..600 {
        let now = state(script);
        if now.contains("Settling") {
            return now;
        }
        script.run("wait 1").unwrap();
    }
    panic!("the next pitch never came: {}", state(script));
}

/// Hits the next pitch `aside` pixels to the side of straight, and asks
/// for the one after.
fn hit(script: &mut Script, aside: f32) {
    let crosses = number(&ready(script), "crossing ").unwrap();
    // A hit goes four pixels aside for each the ball is off the middle,
    // and three the other way for each the ring is off the ball.
    let off = (4.0 * (crosses - MIDDLE) - aside) / 3.0;
    pitch(script, 0, (off, 0.0));
    next(script);
}

/// Where a fielder stands on the field, by his name in the art.
fn stands(script: &Script, name: &str) -> (f32, f32) {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let fielder = stage.find(&main, &["field", name]).unwrap();
    let matrix = stage.child(&fielder).unwrap().matrix;
    (matrix.tx, matrix.ty)
}

/// Where the left fielder's mark is on the little field in the corner.
fn little_mark(script: &Script) -> (f32, f32) {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let mut mark = stage.find_symbol(&main, 1092).unwrap();
    mark.push(6);
    let matrix = stage.child(&mark).unwrap().matrix;
    (matrix.tx, matrix.ty)
}

fn all(script: &Script, names: &[&str]) -> Vec<(f32, f32)> {
    names.iter().map(|name| stands(script, name)).collect()
}

/// A man as the batting view draws him behind the pitcher: where the middle
/// of him comes across it, how tall he is, and the depth he is drawn at.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Drawn {
    middle: f32,
    tall: f32,
    depth: u16,
}

fn drawn(script: &Script, path: &[u16]) -> Drawn {
    let (stage, library) = (&script.runner.stage, &script.runner.library);
    let (depth, view) = path.split_last().unwrap();
    let within = stage.to_stage(view).unwrap();
    let [left, top, right, bottom] =
        child_bounds(stage.child(path).unwrap(), within, library).unwrap();
    Drawn {
        middle: (left + right) / 2.0,
        tall: bottom - top,
        depth: *depth,
    }
}

/// The batting view's drawing of a fielder who roams, counting from 0, if
/// it has one: where the art puts him, or where the shift has drawn him
/// again.
fn in_view(script: &Script, who: usize) -> Option<Drawn> {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain")?;
    let again = match who {
        0 => stage.find(&main, &["leftFielder"]),
        3 => stage.find(&main, &["centreFielder"]),
        4 => stage.find(&main, &["rightFielder"]),
        _ => None,
    };
    let path = again.or_else(|| {
        let (_, depth) = art::VIEW_FIELDERS.iter().find(|(index, _)| *index == who)?;
        let mut path = main.clone();
        path.push(*depth);
        let fielder = stage.child(&path)?.symbol == art::VIEW_FIELDER;
        fielder.then_some(path)
    })?;
    Some(drawn(script, &path))
}

/// The men drawn there who do not roam: the umpire and the second baseman.
fn stay_in_view(script: &Script) -> Vec<Drawn> {
    let stage = &script.runner.stage;
    let main = stage.find_named(&[], "gameMain").unwrap();
    let roam = |depth: &u16| art::VIEW_FIELDERS.iter().any(|(_, at)| at == depth);
    let view = stage.clip(&main).unwrap();
    let stay = view.children.iter().filter(|(depth, child)| {
        child.symbol == art::VIEW_UMPIRE
            || (child.symbol == art::VIEW_FIELDER && child.name.is_none() && !roam(depth))
    });
    stay.map(|(&depth, _)| {
        let mut path = main.clone();
        path.push(depth);
        drawn(script, &path)
    })
    .collect()
}

/// How far across the batting view the art's pointer goes for a ball hit
/// from one side of the field to the other.
fn view_across() -> f32 {
    let ground = Ground::default();
    (ground.foul.1 - ground.foul.0) * Rules::default().field.aim_share
}

#[test]
fn nobody_moves_until_three_balls_have_gone_one_way_and_then_they_all_do() {
    let (Some(mut script), Some(mut plain)) = (game(true), game(false)) else {
        return;
    };
    ready(&mut script);
    ready(&mut plain);
    let (roamers, stay) = (all(&script, &ROAMERS), all(&script, &STAY));
    let mark = little_mark(&script);
    // They start where the game has always had them.
    assert_eq!(roamers, all(&plain, &ROAMERS));
    // Behind the pitcher in the batting view are the shortstop, the centre
    // fielder and the right fielder, and two men who stay. The left fielder
    // is out of the picture.
    let view: Vec<Option<Drawn>> = (0..5).map(|who| in_view(&script, who)).collect();
    let stay_view = stay_in_view(&script);
    assert_eq!(
        view,
        (0..5).map(|who| in_view(&plain, who)).collect::<Vec<_>>()
    );
    assert!(view[0].is_none() && view[1].is_some() && view[3].is_some() && view[4].is_some());
    assert_eq!(stay_view.len(), 2);
    for _ in 0..2 {
        hit(&mut script, LEFT);
        let now = ready(&mut script);
        assert!(!now.contains("shifted"), "{now}");
        assert_eq!(all(&script, &ROAMERS), roamers);
        assert_eq!(
            (0..5).map(|who| in_view(&script, who)).collect::<Vec<_>>(),
            view
        );
        assert!(said(&script, "shift").is_empty());
    }
    hit(&mut script, LEFT);
    let now = ready(&mut script);
    assert!(now.contains(", shifted left 0."), "{now}");
    assert_eq!(said(&script, "shift"), ["SHIFT LEFT"]);
    for (was, is) in roamers.iter().zip(all(&script, &ROAMERS)) {
        assert!(is.0 < was.0 - 10.0, "{was:?} to {is:?}");
    }
    // The pitcher and the men at the bases are where they were.
    assert_eq!(all(&script, &STAY), stay);
    // And the little field in the corner shows it.
    assert!(little_mark(&script).0 < mark.0 - 2.0);
    // So do the men behind the pitcher in the batting view. Each has gone
    // as far across it as the art's pointer does for a ball hit as much
    // further over as his fielder has gone, and is no nearer or further off.
    let ground = Ground::default();
    let moved = all(&script, &ROAMERS);
    for (roamer, who) in [(1, 1), (2, 3), (3, 4)] {
        let (was, is) = (view[who].unwrap(), in_view(&script, who).unwrap());
        let over = ground.across(moved[roamer]) - ground.across(roamers[roamer]);
        let slid = is.middle - was.middle;
        assert!(slid < -20.0, "fielder {who}: {was:?} to {is:?}");
        assert!(
            (slid - over * view_across()).abs() < 0.5,
            "fielder {who}: {slid}"
        );
        assert!((is.tall - was.tall).abs() < 0.01, "{was:?} to {is:?}");
    }
    // The two who stay have stayed, and the outfielders, who may now stand
    // behind them, are drawn under them.
    assert_eq!(stay_in_view(&script), stay_view);
    for who in [3, 4] {
        let depth = in_view(&script, who).unwrap().depth;
        assert!(stay_view.iter().all(|stays| depth < stays.depth));
    }
}

#[test]
fn going_the_other_way_brings_them_back_and_then_over() {
    let Some(mut script) = game(true) else {
        return;
    };
    for _ in 0..3 {
        hit(&mut script, LEFT);
    }
    let left = number(&ready(&mut script), "shifted left ").unwrap();
    let mut over = None;
    for hits in 1..=8 {
        hit(&mut script, RIGHT);
        let now = ready(&mut script);
        if let Some(less) = number(&now, "shifted left ") {
            assert!(less < left, "{now}");
        }
        if now.contains("shifted right") && over.is_none() {
            over = Some(hits);
        }
    }
    // Eight balls are all they go by, so by now it is as if the first
    // three had never been.
    assert!(over.is_some());
    assert_eq!(said(&script, "shift"), ["SHIFT RIGHT"]);
    let centre = stands(&script, "fielder4");
    let Some(mut plain) = game(false) else {
        return;
    };
    ready(&mut plain);
    assert!(centre.0 > stands(&plain, "fielder4").0 + 10.0);
    // In the batting view the centre fielder has gone to the right with
    // him, and the left fielder has come into the picture.
    let (was, is) = (in_view(&plain, 3).unwrap(), in_view(&script, 3).unwrap());
    assert!(is.middle > was.middle + 20.0, "{was:?} to {is:?}");
    assert_eq!(in_view(&plain, 0), None);
    let left = in_view(&script, 0).unwrap();
    assert!(left.middle > 0.0 && left.middle < MIDDLE, "{left:?}");
    assert!((left.tall - was.tall).abs() < 0.01, "{left:?}");
}

#[test]
fn without_the_mod_they_stay_where_they_are() {
    let Some(mut script) = game(false) else {
        return;
    };
    ready(&mut script);
    let roamers = all(&script, &ROAMERS);
    let view: Vec<Option<Drawn>> = (0..5).map(|who| in_view(&script, who)).collect();
    for _ in 0..4 {
        hit(&mut script, LEFT);
    }
    let now = ready(&mut script);
    assert!(!now.contains("shifted"), "{now}");
    assert_eq!(all(&script, &ROAMERS), roamers);
    assert_eq!(
        (0..5).map(|who| in_view(&script, who)).collect::<Vec<_>>(),
        view
    );
    assert!(said(&script, "shift").is_empty());
}
