use std::{convert::Infallible, fmt::Debug};

use futures::StreamExt;

use crate::{Message, MessageStream};

pub trait Transport {
    type Message: Message;
    type Error: Debug;

    fn setup(
        &mut self,
        service_name: &str,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>>;

    fn consume(
        self,
        service_name: &str,
    ) -> impl std::future::Future<Output = Result<MessageStream<Self::Message, Self::Error>, Self::Error>>;
}

pub struct InMemoryMessage {
    payload: Vec<u8>,
}

impl InMemoryMessage {
    pub fn new(payload: Vec<u8>) -> Self {
        Self { payload }
    }
}

impl Message for InMemoryMessage {
    type Error = Infallible;

    fn payload(&self) -> &[u8] {
        &self.payload
    }

    async fn ack(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn nack(&self) -> Result<(), Self::Error> {
        unimplemented!("cannot NACK an in-memory message")
    }
}

pub struct InMemoryTransport {
    rx: tokio::sync::mpsc::Receiver<InMemoryMessage>,
}

impl InMemoryTransport {
    pub fn new() -> (Sender, Self) {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        (Sender(tx), Self { rx })
    }
}

impl Transport for InMemoryTransport {
    type Message = InMemoryMessage;
    type Error = Infallible;

    async fn setup(&mut self, _service_name: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn consume(
        self,
        _service_name: &str,
    ) -> Result<MessageStream<Self::Message, Self::Error>, Self::Error> {
        Ok(tokio_stream::wrappers::ReceiverStream::new(self.rx)
            .map(Ok)
            .boxed())
    }
}

pub struct Sender(tokio::sync::mpsc::Sender<InMemoryMessage>);

impl Sender {
    pub async fn send(&self, message: InMemoryMessage) {
        self.0.send(message).await.unwrap();
    }
}
