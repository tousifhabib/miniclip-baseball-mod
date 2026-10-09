//! One of a few words: a way to make a type for a field of the art's files
//! that holds one.

/// Makes a type for a field of the art's files that holds one of a few
/// words. The files go on holding the words: a word is read into the name
/// it has here and written back as it was. A word this version does not
/// know is kept as written, so that a file made by a later extractor still
/// reads, and writes back unchanged.
macro_rules! words {
    (
        $(#[$about:meta])*
        $name:ident { $($(#[$each:meta])* $known:ident = $word:literal,)+ }
    ) => {
        $(#[$about])*
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(from = "String", into = "String")]
        pub enum $name {
            $($(#[$each])* $known,)+
            /// A word this version does not know, as it was written.
            Other(String),
        }

        impl $name {
            /// The word, as the files have it.
            pub fn word(&self) -> &str {
                match self {
                    $($name::$known => $word,)+
                    $name::Other(word) => word,
                }
            }
        }

        impl From<&str> for $name {
            fn from(word: &str) -> $name {
                match word {
                    $($word => $name::$known,)+
                    other => $name::Other(other.to_owned()),
                }
            }
        }

        impl From<String> for $name {
            fn from(word: String) -> $name {
                $name::from(word.as_str())
            }
        }

        impl From<$name> for String {
            fn from(word: $name) -> String {
                word.word().to_owned()
            }
        }
    };
}

pub(crate) use words;

#[cfg(test)]
mod tests {
    use crate::{Align, FieldFlag, Look};

    #[test]
    fn a_word_the_format_has_reads_to_its_name_and_is_written_back_as_it_was() {
        for (word, look) in [
            ("up", Look::Up),
            ("over", Look::Over),
            ("down", Look::Down),
            ("hit", Look::Hit),
        ] {
            assert_eq!(Look::from(word), look);
            assert_eq!(String::from(look), word);
        }
        assert_eq!(FieldFlag::from("read_only"), FieldFlag::ReadOnly);
        assert_eq!(FieldFlag::from("html"), FieldFlag::Html);
        assert_eq!(String::from(FieldFlag::UseOutlines), "use_outlines");
        assert_eq!(Align::from("center"), Align::Center);
        assert_eq!(String::from(Align::Right), "right");
    }

    #[test]
    fn a_word_the_format_does_not_have_is_kept_just_as_it_was_written() {
        // A later extractor may write words this version has never heard
        // of. A file with one in it still reads, and writes back unchanged.
        let odd = Align::from("justify");
        assert_eq!(odd, Align::Other("justify".to_owned()));
        assert_eq!(String::from(odd), "justify");
        // Nothing is made of how it is spelt: a known word in capitals is
        // another word.
        assert_eq!(Look::from("UP"), Look::Other("UP".to_owned()));
    }
}
