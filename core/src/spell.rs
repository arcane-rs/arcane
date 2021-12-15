//! Helpers with a little bit of type magic 🪄.

use derive_more::Deref;
use ref_cast::RefCast;

/// Helper to hack around specialization.
#[derive(Clone, Copy, Debug, Deref, RefCast)]
#[repr(transparent)]
pub struct Borrowed<T: ?Sized>(pub T);
