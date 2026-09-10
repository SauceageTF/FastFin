use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const CLIENT_NAME: &str = "FastFin";
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
    #[serde(rename = "SeriesId")]
    pub series_id: Option<String>,
    #[serde(rename = "SeasonId")]
    pub season_id: Option<String>,
    #[serde(rename = "UserData")]
    pub user_data: Option<UserData>,
    #[serde(rename = "BackdropImageTags")]
    pub backdrop_image_tags: Option<Vec<String>>,
    #[serde(rename = "ParentBackdropItemId")]
    pub parent_backdrop_item_id: Option<String>,
    #[serde(rename = "Genres")]
    pub genres: Option<Vec<String>>,
    #[serde(rename = "CommunityRating")]
    pub community_rating: Option<f64>,
    #[serde(rename = "OfficialRating")]
    pub official_rating: Option<String>,
    #[serde(rename = "Taglines")]
    pub taglines: Option<Vec<String>>,
    /// Which image types this item actually has (keyed by type, e.g.
    /// "Primary", "Logo", "Backdrop") -- a core BaseItemDto field, always
    /// present without needing to ask for it via `Fields`. Used to check
    /// whether a Logo image exists before requesting one, since most items
    /// don't have one and requesting it anyway would just 404.
    #[serde(rename = "ImageTags")]
    pub image_tags: Option<HashMap<String, String>>,
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

/// Normalizes a user-entered server URL into a proper `scheme://host[:port]`
/// form. Users commonly omit the scheme entirely (`dell:8096`) or drop the
/// `//` after it (`http:dell:8096`, easy to typo) -- either one leaves every
/// URL built from it looking like a relative path or a local file path
/// rather than a network address, which breaks image loading and mpv
/// playback silently rather than with an obvious error.
fn normalize_server_url(input: &str) -> String {
    let trimmed = input.trim();
    let (scheme, rest) = if let Some(rest) = trimmed.strip_prefix("https://") {
        ("https", rest)
    } else if let Some(rest) = trimmed.strip_prefix("https:") {
        ("https", rest.trim_start_matches('/'))
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        ("http", rest)
    } else if let Some(rest) = trimmed.strip_prefix("http:") {
        ("http", rest.trim_start_matches('/'))
    } else {
        ("http", trimmed)
    };
    format!("{scheme}://{}", rest.trim_end_matches('/'))
}

pub async fn authenticate(
    server_url: &str,
    username: &str,
    password: &str,
    device_id: &str,
) -> Result<Session, String> {
    let client = reqwest::Client::new();
    let server_url = normalize_server_url(server_url);
    let url = format!("{}/Users/AuthenticateByName", server_url);

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
        server_url,
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

/// Jellyfin's dedicated "recently added" endpoint for a library. Unlike
/// `/Items`, this one returns a bare JSON array rather than a
/// `{Items: [...], TotalRecordCount}` envelope, so it's deserialized
/// straight into `Vec<Item>` instead of through `ItemsResponse`.
pub async fn get_latest_items(session: &Session, library_id: &str) -> Result<Vec<Item>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Users/{}/Items/Latest?ParentId={}&Limit=16&Fields=Overview,ProductionYear,RunTimeTicks",
        session.server_url, session.user_id, library_id
    );

    let response = authed_request(&client, session, &url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load latest items: {}", response.status()));
    }

    response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))
}

pub async fn get_item(session: &Session, item_id: &str) -> Result<Item, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Users/{}/Items/{}?Fields=Overview,ProductionYear,RunTimeTicks,Genres,Taglines",
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

/// Jellyfin's `/Items/{id}/Similar` works for both movies and series -- no
/// need to branch on item type to pick a different endpoint.
pub async fn get_similar_items(session: &Session, item_id: &str) -> Result<Vec<Item>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Items/{}/Similar?userId={}&Limit=12&Fields=Overview,ProductionYear,RunTimeTicks",
        session.server_url, item_id, session.user_id
    );

    let response = authed_request(&client, session, &url)
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load similar items: {}", response.status()));
    }

    let parsed: ItemsResponse = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;

    Ok(parsed.items)
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

#[cfg(test)]
mod tests {
    use super::normalize_server_url;

    #[test]
    fn normalizes_missing_and_malformed_schemes() {
        assert_eq!(normalize_server_url("dell:8096"), "http://dell:8096");
        assert_eq!(normalize_server_url("http:dell:8096"), "http://dell:8096");
        assert_eq!(normalize_server_url("http://dell:8096"), "http://dell:8096");
        assert_eq!(normalize_server_url("http://dell:8096/"), "http://dell:8096");
        assert_eq!(normalize_server_url("https:jellyfin.example.com"), "https://jellyfin.example.com");
        assert_eq!(normalize_server_url("https://jellyfin.example.com/"), "https://jellyfin.example.com");
    }
}
