use proptest::prelude::*;

use super::*;
use crate::rules::Rules;

proptest! {
    #[test]
    fn a_ball_sent_to_come_down_so_far_off_after_so_many_frames_does_just_that(
        // Towards anywhere between the foul lines, to come down short of
        // the wall, which would get in its way.
        across in 0.0f32..=1.0,
        carry in 100.0f32..800.0,
        frames in 4u16..=600,
        peak in 5.0f32..400.0,
    ) {
        let (rules, ground) = (Rules::default().field, Ground::default());
        let (left, right) = ground.foul;
        let mark = (left + (right - left) * across, ground.mark_y);
        let mut ball = Ball::sent(ground.home, mark, carry, f32::from(frames), peak);
        let (mut taken, mut highest) = (0, 0.0f32);
        loop {
            taken += 1;
            // Nothing happens to it on the way but coming down.
            match ball.step(ground.home, 0.0, &rules) {
                Happened::Landed => break,
                happened => prop_assert_eq!(happened, Happened::Nothing),
            }
            highest = highest.max(ball.height);
            prop_assert!(taken < frames, "still up after its {} frames", frames);
        }
        prop_assert_eq!(taken, frames);
        let far = reach(ground.home, ball.at);
        prop_assert!((far - carry).abs() < 1.0, "it came down {} off", far);
        // The top of its flight comes between two frames, so the
        // highest it is seen is one part short of the height it was
        // sent to, in as many parts as the frames it is up for.
        let seen = peak * (1.0 - 1.0 / f32::from(frames));
        prop_assert!((highest - seen).abs() < peak * 0.001, "it went {} high", highest);
    }

    #[test]
    fn any_swing_covers_the_field_as_fast_towards_any_part_of_it(
        // Any timing and any height of the ring, sent anywhere between the
        // foul lines.
        power in 9.0f32..=30.0,
        under in -60.0f32..=60.0,
        across in 0.0f32..=1.0,
    ) {
        let (rules, ground) = (Rules::default(), Ground::default());
        let (home, straight) = (ground.home, (303.8, ground.mark_y));
        let mark = ground.foul.0 + across * (ground.foul.1 - ground.foul.0);
        let swing = |aside: f32| Contact { power, under, aside };
        let sent = swing((mark - straight.0) * rules.field.aim_share);
        let hit = |contact: &Contact| Ball::hit(home, straight, contact, &rules.hit, &rules.field);
        let (mut up_the_middle, mut ball) = (hit(&swing(0.0)), hit(&sent));
        for frame in 0..900 {
            // Until either comes down or comes to the wall, which the
            // last place of a sum may put a frame apart.
            let happened = [
                up_the_middle.step(home, sent.miss(), &rules.field),
                ball.step(home, sent.miss(), &rules.field),
            ];
            if happened != [Happened::Nothing; 2] {
                break;
            }
            let (far, as_far) = (covered(home, ball.at), covered(home, up_the_middle.at));
            prop_assert!((far - as_far).abs() < 0.5, "frame {}: {} and {}", frame, far, as_far);
            prop_assert!((ball.height - up_the_middle.height).abs() < 0.001);
        }
        // And it went the way it was sent.
        let is_across = ground.across(ball.at);
        prop_assert!((is_across - across).abs() < 0.001, "{} across", is_across);
    }

    #[test]
    fn any_point_of_the_ground_is_as_far_across_and_as_far_off_as_was_asked_for(
        across in 0.0f32..=1.0,
        far in 50.0f32..1200.0,
    ) {
        let ground = Ground::default();
        let at = ground.point(across, far);
        let (is_across, is_far) = (ground.across(at), reach(ground.home, at));
        prop_assert!((is_across - across).abs() < 0.001, "{} across", is_across);
        prop_assert!((is_far - far).abs() < 0.1, "{} off", is_far);
    }
}
