use trolley::{
    codec::SerdeJsonCodec, rabbitmq::RabbitTransport, DeliveryOutcome, Handler, ServiceConfig,
};

struct LogMessage;

impl Handler<String> for LogMessage {
    async fn handle(&self, ctx: trolley::DeliveryContext, message: String) -> DeliveryOutcome {
        if message == "die" {
            eprintln!("refusing message: die");
            return ctx.nack();
        }

        eprintln!("received a message: {message}");
        ctx.ack()
    }
}

#[tokio::main]
async fn main() {
    ServiceConfig::new("rabbitmq-example")
        .with_codec::<SerdeJsonCodec>()
        .on_codec_error(|err| {
            eprintln!("JSON error: {err:?}");
        })
        .with_transport(
            RabbitTransport::new("amqp://guest:guest@localhost:5672").bind("amq.topic", "hello"),
        )
        .with_handler(LogMessage)
        .start()
        .await
        .unwrap();
}
