use std::collections::HashMap;

use crate::video::VideoChunk;

pub struct Reassembler {
    buffer: HashMap<u32, (u16, Vec<Option<Vec<u8>>>)>,
    last_frame_id: u32,
    got_keyframe: bool,
}

impl Reassembler {
    pub fn new() -> Self {
        Self {
            buffer: HashMap::new(),
            last_frame_id: 0,
            got_keyframe: false,
        }
    }
    pub fn insert(&mut self, chunk: VideoChunk) -> Option<Vec<u8>> {
        let frame_id = chunk.frame_id;
        let chunk_index = chunk.chunk_index;
        let total_chunks = chunk.total_chunks;
        let is_keyframe = chunk.is_keyframe;

        if frame_id < self.last_frame_id {
            return None;
        }

        if !self.got_keyframe {
            if !is_keyframe {
                return None;
            }
            self.got_keyframe = true;
            self.buffer.clear();
        }
        // println!(
        //     "inserting frame:{} chunk:{}/{} keyframe:{}",
        //     frame_id, chunk_index, total_chunks, is_keyframe
        // );

        let entry = self
            .buffer
            .entry(frame_id)
            .or_insert((total_chunks, vec![None; total_chunks as usize]));
        entry.1[chunk_index as usize] = Some(chunk.payload);

        let all_arrived = entry.1.iter().all(|c| c.is_some());

        if all_arrived {
            let (_, chunks) = self.buffer.remove(&frame_id)?;
            let nal_bytes: Vec<u8> = chunks.into_iter().flatten().flatten().collect();

            self.buffer.retain(|id, _| *id > frame_id);
            self.last_frame_id = frame_id;
            if nal_bytes.len() < 4 {
                eprintln!("reassembler: nal too short: {} bytes", nal_bytes.len());
                return None;
            }
            let start_code_ok = nal_bytes.starts_with(&[0x00, 0x00, 0x00, 0x01])
                || nal_bytes.starts_with(&[0x00, 0x00, 0x01]);
            if !start_code_ok {
                eprintln!("reassembler: invalid NAL start code: {:?}", &nal_bytes[..4]);
                return None;
            }
            // println!(
            //     "reassembler: frame {} complete, {} bytes",
            //     frame_id,
            //     nal_bytes.len()
            // );
            return Some(nal_bytes);
        }
        None
    }
}
