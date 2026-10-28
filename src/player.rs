use std::io::{self, Read, Seek};

use anyhow::Result;
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

enum PlayerCommand {
    Pause,
    Play,
    Start(String),
}

pub struct Player {
    rx: Receiver<PlayerCommand>,
    playback: RodioPlayer,
    device: MixerDeviceSink,
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
                device,
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
                    if let Some(task) = self.stream_task.take() {
                        task.abort();
                    }

                    self.playback.stop();

                    let (stream_tx, stream_rx) = mpsc::channel::<ChunkRequest>(32);
                    let api = self.api.clone();

                    self.stream_task = Some(tokio::spawn(async move {
                        if let Err(err) = stream(track_id, stream_rx, api).await {
                            error!("Streaming failed: {err:#}");
                        }
                    }));

                    let cursor = AudioCursor {
                        streamer: stream_tx,
                        chunk: 0,
                        offset: 0,
                    };

                    let decoder = tokio::task::spawn_blocking(move || {
                        rodio::Decoder::builder()
                            .with_data(cursor)
                            .with_hint("aac")
                            .build()
                    })
                    .await??;

                    self.playback.append(decoder);
                    self.playback.play();
                }
            }
        }

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
    Pending(Vec<oneshot::Sender<Vec<u8>>>),
    Ready(Bytes),
}

impl Clone for Chunk {
    fn clone(&self) -> Self {
        match self {
            Chunk::Empty => Chunk::Empty,
            Chunk::Ready(_) => panic!("Can't Clone non-Empty Chunks"),
            Chunk::Pending(_) => panic!("Can't Clone non-Empty Chunks"),
        }
    }
}

async fn stream(track_id: String, rx: Receiver<ChunkRequest>, api: ApiClient) -> Result<()> {
    // Fetch manifest
    let manifest_bytes = api.fetch_manifest(&track_id).await?;
    // parse manifest
    let manifest = parse_media_playlist_res(&manifest_bytes).unwrap();
    // Cache will be a vec of chunkstate enums that are either empty
    // Pending (holds list of waiting recievers oneshot channels)
    // Ready (hold chunk bytes)
    let mut cache = vec![Chunk::Empty; manifest.segments.len()];

    // TODO: Receive ChunkRequest
    // Check cache for chunk
    // If its there send through oneshot channel
    // If empty spawn chunk fetch
    // If pending add oneshot channel to vec

    Ok(())
}

async fn chunk_fetch(
    url: &str,
    chunk_pos: u8,
    api: ApiClient,
    cache: &Vec<Chunk>,
    pending: Option<oneshot::Sender<Vec<u8>>>,
) {
    // TODO: Set chunk to pending
    // open cache[chunk_pos] and set to pending()
    // Download segment
    let s = api.fetch_segment(url);
    // TODO: Decrypt
    // aes-cbc things
    let s_decrypted = todo!();
    // TODO: Demux to AAC-LC
    ts_demux(s_decrypted);
    // TODO: Add chunk to cache
    // at cache[chunk_pos] do the thing
    // If given, send through oneshot channel
    todo!()
}

async fn ts_demux(data: Vec<u8>) -> Vec<u8> {
    todo!()
}

type ChunkRequest = (u8, oneshot::Sender<Bytes>);

pub struct AudioCursor {
    streamer: Sender<ChunkRequest>,
    chunk: u8,
    offset: usize,
}

impl Read for AudioCursor {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        // TODO: IDFK HOW TO DO THIS
    }
}

impl Seek for AudioCursor {
    fn seek() {
        // TODO: IDFK HOW TO DO THIS EITHER
    }
}
