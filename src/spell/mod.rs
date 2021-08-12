pub mod hlist;
pub mod maybe;

pub use self::{
    hlist::{HList as With, HList as And},
    maybe::{Just, Just as Existing, Maybe, Nothing, Nothing as Absent},
};

// ------------------

use crate::es;

pub trait EventHydration<Evs: ?Sized, Agg>: Maybe<Agg> {
    type Hydrated: Maybe<Agg>;

    #[must_use]
    fn hydrate(self, events: &Evs) -> Self::Hydrated;
}
/*
impl<Ev, Agg> EventHydrated<Ev> for Existing<Agg>
where
    Ev: es::Event + ?Sized,
    Agg: es::EventSourced<Ev>,
{
    type Hydrated = Self;

    #[inline]
    fn hydrate(mut self, event: &Ev) -> Self::Hydrated {
        self.0.apply(event);
        self
    }
}

impl<Ev, Agg> EventHydrated<Ev> for Absent<Agg>
where
    Ev: es::Event + ?Sized,
    Agg: es::EventInitialized<Ev>,
{
    type Hydrated = Existing<Agg>;

    #[inline]
    fn hydrate(self, event: &Ev) -> Self::Hydrated {
        Just(Agg::initialize(event))
    }
}
*/
/*
impl<Ev, Agg, E, T> EventHydrated<With<E, Nothing<Ev>>> for T
where
    Ev: es::Event,
    E: Maybe<Ev>,
    T: Maybe<Agg> + EventHydrated<Ev>,
{
    type Hydrated = <T as EventHydrated<Ev>>::Hydrated;

    fn hydrate(self, list: &With<E, Nothing<Ev>>) -> Self::Hydrated {
        if let Some(event) = list.this().as_option() {
            self.hydrate(event)
        } else {
            self
        }
    }
}*/

/*
impl<Ev, Agg> EventHydration<With<Just<Ev>, Nothing<Ev>>, Agg> for Absent<Agg>
where
    Ev: es::Event,
    Agg: es::EventInitialized<Ev>,
{
    type Hydrated = Existing<Agg>;

    #[inline]
    fn hydrate(self, events: &With<Just<Ev>, Nothing<Ev>>) -> Self::Hydrated {
        Existing(Agg::initialize(&events.this().0))
    }
}*/

impl<T, Agg> EventHydration<Nothing<T>, Agg> for Option<Agg> {
    type Hydrated = Self;

    #[inline]
    fn hydrate(self, _: &Nothing<T>) -> Self::Hydrated {
        self
    }
}

impl<Ev, Agg, Tail> EventHydration<With<Option<Ev>, Tail>, Agg> for Option<Agg>
where
    Ev: es::Event,
    Agg: es::EventSourced<Ev>,
    Self: EventHydration<Tail, Agg>,
{
    type Hydrated = <Self as EventHydration<Tail, Agg>>::Hydrated;

    fn hydrate(mut self, events: &With<Option<Ev>, Tail>) -> Self::Hydrated {
        if let Some(event) = events.this().as_ref() {
            if let Some(agg) = &mut self {
                agg.apply(event)
            }
        }
        self.hydrate(events.other())
    }
}

impl<Ev, Agg, Tail> EventHydration<With<Ev, Tail>, Agg> for Option<Agg>
where
    Ev: es::Event,
    Agg: es::EventSourced<Ev>,
    Self: EventHydration<Tail, Agg>,
{
    type Hydrated = <Self as EventHydration<Tail, Agg>>::Hydrated;

    fn hydrate(mut self, events: &With<Ev, Tail>) -> Self::Hydrated {
        if let Some(agg) = &mut self {
            agg.apply(events.this())
        }
        self.hydrate(events.other())
    }
}

impl<Ev, Agg, Tail> EventHydration<With<Option<Init<Ev>>, Tail>, Agg>
    for Option<Agg>
where
    Ev: es::Event,
    Agg: es::EventInitialized<Ev>,
    Self: EventHydration<Tail, Agg>,
{
    type Hydrated = <Self as EventHydration<Tail, Agg>>::Hydrated;

    fn hydrate(self, events: &With<Option<Init<Ev>>, Tail>) -> Self::Hydrated {
        match (self, events.this()) {
            (Some(agg), Some(ev)) => {
                // boom?!
                Some(agg)
            },
            (Some(agg), None) => Some(agg),
            (None, Some(ev)) => Some(Agg::initialize(&*ev)),
            (None, None) => None,
        }
        .hydrate(events.other())
    }
}

use derive_more::{Deref, DerefMut};

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct Init<T>(pub T);

// Some(A) |> None     -> A init T, and then apply if any

// Some(A) |> Some(T)  -> T apply A
// None    |> Some(T)  -> no-op
// None    |> None     -> reject


/*
pub trait HandableCommand<Cmd, Agg> {
    fn handle_presense();
    fn handle_absence();

}

impl<Cmd, Agg> HandableCommand<Cmd, Agg> for Existing<Agg>{}

impl<Cmd, Agg> HandableCommand<Cmd, Agg> for Absent<Agg>{}
*/
