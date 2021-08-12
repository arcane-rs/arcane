use derive_more::{Deref, DerefMut};

use super::Nothing;

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct HList<Head, Tail: ?Sized = Nothing<Head>> {
    #[deref]
    #[deref_mut]
    head: Head,
    tail: Tail,
}

impl<T> HList<T> {
    #[inline]
    #[must_use]
    pub fn data(data: T) -> Self {
        Self {
            head: data,
            tail: Nothing::here(),
        }
    }
}

impl<Head, Tail> HList<Head, Tail> {
    #[inline]
    #[must_use]
    pub fn and<T>(
        self,
        data: T,
    ) -> HList<Head, <HList<T, Tail> as SinkHead<T>>::Out>
    where
        HList<T, Tail>: SinkHead<T>,
    {
        HList {
            head: data,
            tail: self,
        }
        .sink_head()
    }

    #[inline]
    #[must_use]
    pub fn into_tuple(self) -> (Head, Tail) {
        (self.head, self.tail)
    }
}

impl<Head, Tail: ?Sized> HList<Head, Tail> {
    #[inline]
    #[must_use]
    pub fn this(&self) -> &Head {
        &self.head
    }

    #[inline]
    #[must_use]
    pub fn this_mut(&mut self) -> &mut Head {
        &mut self.head
    }

    #[inline]
    #[must_use]
    pub fn other(&self) -> &Tail {
        &self.tail
    }

    #[inline]
    #[must_use]
    pub fn other_mut(&mut self) -> &mut Tail {
        &mut self.tail
    }
}

pub trait SinkHead<T> {
    type Out;

    #[must_use]
    fn sink_head(self) -> Self::Out;
}

impl<Head> SinkHead<Head> for HList<Head> {
    type Out = Self;

    #[inline]
    fn sink_head(self) -> Self::Out {
        self
    }
}

impl<A, B, Tail> SinkHead<A> for HList<A, HList<B, Tail>>
where
    HList<A, Tail>: SinkHead<A>,
{
    type Out = HList<B, <HList<A, Tail> as SinkHead<A>>::Out>;

    #[inline]
    fn sink_head(self) -> Self::Out {
        HList {
            head: self.tail.head,
            tail: HList {
                head: self.head,
                tail: self.tail.tail,
            }
            .sink_head(),
        }
    }
}
