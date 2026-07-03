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

impl CanonicalId for u64 {
    fn fmt_canonical_id<W>(&self, f: &mut W) -> fmt::Result
    where W: fmt::Write + ?Sized {
        write!(f, "{}", self)
    }
}

impl CanonicalId for String {
    fn fmt_canonical_id<W>(&self, f: &mut W) -> fmt::Result
    where W: fmt::Write + ?Sized {
        write!(f, "{}", self)
    }
}
