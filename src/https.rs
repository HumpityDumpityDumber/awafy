use std::ops::Deref;
use std::sync::Arc;

use crate::auth::{self, get_device_id};
use crate::models::{
    Album, Artist, Code, Lounge, Queue, QueueAction, QueueEvents, QueueSong, Session, Song,
    StreamingToken,
};
use anyhow::{Context, Error, Result};
use bytes::Bytes;
use futures_util::lock::Mutex;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde_json::{Value, json};
use tokio::time::{Duration, sleep};
use tracing::debug;

const JSON_TYPE: &str = "application/json";
const APK_VERSION: &str = "5.20.0";

#[derive(Clone)]
pub struct AuthedClient {
    client: ApiClient,
    pub session: Arc<Mutex<Session>>,
    pub streaming_token: Option<StreamingToken>,
}

#[derive(Clone)]
pub struct ApiClient {
    pub client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(abema: bool) -> Result<Self> {
        let base_url: String = if abema {
            "https://api-abema.awa.io"
        } else {
            "https://api.awa.io"
        }
        .to_string();
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSON_TYPE));
        headers.insert(ACCEPT, HeaderValue::from_static(JSON_TYPE));
        headers.insert("X-Device-Id-Type", HeaderValue::from_static("3"));
        headers.insert("X-Device-Id", HeaderValue::from_str(&get_device_id()?)?);
        headers.insert("X-Platform", HeaderValue::from_str("AWAfy")?);
        headers.insert("X-App-Version", HeaderValue::from_str(APK_VERSION)?);

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build reqwest client")?;

        Ok(Self { base_url, client })
    }
    pub async fn get_code(&self) -> Result<Code> {
        let code = self
            .request(true, Method::POST, "/v4/code")
            .send()
            .await?
            .error_for_status()?
            .json::<Code>()
            .await?;

        Ok(code)
    }
    pub async fn poll_login(&self, code: &Code) -> Result<Value> {
        loop {
            let response = self
                .request(true, Method::POST, "/v5/login/code")
                .json(code)
                .send()
                .await?;

            if response.status() == StatusCode::UNAUTHORIZED {
                debug!("Code not authorized! Trying again in 3 seconds...");
                sleep(Duration::from_secs(3)).await;
                continue;
            }

            let response = response.error_for_status()?;
            let login_data = response.json::<Value>().await?;
            return Ok(login_data);
        }
    }
    pub async fn generic_get(&self, url: &str) -> Result<Response> {
        let response = self
            .request(false, Method::GET, url)
            .send()
            .await?
            .error_for_status()?;

        Ok(response)
    }
    pub async fn fetch_segment(&self, url: &str) -> Result<Bytes> {
        let response = self
            .request(false, Method::GET, url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        Ok(response)
    }
    fn request(&self, use_base: bool, method: Method, path: &str) -> RequestBuilder {
        let base = if use_base { &self.base_url } else { "" };
        let url = format!("{base}{path}");
        self.client.request(method, url)
    }
    pub fn upgrade(self) -> Result<AuthedClient> {
        let session = auth::new_session().context("Failed to get credentials")?;

        Ok(AuthedClient {
            client: self,
            session: Arc::new(Mutex::new(session)),
            streaming_token: None,
        })
    }
}

impl AuthedClient {
    async fn request(&self, use_base: bool, method: Method, path: &str) -> RequestBuilder {
        let token = self.session.lock().await.access_token.clone();

        self.client
            .request(use_base, method, path)
            .header("X-Access-Token", token)
    }

    pub async fn refresh_session(&mut self) -> Result<(), Error> {
        let (refresh_token, device_id, device_name) = {
            let session = self.session.lock().await;
            (
                session.refresh_token.clone(),
                session.device_id.clone(),
                session.device_name.clone(),
            )
        };

        let login_data: Value = self
            .request(true, Method::POST, "/v5/authorize")
            .await
            .json(&json!({ "refreshToken": refresh_token }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let new_session = Session::from_login_data(&login_data, &device_id, &device_name);

        *self.session.lock().await = new_session;
        Ok(())
    }

    pub async fn existing_lounge(&self) -> Result<Option<Lounge>> {
        let response: Value = self
            .request(true, Method::GET, "/v6/me/rooms/recommends/sections")
            .await
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if let Some(value) = response["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"].as_str() == Some("owner_section"))
        {
            let lounge: Lounge = serde_json::from_value::<Lounge>(
                value["rooms"].as_array().unwrap().first().unwrap()["room"].clone(),
            )?;

            Ok(Some(lounge))
        } else {
            Ok(None)
        }
    }

    pub async fn fetch_metadata(&self, tracks: Vec<String>) -> Result<Vec<Song>> {
        let response: Value = self
            .request(
                true,
                Method::GET,
                &format!("/v5/tracks/{}", tracks.join(",")),
            )
            .await
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let metadata = &response["dataSet"];
        let tracks = metadata["tracks"].as_array().unwrap();
        let albums = metadata["albums"].as_array().unwrap();
        let artists = metadata["artists"].as_array().unwrap();

        let songs = response["list"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, _)| Song {
                name: tracks[i]["name"].as_str().unwrap().to_string(),
                track_id: tracks[i]["id"].as_str().unwrap().to_string(),
                artist: Artist {
                    name: artists[i]["name"].as_str().unwrap().to_string(),
                    id: artists[i]["id"].as_str().unwrap().to_string(),
                },
                album: Album {
                    name: albums[i]["name"].as_str().unwrap().to_string(),
                    id: albums[i]["id"].as_str().unwrap().to_string(),
                },
            })
            .collect();

        Ok(songs)
    }

