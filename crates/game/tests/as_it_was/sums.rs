//! What a frame's facts are put into: a running sum to compare, or a page to
//! read.
//!
//! Everything that is taken in of a frame goes through [`Sink`], so the page
//! that is written out when two games have to be compared by eye can never
//! say less than the sum that found them different.

use std::fmt::Write;

/// Somewhere to put the facts of a frame, one at a time and in order.
pub trait Sink {
    /// A count, a number of the art's, a frame: anything whole.
    fn number(&mut self, name: &str, number: u64);
    /// A position, a size, a share of a colour.
    fn float(&mut self, name: &str, float: f32);
    fn words(&mut self, name: &str, words: &str);
    /// The facts so far were about one thing, and the next are about another.
    fn next(&mut self) {}

    fn flag(&mut self, name: &str, flag: bool) {
        self.number(name, u64::from(flag));
    }
}

/// A running sum of everything put into it. Two sums are the same when the
/// same things went in, in the same order, and for nothing else that will
/// ever be met by chance.
///
/// It is worked out here and not by anything in the standard library, whose
/// sums may change from one version of Rust to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sum(u64);

impl Sum {
    pub fn new() -> Sum {
        Sum(0xcbf2_9ce4_8422_2325)
    }

    /// The sum, as it is written down: sixteen hexadecimal figures.
    pub fn written(self) -> String {
        format!("{:016x}", self.0)
    }

    fn take(&mut self, word: u64) {
        self.0 = (self.0 ^ word).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        self.0 ^= self.0 >> 32;
    }
}

impl Sink for Sum {
    fn number(&mut self, _name: &str, number: u64) {
        self.take(number);
    }

    fn float(&mut self, _name: &str, float: f32) {
        // Every way of not being a number counts as the same one. Nought
        // and minus nought are told apart, as two results that differ.
        let bits = if float.is_nan() {
            f32::NAN.to_bits()
        } else {
            float.to_bits()
        };
        self.take(u64::from(bits));
    }

    fn words(&mut self, _name: &str, words: &str) {
        // How long it is goes in first, so that "ab" then "c" is not "a"
        // then "bc".
        self.take(words.len() as u64);
        for eight in words.as_bytes().chunks(8) {
            let mut word = [0; 8];
            word[..eight.len()].copy_from_slice(eight);
            self.take(u64::from_le_bytes(word));
        }
    }
}

/// The same facts written out to be read, a thing to a line.
#[derive(Default)]
pub struct Page(pub String);

impl Sink for Page {
    fn number(&mut self, name: &str, number: u64) {
        write!(self.0, "{name}={number} ").expect("writing to a string");
    }

    fn float(&mut self, name: &str, float: f32) {
        write!(self.0, "{name}={float:?} ").expect("writing to a string");
    }

    fn words(&mut self, name: &str, words: &str) {
        write!(self.0, "{name}={words:?} ").expect("writing to a string");
    }

    fn next(&mut self) {
        self.0.push('\n');
    }
}

#[test]
fn the_sum_of_known_things_never_changes() {
    assert_eq!(Sum::new().written(), "cbf29ce484222325");
    let mut sum = Sum::new();
    sum.number("a number", 1);
    assert_eq!(sum.written(), "5a8419103ebe48e4");
    sum.float("a float", 0.5);
    sum.words("some words", "the quick brown fox");
    assert_eq!(sum.written(), "c5d20bcca96197b7");
}

#[test]
fn the_sum_tells_apart_what_a_careless_sum_would_not() {
    let of = |put: &dyn Fn(&mut Sum)| {
        let mut sum = Sum::new();
        put(&mut sum);
        sum
    };
    // Where one piece of writing ends and the next begins.
    assert_ne!(
        of(&|sum| {
            sum.words("", "ab");
            sum.words("", "c");
        }),
        of(&|sum| {
            sum.words("", "a");
            sum.words("", "bc");
        }),
    );
    // The order things came in.
    assert_ne!(
        of(&|sum| {
            sum.number("", 1);
            sum.number("", 2);
        }),
        of(&|sum| {
            sum.number("", 2);
            sum.number("", 1);
        }),
    );
    // Nought and minus nought.
    assert_ne!(
        of(&|sum| sum.float("", 0.0)),
        of(&|sum| sum.float("", -0.0))
    );
    // And every way of not being a number is the same.
    assert_eq!(
        of(&|sum| sum.float("", f32::NAN)),
        of(&|sum| sum.float("", -f32::NAN)),
    );
}
