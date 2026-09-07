use serde::{Deserialize, Serialize};

const CLIENT_NAME: &str = "Saucefin";
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub server_url: String,
    pub user_id: String,
    pub access_token: String,
    pub device_id: String,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    #[serde(rename = "AccessToken")]
    access_token: String,
    #[serde(rename = "User")]
    user: AuthUser,
}

#[derive(Debug, Deserialize)]
struct AuthUser {
    #[serde(rename = "Id")]
    id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "CollectionType")]
    pub collection_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LibrariesResponse {
    #[serde(rename = "Items")]
    items: Vec<Library>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub item_type: String,
    #[serde(rename = "Overview")]
    pub overview: Option<String>,
    #[serde(rename = "ProductionYear")]
    pub production_year: Option<i32>,
    #[serde(rename = "RunTimeTicks")]
    pub run_time_ticks: Option<i64>,
    #[serde(rename = "IndexNumber")]
    pub index_number: Option<i32>,
    #[serde(rename = "SeriesName")]
    pub series_name: Option<String>,
    #[serde(rename = "UserData")]
    pub user_data: Option<UserData>,
    #[serde(rename = "BackdropImageTags")]
    pub backdrop_image_tags: Option<Vec<String>>,
    #[serde(rename = "ParentBackdropItemId")]
    pub parent_backdrop_item_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserData {
    #[serde(rename = "PlaybackPositionTicks")]
    pub playback_position_ticks: Option<i64>,
    #[serde(rename = "PlayedPercentage")]
    pub played_percentage: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ItemsResponse {
    #[serde(rename = "Items")]
    items: Vec<Item>,
}

fn auth_header(device_id: &str, token: Option<&str>) -> String {
    let mut header = format!(
        r#"MediaBrowser Client="{}", Device="Windows Desktop", DeviceId="{}", Version="{}""#,
        CLIENT_NAME, device_id, CLIENT_VERSION
    );
    if let Some(token) = token {
        header.push_str(&format!(r#", Token="{}""#, token));
    }
    header
}

pub async fn authenticate(
    server_url: &str,
    username: &str,
    password: &str,
    device_id: &str,
) -> Result<Session, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Users/AuthenticateByName",
        server_url.trim_end_matches('/')
    );

    let response = client
        .post(&url)
        .header("X-Emby-Authorization", auth_header(device_id, None))
        .json(&serde_json::json!({ "Username": username, "Pw": password }))
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Login failed: server responded with {}",
            response.status()
        ));
    }

    let auth: AuthResponse = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;

    Ok(Session {
        server_url: server_url.trim_end_matches('/').to_string(),
        user_id: auth.user.id,
        access_token: auth.access_token,
        device_id: device_id.to_string(),
    })
}

fn authed_request(client: &reqwest::Client, session: &Session, url: &str) -> reqwest::RequestBuilder {
    client
        .get(url)
        .header("X-Emby-Authorization", auth_header(&session.device_id, Some(&session.access_token)))
}

pub async fn get_libraries(session: &Session) -> Result<Vec<Library>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/Users/{}/Views", session.server_url, session.user_id);

    let response = authed_request(&client, session, &url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load libraries: {}", response.status()));
    }

    let parsed: LibrariesResponse = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;

    Ok(parsed.items)
}

pub async fn get_items(session: &Session, library_id: &str) -> Result<Vec<Item>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Users/{}/Items?ParentId={}&Fields=Overview,ProductionYear,RunTimeTicks",
        session.server_url, session.user_id, library_id
    );

    let response = authed_request(&client, session, &url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load items: {}", response.status()));
    }

    let parsed: ItemsResponse = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;

    Ok(parsed.items)
}

pub async fn get_item(session: &Session, item_id: &str) -> Result<Item, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Users/{}/Items/{}?Fields=Overview,ProductionYear,RunTimeTicks",
        session.server_url, session.user_id, item_id
    );

    let response = authed_request(&client, session, &url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load item: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))
}

pub async fn get_seasons(session: &Session, series_id: &str) -> Result<Vec<Item>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Shows/{}/Seasons?userId={}&Fields=Overview,ProductionYear",
        session.server_url, series_id, session.user_id
    );

    let response = authed_request(&client, session, &url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load seasons: {}", response.status()));
    }

    let parsed: ItemsResponse = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;

    Ok(parsed.items)
}

