use crate::{cqrs, es::aggregate};

pub trait Command: cqrs::Command {
    #[inline]
    fn expected_version(&self) -> Option<aggregate::Version> {
        None
    }
}
