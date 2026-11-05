use std::{
    io::{self, Read, Seek, SeekFrom},
    mem,
};

use aes::cipher::{KeyIvInit, block_padding::Pkcs7};
use aes::{Aes128, cipher::BlockModeDecrypt};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use m3u8_rs::parse_media_playlist_res;
use rodio::{MixerDeviceSink, Player as RodioPlayer};
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};
use tracing::error;

use crate::https::ApiClient;

mod demux;

pub enum PlayerCommand {
    Pause,
    Play,
    Start(String),
}

pub struct Player {
    rx: Receiver<PlayerCommand>,
    playback: RodioPlayer,
    _device: MixerDeviceSink,
    stream_task: Option<JoinHandle<()>>,
    api: ApiClient,
}

impl Player {
    pub async fn new(api: ApiClient) -> Result<(Self, PlayerHandle)> {
        let (tx, rx) = mpsc::channel::<PlayerCommand>(32);
        let device = rodio::DeviceSinkBuilder::open_default_sink()?;
        let playback = RodioPlayer::connect_new(device.mixer());

        Ok((
            Self {
                rx,
                playback,
                _device: device,
                stream_task: None,
                api,
            },
            PlayerHandle { tx },
        ))
    }

    pub async fn run(mut self) -> Result<()> {
        while let Some(command) = self.rx.recv().await {
            match command {
                PlayerCommand::Play => {
                    self.playback.play();
                }

                PlayerCommand::Pause => {
                    self.playback.pause();
                }

                PlayerCommand::Start(track_id) => {
                    // Abort existing stream function
                    if let Some(task) = self.stream_task.take() {
                        task.abort();
                    }

                    // Stop rodio playback
                    self.playback.stop();

                    let (stream_tx, mut stream_rx) = mpsc::channel::<ChunkRequest>(32);
                    let mut api = self.api.clone();

                    // Start stream function
                    self.stream_task = Some(tokio::spawn(async move {
                        if let Err(err) = stream(track_id, &mut stream_rx, &mut api).await {
                            error!("Streaming failed: {err:#}");
                        }
                    }));

                    let reader = AudioReader {
                        streamer: stream_tx,
                        chunk: None,
                        chunk_num: 0,
                        offset: 0,
                    };

                    let decoder = tokio::task::spawn_blocking(move || {
                        rodio::Decoder::builder()
                            .with_seekable(false)
                            .with_data(reader)
                            .with_hint("aac")
                            .build()
                    })
                    .await??;

                    // Attach new audio and start rodio playback
                    self.playback.append(decoder);
                    self.playback.play();
                }
            }
        }

        // If
        if let Some(task) = self.stream_task.take() {
            task.abort();
        }

        self.playback.stop();

        Ok(())
    }
}

pub struct PlayerHandle {
    tx: Sender<PlayerCommand>,
}

impl PlayerHandle {
    pub async fn play(&self) -> Result<()> {
        Ok(self.tx.send(PlayerCommand::Play).await?)
    }

    pub async fn pause(&self) -> Result<()> {
        Ok(self.tx.send(PlayerCommand::Pause).await?)
    }

    pub async fn start(&self, track_id: String) -> Result<()> {
        Ok(self.tx.send(PlayerCommand::Start(track_id)).await?)
    }
}

enum Chunk {
    Empty,
    Pending(Vec<oneshot::Sender<Bytes>>),
    Ready(Bytes),
}

