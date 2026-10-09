//! A colour as the art's files write it, and a change made to colours.

use serde::{Deserialize, Serialize};

/// Written as `#rrggbbaa`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl From<Color> for String {
    fn from(c: Color) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", c.r, c.g, c.b, c.a)
    }
}

impl TryFrom<String> for Color {
    type Error = String;

    fn try_from(s: String) -> Result<Self, String> {
        let hex = s.strip_prefix('#').unwrap_or(&s);
        // Every one of the eight has to be a hex digit. Reading a pair as
        // a number would let a plus sign by in place of the first of them.
        if hex.len() != 8 || !hex.bytes().all(|digit| digit.is_ascii_hexdigit()) {
            return Err(format!("expected a colour like #rrggbbaa, got {s:?}"));
        }
        let byte = |i: usize| {
            u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("bad colour {s:?}: {e}"))
        };
        Ok(Color {
            r: byte(0)?,
            g: byte(2)?,
            b: byte(4)?,
            a: byte(6)?,
        })
    }
}

/// Each channel becomes `channel * mult + add`, with `add` on a 0-255 scale.
/// Channel order is red, green, blue, alpha.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorTransform {
    pub mult: [f64; 4],
    pub add: [i16; 4],
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn color_round_trips_through_its_hex_form() {
        let c = Color {
            r: 0x12,
            g: 0xab,
            b: 0x00,
            a: 0x4d,
        };
        assert_eq!(String::from(c), "#12ab004d");
        assert_eq!(Color::try_from("#12ab004d".to_owned()), Ok(c));
    }

    #[test]
    fn color_rejects_malformed_text() {
        assert!(Color::try_from("#12ab00".to_owned()).is_err());
        assert!(Color::try_from("#zzzzzzzz".to_owned()).is_err());
    }

    /// Marks a colour might be written with, rightly or wrongly: figures and
    /// letters that are hex digits and one that is not, signs, the hash, and
    /// letters that take more than one byte to write.
    const MARKS: [char; 12] = [
        '0', '7', '9', 'a', 'f', 'C', 'g', '+', '-', '#', '\u{e9}', '\u{6f22}',
    ];

    proptest! {
        #[test]
        fn any_colour_is_written_as_a_hash_and_eight_small_hex_digits_and_reads_back_the_same(
            r: u8,
            g: u8,
            b: u8,
            a: u8,
        ) {
            let colour = Color { r, g, b, a };
            let written = String::from(colour);
            let digits = written.strip_prefix('#').expect("a hash to begin with");
            prop_assert_eq!(digits.len(), 8, "{}", written);
            let small = |digit: u8| matches!(digit, b'0'..=b'9' | b'a'..=b'f');
            prop_assert!(digits.bytes().all(small), "{}", written);
            prop_assert_eq!(Color::try_from(written.clone()), Ok(colour));
            // The hash may be left off, and the letters may be capitals.
            prop_assert_eq!(Color::try_from(digits.to_owned()), Ok(colour));
            prop_assert_eq!(Color::try_from(written.to_uppercase()), Ok(colour));
        }

        #[test]
        fn anything_but_eight_hex_digits_is_refused_and_eight_hex_digits_never_are(
            marks in prop::collection::vec(prop::sample::select(&MARKS[..]), 0..=10),
        ) {
            let text: String = marks.into_iter().collect();
            // Reading it must not fall over on the way, either: letters of
            // more than one byte are not to be cut in half.
            let read = Color::try_from(text.clone());
            let digits = text.strip_prefix('#').unwrap_or(&text);
            // Eight hex digits are always a colour, and nothing else is.
            let hex = digits.len() == 8 && digits.bytes().all(|digit| digit.is_ascii_hexdigit());
            prop_assert_eq!(read.is_ok(), hex, "{:?} was read as {:?}", text, read);
        }
    }

    #[test]
    fn a_sign_is_not_let_by_in_place_of_a_hex_digit() {
        // The text is read two letters at a time, each pair as a number,
        // and a number may be written with a plus sign before it. This is
        // no colour at all, and was once read as one.
        for text in ["#+1+2+3+4", "#1+2+3+4+", "#-1-2-3-4", "+1+2+3+4"] {
            let read = Color::try_from(text.to_owned());
            assert!(read.is_err(), "{text:?} was read as {read:?}");
        }
    }
}
