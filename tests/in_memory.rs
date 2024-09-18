use std::sync::Arc;

use tokio::sync::Mutex;
use trolley::{
    codec::SerdeJsonCodec,
    transport::{InMemoryMessage, InMemoryTransport},
    DeliveryContext, DeliveryOutcome, Handler, ServiceConfig,
};

#[derive(Default)]
pub struct TestHandler {
    received: Mutex<Option<TestMessage>>,
}

impl Handler<TestMessage> for TestHandler {
    async fn handle(&self, ctx: DeliveryContext, message: TestMessage) -> DeliveryOutcome {
        *self.received.lock().await = Some(message);
        ctx.shutdown()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TestMessage {
    hello: String,
}

impl From<TestMessage> for InMemoryMessage {
    fn from(message: TestMessage) -> Self {
        let payload = serde_json::to_vec(&message).unwrap();
        Self::new(payload)
    }
}

#[tokio::test]
async fn it_processes_a_message() {
    let handler = Arc::new(TestHandler::default());
    let (tx, transport) = InMemoryTransport::new();

    let svc = ServiceConfig::new("hello")
        .with_transport(transport)
        .with_codec::<SerdeJsonCodec>()
        .with_handler(handler.clone())
        .start();

    tx.send(
        TestMessage {
            hello: "world".into(),
        }
        .into(),
    )
    .await;

    svc.await.unwrap();

    assert!(handler.received.lock().await.as_ref().unwrap().hello == "world");
}
