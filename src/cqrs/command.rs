use async_trait::async_trait;
use std::future::Future;

use crate::cqrs::Aggregate;

pub trait Command {
    type Aggregate: Aggregate + ?Sized;

    fn aggregate_id(&self) -> Option<&<Self::Aggregate as Aggregate>::Id>;
}

#[async_trait(?Send)]
pub trait Handler<Cmd: Command> {
    type Result;

    async fn handle(&mut self, cmd: Cmd) -> Self::Result
    where Cmd: 'async_trait;
}

#[async_trait(?Send)]
pub trait Gateway<Cmd: Command, Meta> {
    type Err;
    type Ok;

    async fn send(&self, cmd: Cmd, meta: Meta) -> Result<Self::Ok, Self::Err>
    where
        Cmd: 'async_trait,
        Meta: 'async_trait;
}
