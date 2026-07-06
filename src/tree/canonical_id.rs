use std::fmt;

pub(crate) const MAX_CANONICAL_ID_LEN: usize = 64;

pub(crate) fn fmt_escaped<W>(s: &str, f: &mut W) -> fmt::Result
where W: fmt::Write + ?Sized {
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'_' {
            write!(f, "{}", char::from(b))?;
        } else {
            write!(f, "%{b:02X}")?;
        }
    }
    Ok(())
}

/// Generates a deterministic canonical ID.
///
/// `QuorumTree` uses canonical IDs for equality and ordering. Implementations
/// for application node IDs should be stable across process restarts and
/// software versions whenever the logical node identity is unchanged.
///
/// Implementations must be consistent with [`Eq`]: two IDs must emit the same
/// canonical ID if and only if they are equal. If two distinct IDs shared one
/// canonical ID, two structurally different trees could compare as equal.
///
/// User-provided node IDs may emit any string. When a user ID is embedded in a
/// [`Node`](crate::Node), this crate escapes short IDs and hashes long IDs to
/// keep tree IDs unambiguous and bounded.
pub trait CanonicalId {
    /// Writes this value's canonical ID into `f`.
    ///
    /// Implement this method directly when the ID can be written without an
    /// intermediate allocation.
    fn fmt_canonical_id<W>(&self, f: &mut W) -> fmt::Result
    where W: fmt::Write + ?Sized;

    /// Returns this value's canonical ID as a [`String`].
    fn canonical_id(&self) -> String {
        let mut s = String::new();
        self.fmt_canonical_id(&mut s).expect("writing to String should not fail");
        s
    }
}

macro_rules! impl_canonical_id {
    ($($t:ty),* $(,)?) => {
        $(impl CanonicalId for $t {
            fn fmt_canonical_id<W>(&self, f: &mut W) -> fmt::Result
            where W: fmt::Write + ?Sized {
                write!(f, "{}", self)
            }
        })*
    };
}

impl_canonical_id!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, String, &str
);
