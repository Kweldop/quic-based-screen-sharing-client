pub mod convert;
pub mod reassembler;

use wincode::{SchemaRead, SchemaWrite};

#[derive(SchemaWrite, SchemaRead, Debug)]
pub struct VideoChunk {
    pub frame_id: u32,
    pub chunk_index: u16,
    pub total_chunks: u16,
    // pub width: u32,
    // pub height: u32,
    pub is_keyframe: bool,
    pub payload: Vec<u8>,
}
