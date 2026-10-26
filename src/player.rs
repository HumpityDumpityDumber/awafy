use std::io::{self, Read, Seek};

use anyhow::Result;
use bytes::Bytes;
use rodio::{MixerDeviceSink, Player as RodioPlayer};
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
};
use tracing::error;

use crate::{auth::get_device_id, https::ApiClient};

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
}

impl Player {
    pub async fn new() -> Result<(Self, PlayerHandle)> {
        let (tx, rx) = mpsc::channel::<PlayerCommand>(32);
        let device = rodio::DeviceSinkBuilder::open_default_sink()?;
        let playback = RodioPlayer::connect_new(device.mixer());

        Ok((
            Self {
                rx,
                playback,
                device,
                stream_task: None,
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

                    let client = ApiClient::new(&get_device_id()?)?;

                    self.stream_task = Some(tokio::spawn(async move {
                        if let Err(err) = stream(track_id, stream_rx, client).await {
                            error!("Streaming failed: {err:#}");
                        }
                    }));

                    let cursor = AudioCursor(stream_tx);

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

async fn stream(track_id: String, rx: Receiver<ChunkRequest>, client: ApiClient) -> Result<()> {
    // Get streaming token
    //
    // Fetch manifest
    // Request key
    //
    // Cache will be a vec of chunkstate enums that are either empty
    // Pending (holds list of waiting recievers oneshot channels)
    // Ready (hold chunk bytes)

    // TODO: Chunk fetch async function
    // Takes in u8
    // Download segment
    // Decrypt
    // Demux to raw AAC
    // Wrap in ADTS headers
    // Add chunk to queue
    // If given, send through oneshot channel

    // TODO: Receive ChunkRequest
    // Check cache for chunk
    // If its there send through oneshot channel
    // If empty spawn chunk fetch
    // If pending add oneshot channel to vec

    Ok(())
}

type ChunkRequest = (u8, oneshot::Sender<Bytes>);

pub struct AudioCursor(Sender<ChunkRequest>);

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
