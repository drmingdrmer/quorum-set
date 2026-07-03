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

/// A trait that generates a canonical ID for an instance.
pub trait CanonicalId {
    fn fmt_canonical_id<W>(&self, f: &mut W) -> fmt::Result
    where W: fmt::Write + ?Sized;

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
