use crate::models::{Code, Lounge, Queue, QueueEvents, Session, Song};
use anyhow::{Context, Error, Result};
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::{Value, json};
use tokio::time::{Duration, sleep};

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
                println!("Code not authorized! Trying again in 3 seconds...");
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

        let new_session = Session::from_login_data(&login_data, &session.device_id);
        *session = new_session;
        dbg!(&session.access_token);
        Ok(())
    }

    pub async fn create_lounge(&self, session: &Session) -> Result<Lounge> {
        let name = format!(
            "{} {}",
            random_word::get(random_word::Lang::En),
            random_word::get(random_word::Lang::Ja)
        );

        let payload = json!({
            "name": name,
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
        cursor: Option<u64>,
    ) -> Result<QueueEvents> {
        let url: String = if cursor.is_some() {
            format!(
                "/v6/room/{}/queue/events?since={}",
                lounge.id,
                cursor.unwrap()
            )
        } else {
            format!("/v6/room/{}/queue/events", lounge.id)
        };

        let events: QueueEvents = self
            .request(Method::GET, url.as_str(), Some(session))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(events)
    }

    pub async fn get_streaming_token(&self, session: &Session) -> Result<StreamingToken> {
        let response = self
            .request(
                Method::GET,
                "https://api.awa.io/v4/device/token",
                Some(&session),
            )
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
    }

    pub async fn update_queue(&self, lounge: &Lounge, session: &Session) -> Result<Queue> {
        let response: Value = self
            .request(
                Method::GET,
                format!("/v6/room/{}/queue", lounge.id).as_str(),
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
                    .filter_map(|song| {
                        Some(Song {
                            name: "placeholder name".to_string(),
                            id: song.get("trackId")?.as_str()?.to_string(),
                            album: "placeholder album".to_string(),
                            album_art: "placeholder album art".to_string(),
                        })
                    })
                    .collect(),
                cursor: response["next"].to_string(),
                last_id: response["id"].to_string(),
            }
        } else {
            Queue {
                tracks: vec![],
                cursor: response["next"].to_string(),
                last_id: response["id"].to_string(),
            }
        };

        Ok(queue)
    }
}
