use futures::{
    future,
    stream::{self, LocalBoxStream, StreamExt},
};

struct Event;

trait TransformedByDefault<Adapter> {
    fn transform(self, adapter: Adapter) -> LocalBoxStream<'static, Event>;
}

impl<Adapter, Ev> TransformedByDefault<Adapter> for &Ev {
    fn transform(self, _: Adapter) -> LocalBoxStream<'static, Event> {
        stream::empty().boxed_local()
    }
}

trait TransformedBy<Adapter> {
    fn transform(self, adapter: Adapter) -> LocalBoxStream<'static, Event>;
}

struct A;

struct Skipped;

struct Once;

impl TransformedBy<A> for Once {
    fn transform(self, _: A) -> LocalBoxStream<'static, Event> {
        stream::once(future::ready(Event)).boxed_local()
    }
}

#[tokio::main]
async fn main() {
    trait Test {}

    impl Test for A {}

    let once = Once;
    let skipped = Skipped;

    assert_eq!(once.transform(A).collect::<Vec<_>>().await.len(), 1);
    assert_eq!(skipped.transform(A).collect::<Vec<_>>().await.len(), 0);
}

trait Foo {}
trait MarkerFoo {}

impl<T: MarkerFoo> Foo for T {}

impl Foo for A {}
