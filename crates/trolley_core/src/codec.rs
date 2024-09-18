use std::fmt::Debug;

pub trait Codec: Default {
    type Error: Debug;

    fn serialize<M: serde::ser::Serialize>(&self, message: &M) -> Result<Vec<u8>, Self::Error>;
    fn deserialize<M: serde::de::DeserializeOwned>(&self, payload: &[u8])
        -> Result<M, Self::Error>;
}

#[derive(Default)]
pub struct SerdeJsonCodec;

impl Codec for SerdeJsonCodec {
    type Error = serde_json::Error;

    fn serialize<M: serde::ser::Serialize>(&self, message: &M) -> Result<Vec<u8>, Self::Error> {
        serde_json::to_vec(message)
    }

    fn deserialize<M: serde::de::DeserializeOwned>(
        &self,
        payload: &[u8],
    ) -> Result<M, Self::Error> {
        serde_json::from_slice(payload)
    }
}
