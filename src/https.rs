use crate::models::{Code, Lounge, Queue, QueueAction, QueueEvents, QueueSong, Session};
use anyhow::{Context, Error, Result};
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::{Value, json};
use tokio::time::{Duration, sleep};
use tracing::{debug, info};

const BASE_URL: &str = "https://api.awa.io";
const JSON_TYPE: &str = "application/json";
const APK_VERSION: &str = "5.20.0";

pub struct ApiClient {
    pub client: Client,
}

impl ApiClient {
    pub fn new(device_id: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSON_TYPE));
        headers.insert(ACCEPT, HeaderValue::from_static(JSON_TYPE));
        headers.insert("X-Device-Id-Type", HeaderValue::from_static("3"));
        headers.insert("X-Device-Id", HeaderValue::from_str(device_id)?);
        headers.insert("X-Platform", HeaderValue::from_str("AWAfy")?);
        headers.insert("X-App-Version", HeaderValue::from_str(APK_VERSION)?);

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to build reqwest client")?;

        Ok(Self { client })
    }

    fn try_attach_auth(&self, req: RequestBuilder, session: Option<&Session>) -> RequestBuilder {
        if let Some(session) = session {
            req.header("X-Access-Token", session.access_token.as_str())
        } else {
            req
        }
    }

    fn request(&self, method: Method, path: &str, session: Option<&Session>) -> RequestBuilder {
        let url = format!("{BASE_URL}{path}");
        let req = self.client.request(method, url);
        self.try_attach_auth(req, session)
    }

    pub async fn get_code(&self) -> Result<Code> {
        let code = self
            .request(Method::POST, "/v4/code", None)
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
                .request(Method::POST, "/v5/login/code", None)
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

    pub async fn refresh_session(&self, session: &mut Session) -> Result<(), Error> {
        let login_data: Value = self
            .request(Method::POST, "/v5/authorize", None)
            .json(&json!({ "refreshToken": session.refresh_token }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let new_session =
            Session::from_login_data(&login_data, &session.device_id, &session.device_name);
        *session = new_session;
        debug!("{}", &session.access_token);
        Ok(())
    }

    pub async fn existing_lounge(&self, session: &Session) -> Result<Option<Lounge>> {
        let response: Value = self
            .request(
                Method::GET,
                "/v6/me/rooms/recommends/sections",
                Some(session),
            )
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

            return Ok(Some(lounge));
        } else {
            return Ok(None);
        };
    }

    pub async fn create_lounge(&self, session: &Session) -> Result<Lounge> {
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
            .request(Method::POST, "/v6/room", Some(session))
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<Lounge>()
            .await?;

        Ok(lounge)
    }

    pub async fn finish_lounge(&self, lounge: &Lounge, session: &Session) -> Result<()> {
        let _response: Value = self
            .request(
                Method::POST,
                format!("/v6/room/{}/archive", lounge.id).as_str(),
                Some(session),
            )
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(())
    }

    pub async fn fetch_queue_events(
        &self,
        lounge: &Lounge,
        session: &Session,
    ) -> Result<QueueEvents> {
        let url: String = format!(
            "/v6/room/{}/queue/events?since={}",
            lounge.id, lounge.cursor
        );

        let response: Value = self
            .request(Method::GET, url.as_str(), Some(session))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let events = QueueEvents {
            events: response["events"]
                .as_array()
                .cloned()
                .unwrap_or(vec![])
                .iter()
                .map(|s| match s["action"].as_u64().unwrap_or(0) {
                    1 => QueueAction::Reset,
                    2 => QueueAction::Add(
                        s["targetContents"][0]["mediaTracks"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|t| QueueSong {
                                name: None,
                                track_id: t["trackId"].as_str().unwrap().to_string(),
                                local_id: t["mediaTrackId"].as_str().unwrap().to_string(),
                                album: None,
                                album_art: None,
                            })
                            .collect(),
                    ),
                    3 => QueueAction::Remove(s["mediaTrackId"].as_str().unwrap().to_string()),
                    4 => QueueAction::Play(s["mediaTrackId"].as_str().unwrap().to_string()),
                    5 => QueueAction::Pause(s["mediaTrackId"].as_str().unwrap().to_string()),
                    7 => QueueAction::Move {
                        id: s["mediaTrackId"].as_str().unwrap().to_string(),
                        dest_id: s["destinationMediaTrackId"].as_str().unwrap().to_string(),
                    },
                    _ => QueueAction::Unknown,
                })
                .collect(),
            cursor: response["next"].as_str().unwrap().to_string(),
            last_id: response["id"].as_str().unwrap().to_string(),
        };

        info!("events: {:#?}", events);

        Ok(events)
    }

    // pub async fn get_streaming_token(&self, session: &Session) -> Result<StreamingToken> {
    //     let streaming_token = self
    //         .request(
    //             Method::GET,
    //             "https://api.awa.io/v4/device/token",
    //             Some(&session),
    //         )
    //         .send()
    //         .await?
    //         .error_for_status()?
    //         .json()
    //         .await?;

    //     Ok(streaming_token)
    // }

    pub async fn fetch_queue(&self, lounge_id: &str, session: &Session) -> Result<Queue> {
        let response: Value = self
            .request(
                Method::GET,
                format!("/v6/room/{}/queue", lounge_id).as_str(),
                Some(session),
            )
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let queue: Queue = if let Some(tracks) =
            response["mediaQueue"]["mediaPlaylist"]["mediaTracks"].as_array()
        {
            Queue {
                tracks: tracks
                    .iter()
                    .map(|song| QueueSong {
                        name: None,
                        track_id: song["trackId"].as_str().unwrap().to_string(),
                        local_id: song["mediaTrackId"].as_str().unwrap().to_string(),
                        album: None,
                        album_art: None,
                    })
                    .collect(),
                cursor: response["next"].as_str().unwrap().to_string(),
                last_id: response["id"].as_str().unwrap().to_string(),
                is_playing: if response["playerState"]["isPlaying"].as_bool() == Some(true) {
                    true
                } else {
                    false
                },
                pos_index: Some(response["mediaPlaylist"]["mediaTracks"].as_u64().unwrap() as usize),
            }
        } else {
            Queue {
                tracks: vec![],
                cursor: response["next"].as_str().unwrap().to_string(),
                last_id: response["id"].as_str().unwrap().to_string(),
                is_playing: false,
                pos_index: None,
            }
        };

        Ok(queue)
    }
}
