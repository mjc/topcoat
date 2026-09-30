//! Storage and parsing helpers shared by the subtag types.

use std::{
    cmp::Ordering,
    fmt::{self, Debug},
};

/// A fixed-capacity ASCII alphanumeric string, padded with NUL bytes.
///
/// The layout matches ICU4X's subtag storage, so the raw bytes convert to its
/// types directly. Comparisons and hashing use the stored bytes, so two
/// buffers with the same text are equal, and a shorter text sorts before a
/// longer one with the same prefix.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Buffer<const N: usize>([u8; N]);

impl<const N: usize> Buffer<N> {
    /// Copies `bytes` into a buffer. Returns `None` when the text is empty,
    /// longer than `N`, or not entirely ASCII alphanumeric.
    pub(crate) const fn new(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() || bytes.len() > N {
            return None;
        }
        let mut stored = [0; N];
        let mut index = 0;
        while index < bytes.len() {
            if !bytes[index].is_ascii_alphanumeric() {
                return None;
            }
            stored[index] = bytes[index];
            index += 1;
        }
        Some(Self(stored))
    }

    /// Returns the stored bytes, NUL padded to the capacity.
    pub(crate) const fn into_raw(self) -> [u8; N] {
        self.0
    }

    pub(crate) const fn as_str(&self) -> &str {
        let (text, _) = self.0.split_at(self.len());
        match str::from_utf8(text) {
            Ok(text) => text,
            Err(_) => unreachable!(),
        }
    }

    /// Returns the length of the text: the position of the first NUL byte,
    /// or the capacity when there is none.
    pub(crate) const fn len(&self) -> usize {
        let mut index = 0;
        while index < N && self.0[index] != 0 {
            index += 1;
        }
        index
    }

    /// Returns whether every byte is an ASCII letter.
    pub(crate) const fn is_alphabetic(&self) -> bool {
        let mut index = 0;
        while index < self.len() {
            if !self.0[index].is_ascii_alphabetic() {
                return false;
            }
            index += 1;
        }
        true
    }

    /// Returns whether every byte is an ASCII digit.
    pub(crate) const fn is_numeric(&self) -> bool {
        let mut index = 0;
        while index < self.len() {
            if !self.0[index].is_ascii_digit() {
                return false;
            }
            index += 1;
        }
        true
    }

    /// Returns whether the first byte is an ASCII digit.
    pub(crate) const fn starts_with_digit(&self) -> bool {
        self.0[0].is_ascii_digit()
    }

    /// Compares the texts byte by byte, like the derived `Ord`, for use in
    /// const context.
    pub(crate) const fn compare(&self, other: &Self) -> Ordering {
        let mut index = 0;
        while index < N {
            if self.0[index] < other.0[index] {
                return Ordering::Less;
            }
            if self.0[index] > other.0[index] {
                return Ordering::Greater;
            }
            index += 1;
        }
        Ordering::Equal
    }

    pub(crate) const fn to_lowercase(mut self) -> Self {
        let mut index = 0;
        while index < self.len() {
            self.0[index] = self.0[index].to_ascii_lowercase();
            index += 1;
        }
        self
    }

    pub(crate) const fn to_uppercase(mut self) -> Self {
        let mut index = 0;
        while index < self.len() {
            self.0[index] = self.0[index].to_ascii_uppercase();
            index += 1;
        }
        self
    }

    /// Upper-cases the first byte and lower-cases the rest.
    pub(crate) const fn to_titlecase(self) -> Self {
        let mut buffer = self.to_lowercase();
        buffer.0[0] = buffer.0[0].to_ascii_uppercase();
        buffer
    }
}

impl<const N: usize> Debug for Buffer<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_text_up_to_the_capacity() {
        assert_eq!(Buffer::<4>::new(b"Latn").unwrap().as_str(), "Latn");
        assert_eq!(Buffer::<4>::new(b"a").unwrap().as_str(), "a");
        assert!(Buffer::<4>::new(b"Latin").is_none());
        assert!(Buffer::<4>::new(b"").is_none());
    }

    #[test]
    fn rejects_bytes_outside_ascii_alphanumerics() {
        assert!(Buffer::<8>::new(b"en-US").is_none());
        assert!(Buffer::<8>::new("dé".as_bytes()).is_none());
        assert!(Buffer::<8>::new(b"en US").is_none());
    }

    #[test]
    fn classifies_letters_and_digits() {
        let letters = Buffer::<4>::new(b"Latn").unwrap();
        let digits = Buffer::<4>::new(b"419").unwrap();
        let mixed = Buffer::<4>::new(b"1a").unwrap();

        assert!(letters.is_alphabetic() && !letters.is_numeric());
        assert!(digits.is_numeric() && !digits.is_alphabetic());
        assert!(!mixed.is_alphabetic() && !mixed.is_numeric());
        assert!(mixed.starts_with_digit() && !letters.starts_with_digit());
    }

    #[test]
    fn folds_case() {
        let buffer = Buffer::<4>::new(b"lATn").unwrap();

        assert_eq!(buffer.to_lowercase().as_str(), "latn");
        assert_eq!(buffer.to_uppercase().as_str(), "LATN");
        assert_eq!(buffer.to_titlecase().as_str(), "Latn");
    }

    #[test]
    fn equality_and_order_follow_the_text() {
        let short = Buffer::<8>::new(b"ab").unwrap();
        let long = Buffer::<8>::new(b"abc").unwrap();

        assert_eq!(short, Buffer::<8>::new(b"ab").unwrap());
        assert_ne!(short, long);
        assert!(short < long);
        assert!(Buffer::<8>::new(b"b").unwrap() > long);
    }

    #[test]
    fn const_comparison_matches_the_derived_order() {
        let texts: [&[u8]; 5] = [b"a", b"ab", b"abc", b"b", b"1996"];
        for left in texts {
            for right in texts {
                let left = Buffer::<8>::new(left).unwrap();
                let right = Buffer::<8>::new(right).unwrap();
                assert_eq!(left.compare(&right), left.cmp(&right), "{left:?} {right:?}");
            }
        }
    }

    #[test]
    fn works_in_const_context() {
        const SCRIPT: Buffer<4> = match Buffer::new(b"hant") {
            Some(buffer) => buffer.to_titlecase(),
            None => panic!(),
        };

        assert_eq!(SCRIPT.as_str(), "Hant");
    }
}
