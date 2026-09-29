use std::{sync::Arc, time::Instant};

use dashmap::DashMap;
use openh264::{decoder::Decoder, formats::YUVSource};
use quinn::Connection;

use wincode::{SchemaRead, SchemaWrite};

use crate::{
    apperror::AppError::{self, XCustomError},
    audio::{AudioChunk, audio_client::CpalClient, opus_decoder::OpusDecoder},
    mdns_browser::MdnsBrowser,
    quic_client::Client,
    storage::{KnownHost, Store, StoreValue},
    video::{VideoChunk, convert::Frame, reassembler::Reassembler},
};

mod apperror;
mod audio;
mod mdns_browser;
mod otp_challenge;
mod quic_client;
mod storage;
mod video;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, SchemaRead, SchemaWrite)]
pub enum MediaPacket {
    VideoChunk(VideoChunk),
    AudioChunk(AudioChunk),
}

const BUFFER_SIZE: usize = 480;

#[tokio::main]
async fn main() -> AppResult<()> {
    // 1. shared state
    let store = Store::new(DashMap::new());
    let (video_tx, video_rx) = tokio::sync::mpsc::channel::<VideoChunk>(512);
    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel::<AudioChunk>(4);
    let mut browser = MdnsBrowser::new()?;
    let rec = browser.browse()?;

    browser.monitor_events(rec, &store)?;

    // 3. wait for at least one host to be discovered
    //    before attempting connection
    println!("Scanning for hosts...");
    let host = wait_for_host(&store).await?;
    println!("Found host: {}", host.hostname);

    // 4. connect
    let client = Arc::new(Client::new(&store)?);
    let client_accept = Arc::clone(&client);
    let connection = Arc::new(client_accept.connect(&store).await?);
    println!("waiting to accept stream");
    let otp_connection = Arc::clone(&connection);
    let otp_handle = tokio::task::spawn(async move {
        let (mut send, mut recv) = match otp_connection.accept_bi().await {
            Ok(bi) => bi,
            Err(e) => {
                eprintln!("{}", e);
                return None;
            }
        };
        match otp_challenge::otp_challenge(&mut recv, &mut send).await {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("{}", e);
                None
            }
        }
    });
    let (width, height, sample_rate, channels) = otp_handle
        .await?
        .ok_or(XCustomError("OTP challenge failed"))?;
    println!("Waiting for video stream");
    let display_connection = Arc::clone(&connection);
    // 5. keep process alive
    tokio::spawn(async move {
        if let Err(e) = receive_datagram(display_connection, video_tx, audio_tx).await {
            eprintln!("{}", e);
            return;
        };
    });
    tokio::select! {
        m=receive_video(video_rx,width,height)=>m?,
        m=receive_audio(audio_rx,sample_rate,channels)=>m?
    }

    println!("Disconnecting...");
    client.endpoint.close(0u32.into(), b"closed");
    Ok(())
}

// polls dashmap until a host appears or timeout
async fn wait_for_host(store: &Store) -> AppResult<KnownHost> {
    let timeout = tokio::time::Duration::from_secs(10);
    let interval = tokio::time::Duration::from_millis(500);

    let start = tokio::time::Instant::now();
    loop {
        // look for any key starting with "host:"
        let found = store.iter().find(|e| e.key().contains("host"));

        if let Some(entry) = found {
            if let StoreValue::Host(host) = entry.value().clone() {
                return Ok(host);
            }
        }

        if start.elapsed() > timeout {
            return Err(XCustomError("No host found within 10 seconds"));
        }

        tokio::time::sleep(interval).await;
    }
}

