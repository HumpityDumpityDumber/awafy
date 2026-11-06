use std::{
    io::{self, Read, Seek, SeekFrom},
    mem,
    sync::Arc,
};

use aes::cipher::{KeyIvInit, block_padding::Pkcs7};
use aes::{Aes128, cipher::BlockModeDecrypt};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use m3u8_rs::parse_media_playlist_res;
use rodio::{MixerDeviceSink, Player as RodioPlayer, source::EmptyCallback};
use tokio::{
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};
use tracing::{debug, error};

use crate::https::ApiClient;

mod demux;

pub enum PlayerCommand {
    Pause,
    Play,
    Start(String),
}

pub enum PlayerEvent {
    SongFinished,
}

pub struct Player {
    command_rx: Receiver<PlayerCommand>,
    event_tx: Sender<PlayerEvent>,
    playback: RodioPlayer,
    _device: MixerDeviceSink,
    stream_task: Option<JoinHandle<()>>,
    api: ApiClient,
}

impl Player {
    pub async fn new(api: ApiClient) -> Result<(Self, PlayerHandle, Receiver<PlayerEvent>)> {
        let (command_tx, command_rx) = mpsc::channel::<PlayerCommand>(32);
        let device = rodio::DeviceSinkBuilder::open_default_sink()?;
        let playback = RodioPlayer::connect_new(device.mixer());

        let (event_tx, event_rx) = mpsc::channel::<PlayerEvent>(32);

        Ok((
            Self {
                command_rx,
                event_tx,
                playback,
                _device: device,
                stream_task: None,
                api,
            },
            PlayerHandle { tx: command_tx },
            event_rx,
        ))
    }

    pub async fn run(mut self) -> Result<()> {
        while let Some(command) = self.command_rx.recv().await {
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

                    let event_tx = self.event_tx.clone();

                    self.playback.append(EmptyCallback::new(Box::new(move || {
                        let _ = event_tx.try_send(PlayerEvent::SongFinished);
                    })));

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
    let cache: Vec<Arc<Mutex<Chunk>>> = (0..manifest.segments.len())
        .map(|_| Arc::new(Mutex::new(Chunk::Empty)))
        .collect();

    // DEBUG PREFETCH WHOLE SONG
    // for i in 0..manifest.segments.len() {
    //     let url = manifest.segments[i].uri.clone();
    //     let api = api.clone();
    //     let chunk = Arc::clone(&cache[i]);

    //     tokio::spawn(async move {
    //         if let Err(err) = chunk_fetch(&url, key, i, api, chunk, None).await {
    //             error!("Chunk fetch failed: {err:#}");
    //         }
    //     });
    // }

    // Receive ChunkRequest
    while let Some((chunk_pos, tx_responder)) = rx.recv().await {
        if chunk_pos >= cache.len() {
            let _ = tx_responder.send(Bytes::new());
            continue;
        }

        // Prefetch next two chunks
        for next in (chunk_pos + 1)..(chunk_pos + 3).min(cache.len()) {
            tokio::spawn(chunk_fetch(
                manifest.segments[next].uri.clone(),
                key,
                next,
                api.clone(),
                Arc::clone(&cache[next]),
                None,
            ));
        }

        // Check cache for chunk
        let mut chunk = cache[chunk_pos].lock().await;

        match &mut *chunk {
            Chunk::Ready(bytes) => {
                // If its there send through oneshot channel
                let _ = tx_responder.send(bytes.clone());
                debug!("chunk already cached");
            }
            Chunk::Pending(clients) => {
                // If pending add oneshot channel to vec
                clients.push(tx_responder);
            }
            Chunk::Empty => {
                debug!("I need to fetch the chunk");
                // If empty spawn chunk fetch
                drop(chunk);

                chunk_fetch(
                    manifest.segments[chunk_pos].uri.clone(),
                    key,
                    chunk_pos,
                    api.clone(),
                    Arc::clone(&cache[chunk_pos]),
                    Some(tx_responder),
                )
                .await?;
            }
        }
    }

    Ok(())
}

async fn chunk_fetch(
    url: String,
    key: [u8; 16],
    chunk_pos: usize,
    api: ApiClient,
    chunk: Arc<Mutex<Chunk>>,
    pending: Option<oneshot::Sender<Bytes>>,
) -> Result<()> {
    // Set chunk to pending
    {
        let mut chunk = chunk.lock().await;

        match &mut *chunk {
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
                *chunk = Chunk::Pending(senders);
            }
        }
    }

    // Download segment
    let s = api.fetch_segment(&url).await?;
    // Decrypt
    let iv = (chunk_pos as u128).to_be_bytes();
    let s_decrypted = decrypt_segment(s, &key, &iv).await?;
    // Demux to AAC-LC
    let demuxed = demux::ts_demux(s_decrypted).await?;

    // Send pending oneshot channels if given
    let mut chunk = chunk.lock().await;

    if let Chunk::Pending(senders) = mem::replace(&mut *chunk, Chunk::Ready(demuxed.clone())) {
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
