use std::marker::PhantomData;

use futures::stream::StreamExt;
use serde::de::DeserializeOwned;

use crate::{Codec, DeliveryContext, DeliveryOutcome, Handler, Message, Outcome, Transport};

pub struct Service<M, H, T, C: Codec> {
    name: String,
    handler: H,
    transport: T,
    codec: C,
    hooks: ServiceHooks<C::Error>,
    _message: PhantomData<M>,
}

impl<M, H, T, C> Service<M, H, T, C>
where
    M: DeserializeOwned,
    H: Handler<M>,
    T: Transport,
    C: Codec,
{
    pub async fn start(
        mut self,
    ) -> Result<(), ServiceError<T::Error, C::Error, <<T as Transport>::Message as Message>::Error>>
    {
        self.transport
            .setup(&self.name)
            .await
            .map_err(ServiceError::Transport)?;

        let mut messages = self
            .transport
            .consume(&self.name)
            .await
            .map_err(ServiceError::Transport)?;

        while let Some(message) = messages.next().await {
            let Ok(message) = message else {
                continue;
            };

            let ctx = DeliveryContext::new();
            let payload = match self.codec.deserialize(message.payload()) {
                Ok(payload) => payload,
                Err(err) => {
                    if let Some(ref hook) = self.hooks.on_codec_error {
                        hook(err);
                    }

                    message.nack().await.map_err(ServiceError::Message)?;
                    continue;
                }
            };

            let DeliveryOutcome(outcome) = self.handler.handle(ctx, payload).await;
            match outcome {
                Outcome::Ack => {
                    message.ack().await.map_err(ServiceError::Message)?;
                    continue;
                }
                Outcome::Nack => {
                    message.nack().await.map_err(ServiceError::Message)?;
                    continue;
                }
                Outcome::Shutdown => break,
            }
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError<T, C, M> {
    #[error(transparent)]
    Transport(T),
    #[error(transparent)]
    Codec(C),
    #[error(transparent)]
    Message(M),
}

pub struct ServiceConfig<T, C, CE> {
    name: String,
    transport: T,
    codec: C,
    hooks: ServiceHooks<CE>,
}

impl ServiceConfig<(), (), ()> {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            transport: (),
            codec: (),
            hooks: ServiceHooks::default(),
        }
    }
}

impl<OldTransport, S, H> ServiceConfig<OldTransport, S, H> {
    pub fn with_transport<T: Transport>(self, transport: T) -> ServiceConfig<T, S, H> {
        ServiceConfig {
            name: self.name,
            transport,
            codec: self.codec,
            hooks: self.hooks,
        }
    }
}

impl<T, OldCodec, H> ServiceConfig<T, OldCodec, H> {
    pub fn with_codec<C: Codec>(self) -> ServiceConfig<T, C, C::Error> {
        ServiceConfig {
            name: self.name,
            transport: self.transport,
            codec: C::default(),
            hooks: ServiceHooks::default(),
        }
    }
}

impl<T, C: Codec, CE> ServiceConfig<T, C, CE> {
    pub fn on_codec_error<F: Fn(C::Error) + 'static>(
        self,
        hook: F,
    ) -> ServiceConfig<T, C, C::Error> {
        ServiceConfig {
            name: self.name,
            transport: self.transport,
            codec: self.codec,
            hooks: ServiceHooks {
                on_codec_error: Some(Box::new(hook)),
            },
        }
    }
}

impl<T: Transport, C: Codec> ServiceConfig<T, C, C::Error> {
    pub fn with_handler<H: Handler<M>, M>(self, handler: H) -> Service<M, H, T, C> {
        Service {
            name: self.name,
            handler,
            transport: self.transport,
            codec: self.codec,
            hooks: self.hooks,
            _message: PhantomData,
        }
    }
}

struct ServiceHooks<C> {
    on_codec_error: Option<Box<dyn Fn(C)>>,
}

impl<C> Default for ServiceHooks<C> {
    fn default() -> Self {
        Self {
            on_codec_error: None,
        }
    }
}