    pub async fn create_lounge(&self) -> Result<Lounge> {
        let session = &self.session.lock().await.clone();

        let payload = json!({
            "name": session.device_name,
            "description": "",
            "topicText": "",
            "allowGifting": false,
            "allowLiveAudio": false,
            "disabledAutoFillTrack": false,
            "coOwnerUsers": [],
            "thumbType": 2,
            "backgroundType": 3
        });

        let lounge = self
            .request(true, Method::POST, "/v6/room")
            .await
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<Lounge>()
            .await?;

        Ok(lounge)
    }

    pub async fn finish_lounge(&self, lounge: &Lounge) -> Result<()> {
        self.request(
            true,
            Method::POST,
            format!("/v6/room/{}/archive", lounge.id).as_str(),
        )
        .await
        .send()
        .await?
        .error_for_status()?;

        Ok(())
    }

    pub async fn fetch_manifest(&mut self, track_id: &str) -> Result<Vec<u8>> {
        self.streaming_token = Some(self.get_streaming_token().await?);

        let streaming_token = self.streaming_token.clone().unwrap(); // FIX: replace with an if let

        let response = self
            .request(
                true,
                Method::GET,
                &format!(
                    "/v4/tracks/{}/audio.m3u8?bitRate=320&audioType=0&accessToken={}",
                    track_id, streaming_token.token
                ),
            )
            .await
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        Ok(response.to_vec())
    }

    pub async fn fetch_queue_events(&self, lounge: &Lounge) -> Result<QueueEvents> {
        let url: String = format!(
            "/v6/room/{}/queue/events?since={}",
            lounge.id, lounge.cursor
        );

        let response: Value = self
            .request(true, Method::GET, url.as_str())
            .await
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let mut events = Vec::new();

        for s in response["events"].as_array().cloned().unwrap_or_default() {
            let event = match s["action"].as_u64().unwrap_or(0) {
                1 => QueueAction::Reset,
                2 => {
                    let song_info = s["targetContents"][0]["mediaTracks"].as_array().unwrap();

                    let song_ids: Vec<(String, String)> = song_info
                        .iter()
                        .map(|t| {
                            (
                                t["trackId"].as_str().unwrap().to_string(),
                                t["mediaTrackId"].as_str().unwrap().to_string(),
                            )
                        })
                        .collect();

                    let songs = self
                        .fetch_metadata(song_ids.iter().map(|t| t.0.clone()).collect())
                        .await?;

                    QueueAction::Add(
                        song_ids
                            .iter()
                            .map(|(t_id, m_id)| QueueSong {
                                song: songs.iter().find(|t| t_id == &t.track_id).unwrap().clone(),
                                local_id: m_id.clone(),
                            })
                            .collect(),
                    )
                }
                3 => QueueAction::Remove(s["mediaTrackId"].as_str().unwrap().to_string()),
                4 => QueueAction::Play(s["mediaTrackId"].as_str().unwrap().to_string()),
                5 => QueueAction::Pause(s["mediaTrackId"].as_str().unwrap().to_string()),
                7 => QueueAction::Move {
                    id: s["mediaTrackId"].as_str().unwrap().to_string(),
                    dest_id: s["destinationMediaTrackId"].as_str().unwrap().to_string(),
                },
                _ => QueueAction::Unknown,
            };

            events.push(event);
        }

        let events = QueueEvents {
            events,
            cursor: response["next"].as_str().unwrap().to_string(),
            last_id: response["id"].as_str().unwrap().to_string(),
        };

        debug!("events: {:#?}", events);

        Ok(events)
    }

    async fn get_streaming_token(&self) -> Result<StreamingToken> {
        let streaming_token = self
            .request(true, Method::GET, "/v4/device/token")
            .await
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(streaming_token)
    }

    pub async fn fetch_queue(&self, lounge_id: &str) -> Result<Queue> {
        let response: Value = self
            .request(
                true,
                Method::GET,
                format!("/v6/room/{}/queue", lounge_id).as_str(),
            )
            .await
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let queue: Queue = if let Some(tracks) =
            response["mediaQueue"]["mediaPlaylist"]["mediaTracks"].as_array()
        {
            let songs = self
                .fetch_metadata(
                    tracks
                        .iter()
                        .map(|t| t["trackId"].clone().as_str().unwrap().to_string())
                        .collect(),
                )
                .await?;

            Queue {
                tracks: tracks
                    .iter()
                    .map(|song| QueueSong {
                        song: songs
                            .iter()
                            .find(|t| song["trackId"].as_str().unwrap() == t.track_id)
                            .unwrap()
                            .clone(),
                        local_id: song["mediaTrackId"].as_str().unwrap().to_string(),
                    })
                    .collect(),
                cursor: response["next"].as_str().unwrap().to_string(),
                last_id: response["id"].as_str().unwrap().to_string(),
                is_playing: response["playerState"]["isPlaying"].as_bool() == Some(true),
                pos_index: response["mediaQueue"]["mediaPlaylist"]
                    .get("currentMediaTrackIndex")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
            }
        } else {
            Queue {
                tracks: vec![],
                cursor: response["next"].as_str().unwrap().to_string(),
                last_id: response["id"].as_str().unwrap().to_string(),
                is_playing: false,
                pos_index: 0,
            }
        };

        Ok(queue)
    }
}

impl Deref for AuthedClient {
    type Target = ApiClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}
