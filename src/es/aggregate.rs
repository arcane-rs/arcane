use derive_more::{AsRef, Display, From};
use smart_default::SmartDefault;

use crate::{
    cqrs::Aggregate,
    es::{event, Event},
};

#[derive(AsRef, Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Hydrated<Agg> {
    ver: Version,
    snapshot_ver: Option<Version>,
    #[as_ref]
    state: Agg,
}

impl<Agg> Hydrated<Agg> {
    #[inline]
    #[must_use]
    pub fn from_version(agg: Agg, ver: Version) -> Self {
        Self {
            ver,
            snapshot_ver: None,
            state: agg,
        }
    }

    #[inline]
    #[must_use]
    pub fn from_snapshot(agg: Agg, ver: Version) -> Self {
        Self {
            ver,
            snapshot_ver: Some(ver),
            state: agg,
        }
    }

    #[inline]
    pub fn id(&self) -> &Agg::Id
    where
        Agg: Aggregate,
    {
        self.state.id()
    }

    #[inline]
    pub fn version(&self) -> Version {
        self.ver
    }

    #[inline]
    pub fn snapshot_version(&self) -> Option<Version> {
        self.snapshot_ver
    }

    #[inline]
    pub fn set_snapshot_version(&mut self, new: Version) {
        self.snapshot_ver = Some(new);
    }

    #[inline(always)]
    pub fn state(&self) -> &Agg {
        &self.state
    }

    #[inline]
    pub fn map<Proj, F>(self, f: F) -> Hydrated<Proj>
    where
        F: FnOnce(Agg) -> Proj,
    {
        Hydrated {
            ver: self.ver,
            snapshot_ver: self.snapshot_ver,
            state: f(self.state),
        }
    }

    #[inline]
    pub fn map_into<Proj: From<Agg>>(self) -> Hydrated<Proj> {
        self.map(Into::into)
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Display,
    Eq,
    From,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    SmartDefault,
)]
pub enum Version {
    #[default]
    #[display(fmt = "initial")]
    Initial,
    Number(event::Number),
}