async fn receive_datagram(
    connection: Arc<Connection>,
    video_tx: tokio::sync::mpsc::Sender<VideoChunk>,
    audio_tx: tokio::sync::mpsc::Sender<AudioChunk>,
) -> AppResult<()> {
    while let Ok(datagram) = connection.read_datagram().await {
        match wincode::deserialize::<MediaPacket>(&datagram)? {
            MediaPacket::VideoChunk(video_chunk) => {
                if let Err(e) = video_tx.send(video_chunk).await {
                    eprintln!("video:{}", e);
                    continue;
                };
            }
            MediaPacket::AudioChunk(audio_chunk) => {
                println!("captured audio chunk");
                if let Err(e) = audio_tx.send(audio_chunk).await {
                    eprintln!("audio:{}", e);
                    continue;
                }
            }
        }
    }

    Ok(())
}

async fn receive_video(
    mut video_rx: tokio::sync::mpsc::Receiver<VideoChunk>,
    width: u32,
    height: u32,
) -> AppResult<()> {
    let t0 = Instant::now();
    let (frame_tx, mut frame_rx) = tokio::sync::mpsc::channel::<Frame>(2);

    tokio::spawn(async move {
        let mut reassembler = Reassembler::new();
        let mut decoder = match Decoder::new() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("decoder init: {}", e);
                return;
            }
        };
        let framesize = width as usize * height as usize * 4;
        let mut rgba_buf = vec![0u8; framesize];
        while let Some(chunk) = video_rx.recv().await {
            let nal_bytes = match reassembler.insert(chunk) {
                Some(n) => n,
                None => continue,
            };

            // println!(
            //     "nal complete: {} bytes at {}ms",
            //     nal_bytes.len(),
            //     t0.elapsed().as_millis()
            // );
            let t_dec = std::time::Instant::now();
            match decoder.decode(&nal_bytes) {
                Ok(Some(yuv)) => {
                    //  println!("decode took: {}ms", t_dec.elapsed().as_millis());

                    let (w, h) = yuv.dimensions();

                    // resize if first frame dimensions differ
                    let needed = yuv.rgba8_len();
                    if rgba_buf.len() != needed {
                        rgba_buf.resize(needed, 0);
                    }

                    yuv.write_rgba8(&mut rgba_buf);

                    let _ = frame_tx.try_send(Frame {
                        width: w,
                        height: h,
                        rgba: rgba_buf.clone(),
                    });

                    // println!("frame sent at {}ms", t0.elapsed().as_millis());
                }
                Ok(None) => {
                    println!("decoder buffering");
                }
                Err(e) => {
                    eprintln!("decode error: {}", e);
                }
            }
        }
    });

    while let Some(_frame) = frame_rx.recv().await {
        //println!("presenting at {}ms", t0.elapsed().as_millis());
    }

    Ok(())
}
async fn receive_audio(
    mut audio_rx: tokio::sync::mpsc::Receiver<AudioChunk>,
    sample_rate: u32,
    channels: u8,
) -> AppResult<()> {
    let t0 = Instant::now();
    let mut decoder = OpusDecoder::new(sample_rate, channels)?;
    let mut cpal_client = CpalClient::new(sample_rate)?;
    let _stream = cpal_client.stream()?; // keep alive for entire session
    let mut last_sequence = 0u32;

    while let Some(chunk) = audio_rx.recv().await {
        println!(
            "recieved audio sequence: {} at {}",
            chunk.sequence,
            t0.elapsed().as_millis()
        );
        if chunk.sequence > last_sequence + 1 {
            println!(
                "audio gap: {} packets dropped",
                chunk.sequence - last_sequence - 1
            );
        }
        last_sequence = chunk.sequence;

        let decoded = match decoder.decode(&chunk.payload) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("decode error: {}", e);
                continue; // skip bad packet — do not stop stream
            }
        };

        println!(
            "decoded audio sequence: {} at {}",
            chunk.sequence,
            t0.elapsed().as_millis()
        );

        cpal_client.push_samples(&decoded);
    }
    println!("receive_audio ended");
    Ok(())
}
fn _rgba_to_minifb(rgba: Vec<u8>) -> Vec<u32> {
    rgba.chunks(4)
        .map(|p| {
            let r = p[0] as u32;
            let g = p[1] as u32;
            let b = p[2] as u32;
            (r << 16) | (g << 8) | b
        })
        .collect()
}
