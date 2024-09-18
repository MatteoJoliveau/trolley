use std::{fmt::Debug, ops::Deref};

pub use futures::stream::BoxStream;

pub use codec::Codec;
pub use service::*;
pub use transport::Transport;

pub mod codec;
mod service;
pub mod transport;

pub type MessageStream<M, E> = BoxStream<'static, Result<M, E>>;

pub trait Message {
    type Error: Debug;

    fn payload(&self) -> &[u8];
    fn ack(&self) -> impl std::future::Future<Output = Result<(), Self::Error>>;
    fn nack(&self) -> impl std::future::Future<Output = Result<(), Self::Error>>;
}

pub trait Handler<M>: Send + Sync {
    fn handle(
        &self,
        ctx: DeliveryContext,
        message: M,
    ) -> impl futures::Future<Output = DeliveryOutcome>;
}

impl<T, H, M> Handler<M> for T
where
    T: Deref<Target = H> + Send + Sync,
    H: Handler<M>,
    M: Send + Sync + 'static,
{
    async fn handle(&self, ctx: DeliveryContext, message: M) -> DeliveryOutcome {
        self.deref().handle(ctx, message).await
    }
}

pub struct DeliveryContext {}

impl DeliveryContext {
    fn new() -> Self {
        Self {}
    }

    pub fn shutdown(&self) -> DeliveryOutcome {
        Outcome::Shutdown.into()
    }

    pub fn ack(&self) -> DeliveryOutcome {
        Outcome::Ack.into()
    }

    pub fn nack(&self) -> DeliveryOutcome {
        Outcome::Nack.into()
    }
}

pub struct DeliveryOutcome(Outcome);

enum Outcome {
    Ack,
    Nack,
    Shutdown,
}

impl From<Outcome> for DeliveryOutcome {
    fn from(outcome: Outcome) -> Self {
        Self(outcome)
    }
}