async fn stream(
    track_id: String,
    rx: &mut Receiver<ChunkRequest>,
    api: &mut ApiClient,
) -> Result<()> {
    // Fetch manifest
    let manifest_bytes = api.fetch_manifest(&track_id).await?;
    // parse manifest
    let manifest = parse_media_playlist_res(&manifest_bytes).unwrap();
    // Get key
    let key_response = api
        .generic_get(&manifest.segments[0].key.clone().unwrap().uri.unwrap())
        .await?
        .bytes()
        .await?;
    let key: [u8; 16] = key_response.as_ref().try_into()?;
    // Cache will be a vec of chunkstate enums that are either empty
    // Pending (holds list of waiting recievers oneshot channels)
    // Ready (hold chunk bytes)
    let mut cache: Vec<Chunk> = (0..manifest.segments.len()).map(|_| Chunk::Empty).collect();

    // DEBUG PREFETCH WHOLE SONG
    for i in 0..manifest.segments.len() {
        chunk_fetch(
            &manifest.segments[i].uri,
            key,
            i,
            api.clone(),
            &mut cache,
            None,
        )
        .await?;
    }

    // Receive ChunkRequest
    while let Some((chunk_pos, tx_responder)) = rx.recv().await {
        if chunk_pos >= cache.len() {
            let _ = tx_responder.send(Bytes::new());
            continue;
        }

        // TODO: Prefetch 2 chunks

        // Check cache for chunk
        match &mut cache[chunk_pos] {
            Chunk::Ready(chunk) => {
                // If its there send through oneshot channel
                let _ = tx_responder.send(chunk.clone());
            }
            Chunk::Pending(clients) => {
                // If pending add oneshot channel to vec
                clients.push(tx_responder);
            }
            Chunk::Empty => {
                // If empty spawn chunk fetch
                chunk_fetch(
                    &manifest.segments[chunk_pos].uri,
                    key,
                    chunk_pos,
                    api.clone(),
                    &mut cache,
                    Some(tx_responder),
                )
                .await?;
            }
        }
    }

    Ok(())
}

async fn chunk_fetch(
    url: &str,
    key: [u8; 16],
    chunk_pos: usize,
    api: ApiClient,
    cache: &mut Vec<Chunk>,
    pending: Option<oneshot::Sender<Bytes>>,
) -> Result<()> {
    // Set chunk to pending
    match &mut cache[chunk_pos] {
        Chunk::Pending(senders) => {
            // Already being downloaded add pending
            if let Some(p) = pending {
                senders.push(p);
            }

            return Ok(());
        }
        Chunk::Ready(bytes) => {
            // already downloaded send data
            if let Some(p) = pending {
                let _ = p.send(bytes.clone());
            }

            return Ok(());
        }
        Chunk::Empty => {
            // Set the task to pending
            let mut senders = Vec::new();
            if let Some(p) = pending {
                senders.push(p);
            }
            cache[chunk_pos] = Chunk::Pending(senders);
        }
    }

    // Download segment
    let s = api.fetch_segment(url).await?;
    // Decrypt
    let iv = (chunk_pos as u128).to_be_bytes();
    let s_decrypted = decrypt_segment(s, &key, &iv).await?;
    // Demux to AAC-LC
    let demuxed = demux::ts_demux(s_decrypted).await?;

    // Send pending oneshot channels if given
    if let Chunk::Pending(senders) =
        mem::replace(&mut cache[chunk_pos], Chunk::Ready(demuxed.clone()))
    {
        for s in senders {
            let _ = s.send(demuxed.clone());
        }
    }

    Ok(())
}

async fn decrypt_segment(segment: Bytes, key: &[u8; 16], iv: &[u8; 16]) -> Result<Bytes> {
    let mut data = segment.to_vec();

    let decrypted = cbc::Decryptor::<Aes128>::new(key.into(), iv.into())
        .decrypt_padded::<Pkcs7>(&mut data)
        .map_err(|_| anyhow!("Invalid AES-128-CBC padding"))?;

    Ok(Bytes::copy_from_slice(decrypted))
}

type ChunkRequest = (usize, oneshot::Sender<Bytes>);

pub struct AudioReader {
    streamer: Sender<ChunkRequest>,
    chunk: Option<Bytes>,
    chunk_num: usize,
    offset: usize,
}

impl AudioReader {
    fn fetch_chunk(&mut self, num: usize) -> Result<()> {
        let (tx, rx) = oneshot::channel();

        self.streamer.blocking_send((num, tx))?;
        let bytes = rx.blocking_recv()?;

        self.chunk_num = num;
        self.offset = 0;
        self.chunk = Some(bytes);

        Ok(())
    }
}

impl Read for AudioReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.chunk.is_none() {
            self.fetch_chunk(self.chunk_num).map_err(io::Error::other)?;
        } else if self.offset >= self.chunk.as_ref().unwrap().len() {
            self.fetch_chunk(self.chunk_num + 1)
                .map_err(io::Error::other)?;
        }

        let chunk = self.chunk.as_ref().unwrap();
        let bytes = &chunk[self.offset..];
        let to_copy = buf.len().min(bytes.len());

        buf[..to_copy].copy_from_slice(&bytes[..to_copy]);
        self.offset += to_copy;

        Ok(to_copy)
    }
}

impl Seek for AudioReader {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Stream is not seekable",
        ))
    }
}
