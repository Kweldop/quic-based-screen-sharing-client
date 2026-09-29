use wincode::{SchemaRead, SchemaWrite};

pub mod audio_client;
pub mod opus_decoder;
pub mod resampler;

#[derive(Debug, SchemaWrite, SchemaRead)]
pub struct AudioChunk {
    pub sequence: u32,
    pub payload: Vec<u8>,
}
