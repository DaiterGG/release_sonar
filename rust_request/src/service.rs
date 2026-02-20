use chrono::NaiveDate;
use random_string::generate;
use reqwest::{self, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::{task::JoinSet, time::sleep};
use url::Url;

use crate::PORT;

pub const CHARSET_STATE: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
pub static CLIENT_DATA: LazyLock<IdSecret> = LazyLock::new(|| {
    let txt = String::from_utf8_lossy(include_bytes!("conf.json"));
    serde_json::from_str::<IdSecret>(&txt).unwrap()
});
pub const REQUEST_STEP: usize = 50;
pub const PRINT_NUM_OF_TRACKS: usize = 20;

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
    items: Vec<Track>,
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

    let res = get_users_tracks(&client, &token, 0).await.unwrap();
    let total = res.get("total").unwrap().as_i64().unwrap() as usize;
    println!("total tracks in the playlist: {}", total);

    let mut all_artists: HashMap<String, i32> = HashMap::new();
    let mut handles: JoinSet<_> = Default::default();

    for offset in (0..total).step_by(REQUEST_STEP) {
        let offset = offset as i32;
        handles.spawn(parallel_req(offset, token.clone(), client.clone()));
    }
    println!("joining");
    let res = {
        let mut this = handles;
        async move {
            let mut output = Vec::with_capacity(this.len());
            let mut count = 0;
            while let Some(res) = this.join_next().await {
                match res {
                    Ok(t) => output.push(t),
                    Err(err) => panic!("{err}"),
                }
                count += 1;
                println!("joined: {}, left: {}", count, this.len());
            }
            output
        }
    }
    .await;
    println!("joined, {}", res.len());

    for i in res {
        match i {
            Ok(hash) => all_artists.extend(hash),
            Err(RequestErr::OutOfTracks) => {
                break;
            }
            Err(RequestErr::MaxRetries) => {}
        }
    }
    println!("results processed");

    let mut filtered_artists = Vec::with_capacity(all_artists.len());
    for (k, v) in all_artists.into_iter() {
        if v >= min_tracks {
            filtered_artists.push(k);
        }
    }
    let tracks = get_new_tracks(&client, filtered_artists, &token)
        .await
        .unwrap();

    // NOTE: encode result
    let mut result = "".to_string();
    for i in 0..usize::min(PRINT_NUM_OF_TRACKS, tracks.len()) {
        let track = &tracks[i];
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
enum RequestErr {
    OutOfTracks,
    MaxRetries,
    // Retryes,
}
async fn parallel_req(
    offset: i32,
    token: String,
    client: Client,
) -> Result<HashMap<String, i32>, RequestErr> {
    let mut body_json = None;
    'retry: for ret in 0..6 {
        if body_json.is_none() {
            let res = get_users_tracks(&client, &token, offset).await;
            body_json = match res {
                Ok(r) => Some(r),
                Err(err) => {
                    println!("Error requesting body: {}", err);
                    println!("retrying: {ret}");
                    continue 'retry;
                }
            };
        }

        if body_json.is_none() {
            continue 'retry;
        }
        let res = get_artist(body_json.take().unwrap());

        if let Ok(some_artists) = res {
            if some_artists.is_empty() {
                println!("fetched empty request");
                return Err(RequestErr::OutOfTracks);
            }
            return Ok(some_artists);
        } else {
            let str = res.unwrap_err();
            println!("Error parsting body: {}", str);
            println!("retrying: {ret}");
        }
    }
    Err(RequestErr::MaxRetries)
}

async fn get_users_tracks(client: &Client, token: &str, offset: i32) -> Result<Value, String> {
    let mut url = Url::parse("https://api.spotify.com/v1/me/tracks").map_err(|e| e.to_string())?;
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
    let timeout = sleep(Duration::from_secs(5));
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
fn get_artist(body_json: Value) -> Result<HashMap<String, i32>, String> {
    let items = body_json.get("items").unwrap();
    let mut ids = HashMap::new();
    let items = items
        .as_array()
        .ok_or_else(|| "parsing error: not array".to_string());
    for item in items? {
        let artists = item.get("track").and_then(|e| e.get("artists")).unwrap();
        if let Some(artists) = artists.as_array() {
            for artist in artists {
                let id = artist.get("id").unwrap().as_str().unwrap();
                println!("{:?}", id);
                let count = ids.get(id).unwrap_or(&1).to_owned();
                ids.insert(id.to_string(), count);
            }
        }
    }
    Ok(ids)
}

const TRACK_PER_ARTIST: usize = 3;
// TODO: make parallel calls
async fn get_new_tracks(
    client: &Client,
    filtered_artists: Vec<String>,
    token: &str,
) -> Result<Vec<TrackItem>, String> {
    let mut tracks_list = Vec::with_capacity(filtered_artists.len() * TRACK_PER_ARTIST);
    for artist in filtered_artists {
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
            .bearer_auth(token)
            .build()
            .map_err(|e| e.to_string())?;
        println!("Requesting: {}", artist);

        let res = client.execute(req).await.map_err(|e| e.to_string())?;

        println!("Status: {}\n", res.status());

        if !res.status().is_success() {
            return Err("Failed to get albums".to_string());
        }

        let body = res.text().await.map_err(|e| e.to_string())?;
        let tracks: NewTracks = serde_json::from_str(&body).map_err(|e| e.to_string())?;

        for item in tracks.items {
            let res = parse_tracks(item);
            if let Ok(track) = res {
                tracks_list.push(track);
            } else {
                print!("Error parsing track of the artist ");
                println!("{}", res.err().unwrap());
                println!("Skipping");
            }
        }
    }
    tracks_list.sort_unstable_by(|a, b| b.date.cmp(&a.date));
    Ok(tracks_list)
}
fn parse_tracks(track: Track) -> Result<TrackItem, String> {
    let track_type = track.track_type;

    let id = track.id;
    let mut uri = String::new();
    uri.push_str(&format!("open.spotify.com/{track_type}/{id}"));
    let precision = track.release_date_precision;
    let fmt = if precision == "year" {
        "%Y"
    } else {
        "%Y-%m-%d"
    };
    let date = NaiveDate::parse_from_str(&track.release_date, fmt)
        .map_err(|e| format!("fmt = {}, {}, {:?}", fmt, e, e.kind()))?;

    let name = track.name;

    let mut artists = String::new();
    for artist in track.artists {
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
