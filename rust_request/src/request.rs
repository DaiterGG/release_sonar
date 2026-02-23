use std::{ time::Duration};

use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;
use url::Url;

use crate::service::{NewTracks, REQUEST_STEP, TRACK_PER_ARTIST, TrackItem};
pub const RETRY: usize = 3;

pub struct UserTracks {
    offset: i32,
    client: Client,
    token: String,
}
impl UserTracks {
    pub fn new(cl: &Client, tk: &str, offset: i32) -> Self {
        Self {
            client: cl.clone(),
            offset,
            token: tk.to_owned(),
        }
    }
    pub async fn make_request_with_retry(&self) -> Result<Value, String> {
        for _ in 0..RETRY {
            let res = self.make_request().await;
            if res.is_ok() {
                return res;
            }
        }
        Err("max retries".to_string())
    }
    async fn make_request(&self) -> Result<Value, String> {
        let offset = self.offset;
        let token = &self.token;
        let client = &self.client;
        let mut url =
            Url::parse("https://api.spotify.com/v1/me/tracks").map_err(|e| e.to_string())?;
        url.query_pairs_mut()
            .append_pair("offset", &offset.to_string())
            .append_pair("limit", &REQUEST_STEP.to_string());

        let req = client
            .get(url)
            .bearer_auth(token)
            .build()
            .map_err(|e| e.to_string())?;
        println!("Requesting: {}", offset);

        let res = client.execute(req).await.map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            println!("Status: {}\n", res.status());
            let err = res.text().await.unwrap();
            let err_mes = format!("Failed to get tracks: {err}");
            return Err(err_mes);
        }

        let seed = random_string::generate(10, "1234567890");
        println!("begin {seed}");
        let timeout = sleep(Duration::from_secs(20));
        tokio::pin!(timeout);
        let res = tokio::select! {
            res1 = res.text() => { Some(res1.unwrap())}
            _ = timeout => { println!("timeout reached {seed}"); None}
        };
        if res.is_none() {
            return Err("timeout".to_string());
        }
        let body = res.unwrap();
        println!("end {seed}");
        let body_json: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        Ok(body_json)
    }
}

pub struct NewTracksRequest {
    artist: String,
    client: Client,
    token: String,
}

impl NewTracksRequest {
    pub fn new(cl: &Client, tk: &str, artist: String) -> Self {
        Self {
            artist,
            client: cl.clone(),
            token: tk.to_owned(),
        }
    }

    pub async fn make_request_with_retry(&self) -> Result<Vec<TrackItem>, String> {
        for _ in 0..RETRY {
            let res = self.make_request().await;
            if res.is_ok() {
                return res;
            }
        }
        Err("max retries".to_string())
    }
    async fn make_request(&self) -> Result<Vec<TrackItem>, String> {
        let artist = &self.artist;
        let client = &self.client;
        let mut url = Url::parse(&format!(
            "https://api.spotify.com/v1/artists/{}/albums",
            artist
        ))
        .map_err(|e| e.to_string())?;
        url.query_pairs_mut()
            .append_pair("offset", "0")
            .append_pair("limit", &TRACK_PER_ARTIST.to_string());

        let req = client
            .get(url)
            .bearer_auth(self.token.clone())
            .build()
            .map_err(|e| e.to_string())?;
        println!("Requesting: {}", artist);

        let res = client.execute(req).await.map_err(|e| e.to_string())?;

        println!("Status: {}\n", res.status());

        if res.status().as_u16() == 429 {
            tokio::time::sleep(Duration::from_secs(5)).await;
            return Err("Rate limit, Retry".to_string());
        }
        if !res.status().is_success() {
            return Err("Failed to get releases".to_string());
        }

        let body = res.text().await.map_err(|e| e.to_string())?;
        let tracks: NewTracks = serde_json::from_str(&body).map_err(|e| e.to_string())?;

        let mut tracks_list = Vec::with_capacity(TRACK_PER_ARTIST);
        for item in tracks.items {
            let res = item.parse_tracks();
            if let Ok(track) = res {
                tracks_list.push(track);
            } else {
                print!("Error parsing track of the artist ");
                println!("{}", res.err().unwrap());
                println!("Skipping");
            }
        }
        Ok(tracks_list)
    }
}
