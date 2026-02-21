use chrono::NaiveDate;
use reqwest::{self, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};
use tokio::{sync::Semaphore, task::JoinSet, time::sleep};
use url::Url;

use crate::{
    PORT,
    request::{self, NewTracksRequest, UserTracks},
};

pub const CHARSET_STATE: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
pub static CLIENT_DATA: LazyLock<IdSecret> = LazyLock::new(|| {
    let txt = String::from_utf8_lossy(include_bytes!("conf.json"));
    serde_json::from_str::<IdSecret>(&txt).unwrap()
});
pub const REQUEST_STEP: usize = 50;
pub const PRINT_NUM_OF_TRACKS: usize = 20;
pub const TRACK_PER_ARTIST: usize = 3;

#[derive(Serialize, Deserialize)]
pub struct TockenResponse {
    access_token: String,
}
#[derive(Serialize, Deserialize)]
pub struct IdSecret {
    pub client_id: String,
    pub client_secret: String,
}
#[derive(Deserialize)]
struct Res {
    list: String,
}
#[derive(Serialize, Deserialize)]
pub struct NewTracks {
    pub items: Vec<Track>,
}
#[derive(Serialize, Deserialize)]
pub struct Track {
    id: String,
    name: String,
    release_date: String,
    release_date_precision: String,
    artists: Vec<Artist>,

    #[serde(rename = "type")]
    track_type: String,
}

impl Track {
    pub fn parse_tracks(self) -> Result<TrackItem, String> {
        let track_type = self.track_type;

        let id = self.id;
        let mut uri = String::new();
        uri.push_str(&format!("open.spotify.com/{track_type}/{id}"));
        let precision = self.release_date_precision;
        let fmt = "%Y-%m-%d";
        let mut date = self.release_date.clone();
        if precision == "year" {
            date.push_str("-01-01");
        }
        let date = NaiveDate::parse_from_str(&date, fmt)
            .map_err(|e| format!("fmt = {}, {}, {:?}", fmt, e, e.kind()))?;

        let name = self.name;

        let mut artists = String::new();
        for artist in self.artists {
            artists.push_str(&artist.name);
            artists.push_str(", ");
        }
        let item = TrackItem {
            name,
            // track_num,
            // uri,
            date,
            artists,
        };
        Ok(item)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Artist {
    name: String,
}
pub struct TrackItem {
    name: String,
    // track_num: TracksNum,
    // uri: String,
    date: NaiveDate,
    artists: String,
}

pub async fn new_releases_list(min_tracks: i32, code: String) -> Result<String, String> {
    let client = reqwest::Client::new();

    let redir_uri = format!("http://127.0.0.1:{}/logincallback", PORT);

    let mut form = HashMap::new();
    form.insert("code".to_string(), code.to_string());
    form.insert("redirect_uri".to_string(), redir_uri.to_string());
    form.insert("grant_type".to_string(), "authorization_code".to_string());

    println!("{}", CLIENT_DATA.client_id);
    println!("{}", CLIENT_DATA.client_secret);
    let req = client
        .post("https://accounts.spotify.com/api/token")
        .basic_auth(
            CLIENT_DATA.client_id.clone(),
            Some(CLIENT_DATA.client_secret.clone()),
        )
        .form(&form)
        .build()
        .map_err(|e| e.to_string())?;

    println!("Requesting token");

    let res = client.execute(req).await.unwrap();

    println!("Status: {}\n", res.status());

    if !res.status().is_success() {
        println!("Full response: {:?}\n", res);
        return Err("error in console".to_string());
    }

    let body = res.text().await.unwrap();
    let body_json: TockenResponse = serde_json::from_str(&body).unwrap();
    let token = body_json.access_token.to_string();
    println!("{}", token);

    let all_artists = get_all_artists(&token, &client).await?;

    let mut filtered_artists = Vec::with_capacity(all_artists.len());
    for (k, v) in all_artists.into_iter() {
        if v >= min_tracks {
            filtered_artists.push(k);
        }
    }
    println!("filtered artist: {}", filtered_artists.len());

    let mut jset: JoinSet<_> = Default::default();

    let mut tracks = Vec::with_capacity(filtered_artists.len() * TRACK_PER_ARTIST);
    let rate_limit = Arc::new(Semaphore::new(3));
    for artist in filtered_artists {
        let request = NewTracksRequest::new(&client, &token, artist);
        let limit = rate_limit
            .clone()
            .acquire_owned()
            .await
            .expect("was closed");
        println!("{}", rate_limit.available_permits());
        jset.spawn(async move {
            let res = request
                .make_request_with_retry()
                .await
                .inspect_err(|e| println!("warning: {e}"))
                .ok();
            drop(limit);
            res
        });
    }
    println!("joining final list :{}", jset.len());
    let results = jset.join_all().await;
    println!("joined final list, {}", results.len());
    for mut item in results.into_iter().flatten() {
        tracks.append(&mut item);
    }

    tracks.sort_unstable_by(|a, b| b.date.cmp(&a.date));

    // NOTE: encode result
    let mut result = "".to_string();
    let limit = usize::min(PRINT_NUM_OF_TRACKS, tracks.len());
    for (i, track) in tracks.iter().enumerate().take(limit) {
        let res = format!(
            "{}. {} - {}, {}\n",
            i + 1,
            track.artists,
            track.name,
            track.date
        );
        result.push_str(&res);
    }
    Ok(result.to_string())
}

type Collect = Arc<Mutex<HashMap<String, i32>>>;
async fn get_all_artists(token: &str, client: &Client) -> Result<HashMap<String, i32>, String> {
    let test_request = UserTracks::new(client, token, 0);
    let res = test_request
        .make_request_with_retry()
        .await
        .expect("failed to fetch total track number");
    let total = res.get("total").unwrap().as_i64().unwrap() as usize;
    println!("total tracks in the playlist: {}", total);

    let mut jset: JoinSet<_> = Default::default();
    let all_artists = Arc::new(Mutex::new(HashMap::new()));

    for offset in (0..total).step_by(REQUEST_STEP) {
        let user_tracks = UserTracks::new(client, token, offset as i32);
        let all = all_artists.clone();
        jset.spawn(async move {
            let res = user_tracks.make_request_with_retry().await;
            if let Ok(val) = res {
                get_artist(val, all);
            }
            println!("joined: {offset}");
        });
    }
    println!("joining");
    jset.join_all().await;
    let result = Arc::try_unwrap(all_artists)
        .expect("joined")
        .into_inner()
        .expect("poisoned");
    println!("joined, {}", result.len());
    Ok(result)
}

fn get_artist(body_json: Value, all: Collect) {
    let items = body_json.get("items").unwrap();
    let items = items.as_array().expect("parsing error: not array");
    let mut all = all.lock().unwrap();
    for item in items {
        let artists = item.get("track").and_then(|e| e.get("artists")).unwrap();
        if let Some(artists) = artists.as_array() {
            for artist in artists {
                let id = artist.get("id").unwrap().as_str().unwrap();
                println!("{:?}", id);
                let mut count = all.get(id).unwrap_or(&0).to_owned();
                count += 1;
                all.insert(id.to_string(), count);
            }
        }
    }
}
