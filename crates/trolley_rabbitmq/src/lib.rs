use futures::StreamExt;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::{FieldTable, LongString},
};
use trolley_core::{Message, MessageStream, Transport};

pub struct RabbitTransport {
    url: String,
    bindings: Vec<Binding>,
    channel: Option<lapin::Channel>,
}

impl RabbitTransport {
    pub fn new(url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            bindings: Vec::default(),
            channel: None,
        }
    }

    pub fn bind(mut self, exchange: impl ToString, routing_key: impl ToString) -> Self {
        self.bindings.push(Binding {
            exchange: exchange.to_string(),
            routing_key: routing_key.to_string(),
        });
        self
    }
}

struct Binding {
    exchange: String,
    routing_key: String,
}

pub struct RabbitMessage(lapin::message::Delivery);

impl Message for RabbitMessage {
    type Error = lapin::Error;

    fn payload(&self) -> &[u8] {
        &self.0.data
    }

    async fn ack(&self) -> Result<(), Self::Error> {
        self.0.ack(BasicAckOptions::default()).await
    }

    async fn nack(&self) -> Result<(), Self::Error> {
        self.0.nack(BasicNackOptions::default()).await
    }
}

impl Transport for RabbitTransport {
    type Message = RabbitMessage;
    type Error = lapin::Error;

    async fn setup(&mut self, queue: &str) -> Result<(), Self::Error> {
        let conn =
            lapin::Connection::connect(&self.url, lapin::ConnectionProperties::default()).await?;

        let channel = conn.create_channel().await?;

        for binding in &self.bindings {
            let errors_queue = format!("{queue}.errors");
            channel
                .queue_declare(
                    &errors_queue,
                    QueueDeclareOptions {
                        durable: true,
                        ..Default::default()
                    },
                    FieldTable::default(),
                )
                .await?;

            channel
                .queue_bind(
                    &errors_queue,
                    &binding.exchange,
                    &errors_queue,
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await?;

            let mut fields = FieldTable::default();
            fields.insert(
                "x-dead-letter-exchange".into(),
                LongString::from(binding.exchange.clone()).into(),
            );
            fields.insert(
                "x-dead-letter-routing-key".into(),
                LongString::from(errors_queue).into(),
            );

            channel
                .queue_declare(
                    queue,
                    QueueDeclareOptions {
                        durable: true,
                        ..Default::default()
                    },
                    fields,
                )
                .await?;

            channel
                .queue_bind(
                    queue,
                    &binding.exchange,
                    &binding.routing_key,
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await?;
        }

        self.channel = Some(channel);
        Ok(())
    }

    async fn consume(
        self,
        queue: &str,
    ) -> Result<MessageStream<Self::Message, Self::Error>, Self::Error> {
        let channel = self.channel.unwrap();

        Ok(channel
            .basic_consume(
                queue,
                queue,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?
            .map(|delivery| delivery.map(RabbitMessage))
            .boxed())
    }
}