pub async fn get_episodes(session: &Session, series_id: &str, season_id: &str) -> Result<Vec<Item>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Shows/{}/Episodes?seasonId={}&userId={}&Fields=Overview,ProductionYear,RunTimeTicks",
        session.server_url, series_id, season_id, session.user_id
    );

    let response = authed_request(&client, session, &url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load episodes: {}", response.status()));
    }

    let parsed: ItemsResponse = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;

    Ok(parsed.items)
}

pub async fn get_playlists(session: &Session) -> Result<Vec<Item>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Users/{}/Items?IncludeItemTypes=Playlist&Recursive=true",
        session.server_url, session.user_id
    );

    let response = authed_request(&client, session, &url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load playlists: {}", response.status()));
    }

    let parsed: ItemsResponse = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;

    Ok(parsed.items)
}

pub async fn get_resume(session: &Session) -> Result<Vec<Item>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Users/{}/Items/Resume?Limit=20&Fields=Overview,ProductionYear,RunTimeTicks",
        session.server_url, session.user_id
    );

    let response = authed_request(&client, session, &url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load continue watching: {}", response.status()));
    }

    let parsed: ItemsResponse = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;

    Ok(parsed.items)
}

pub fn image_url(session: &Session, item_id: &str) -> String {
    format!(
        "{}/Items/{}/Images/Primary?api_key={}",
        session.server_url, item_id, session.access_token
    )
}

/// Backdrop images are a per-item array in Jellyfin (unlike Primary), so the
/// image index must be explicit -- omitting it 404s rather than defaulting.
pub fn backdrop_url(session: &Session, item_id: &str) -> String {
    format!(
        "{}/Items/{}/Images/Backdrop/0?api_key={}",
        session.server_url, item_id, session.access_token
    )
}

/// Builds a direct-play stream URL for the given item.
///
/// This intentionally skips Jellyfin's `PlaybackInfo` transcode-negotiation
/// flow: mpv's own codec support covers direct play for effectively
/// everything Jellyfin would serve raw, which is the whole reason this app
/// renders through mpv instead of a `<video>` tag. If transcoding negotiation
/// is ever needed, swap this function's implementation only.
pub fn build_stream_url(session: &Session, item_id: &str, play_session_id: &str) -> String {
    format!(
        "{}/Videos/{}/stream?static=true&api_key={}&PlaySessionId={}",
        session.server_url, item_id, session.access_token, play_session_id
    )
}

fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds * 10_000_000.0).round() as i64
}

async fn post_playstate(session: &Session, path: &str, body: serde_json::Value) {
    let client = reqwest::Client::new();
    let url = format!("{}{}", session.server_url, path);
    let result = client
        .post(&url)
        .header(
            "X-Emby-Authorization",
            auth_header(&session.device_id, Some(&session.access_token)),
        )
        .json(&body)
        .send()
        .await;
    if let Err(e) = result {
        eprintln!("[jellyfin] Failed to report playback state to {path}: {e}");
    }
}

pub async fn report_playback_start(session: &Session, item_id: &str, play_session_id: &str, position_seconds: f64) {
    post_playstate(
        session,
        "/Sessions/Playing",
        serde_json::json!({
            "ItemId": item_id,
            "PlaySessionId": play_session_id,
            "PositionTicks": seconds_to_ticks(position_seconds),
            "IsPaused": false,
            "CanSeek": true,
        }),
    )
    .await;
}

pub async fn report_playback_progress(
    session: &Session,
    item_id: &str,
    play_session_id: &str,
    position_seconds: f64,
    is_paused: bool,
) {
    post_playstate(
        session,
        "/Sessions/Playing/Progress",
        serde_json::json!({
            "ItemId": item_id,
            "PlaySessionId": play_session_id,
            "PositionTicks": seconds_to_ticks(position_seconds),
            "IsPaused": is_paused,
            "CanSeek": true,
        }),
    )
    .await;
}

pub async fn report_playback_stopped(session: &Session, item_id: &str, play_session_id: &str, position_seconds: f64) {
    post_playstate(
        session,
        "/Sessions/Playing/Stopped",
        serde_json::json!({
            "ItemId": item_id,
            "PlaySessionId": play_session_id,
            "PositionTicks": seconds_to_ticks(position_seconds),
        }),
    )
    .await;
}
