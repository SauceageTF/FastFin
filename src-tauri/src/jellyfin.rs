use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
    /// ISO-8601 timestamp of when the item was added to the library. Only
    /// populated when `DateCreated` is requested via `Fields`.
    #[serde(rename = "DateCreated")]
    pub date_created: Option<String>,
    #[serde(rename = "IndexNumber")]
    pub index_number: Option<i32>,
    /// Season number for episodes (0 = specials).
    #[serde(rename = "ParentIndexNumber")]
    pub parent_index_number: Option<i32>,
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
    #[serde(rename = "Played")]
    pub played: Option<bool>,
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
        "{}/Users/{}/Items?ParentId={}&Fields=Overview,ProductionYear,RunTimeTicks,DateCreated,Genres",
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

    let latest: Vec<Item> = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;

    collapse_episodes_into_series(session, &client, latest).await
}

/// `/Items/Latest` groups new episodes under their series -- but only when a
/// series has *two or more* new episodes. A series with a single new episode
/// comes back as that bare episode, so a TV library's "recently added" row
/// ends up as a mix of series posters and stray episode thumbnails. This
/// folds every such episode back into its series (fetched in one batched
/// `Ids=` request), keeping the original recency order and dropping repeats
/// so a series never shows up twice in the same row.
async fn collapse_episodes_into_series(
    session: &Session,
    client: &reqwest::Client,
    latest: Vec<Item>,
) -> Result<Vec<Item>, String> {
    let series_ids: Vec<&str> = latest
        .iter()
        .filter(|item| item.item_type == "Episode")
        .filter_map(|item| item.series_id.as_deref())
        .collect();
    if series_ids.is_empty() {
        return Ok(latest);
    }

    let url = format!(
        "{}/Users/{}/Items?Fields=Overview,ProductionYear,RunTimeTicks",
        session.server_url, session.user_id
    );
    let response = authed_request(client, session, &url)
        .query(&[("Ids", series_ids.join(","))])
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load latest items: {}", response.status()));
    }

    let parsed: ItemsResponse = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;
    let series_by_id: HashMap<String, Item> =
        parsed.items.into_iter().map(|item| (item.id.clone(), item)).collect();

    let mut seen: HashSet<String> = HashSet::new();
    let mut collapsed = Vec::with_capacity(latest.len());
    for item in latest {
        let resolved = match (item.item_type.as_str(), item.series_id.as_deref()) {
            // A series that was deleted between the two requests (or that
            // the user can't see) just falls back to the episode itself.
            ("Episode", Some(series_id)) => series_by_id.get(series_id).cloned().unwrap_or(item),
            _ => item,
        };
        if seen.insert(resolved.id.clone()) {
            collapsed.push(resolved);
        }
    }
    Ok(collapsed)
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

/// Every episode of a series across all seasons, in season/episode order.
/// Same endpoint as `get_episodes`, just without the `seasonId` filter. The
/// fields list is trimmed to what a random pick needs -- this can be a few
/// hundred items for a long-running show, and none of it is displayed.
pub async fn get_series_episodes(session: &Session, series_id: &str) -> Result<Vec<Item>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Shows/{}/Episodes?userId={}",
        session.server_url, series_id, session.user_id
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

/// Shared tail for every "GET a `{Items: [...]}` envelope" request.
async fn fetch_items(request: reqwest::RequestBuilder, what: &str) -> Result<Vec<Item>, String> {
    let response = request
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Failed to load {what}: {}", response.status()));
    }

    let parsed: ItemsResponse = response
        .json()
        .await
        .map_err(|e| format!("Unexpected response from server: {e}"))?;

    Ok(parsed.items)
}

/// Jellyfin's "Next Up": the next unwatched episode of every series the user
/// has started, most recently watched first. With `series_id` it narrows to
/// that one show, which is what a series page's "Continue" button needs
/// (it also returns S1E1 for a show that's never been started).
pub async fn get_next_up(session: &Session, series_id: Option<&str>) -> Result<Vec<Item>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Shows/NextUp?userId={}&Limit=20&Fields=Overview,ProductionYear,RunTimeTicks",
        session.server_url, session.user_id
    );
    let mut request = authed_request(&client, session, &url);
    if let Some(series_id) = series_id {
        request = request.query(&[("seriesId", series_id)]);
    }
    fetch_items(request, "next up").await
}

/// The episode that comes right after `episode_id` in its series' running
/// order (crossing season boundaries), or `None` at the end of the show.
/// `startItemId` makes the server skip ahead to the given episode, so this
/// is a single two-item request rather than fetching the whole series.
pub async fn get_next_episode(session: &Session, episode_id: &str) -> Result<Option<Item>, String> {
    let episode = get_item(session, episode_id).await?;
    let Some(series_id) = episode.series_id.as_deref() else {
        return Ok(None);
    };
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Shows/{}/Episodes?userId={}&Fields=Overview,ProductionYear,RunTimeTicks",
        session.server_url, series_id, session.user_id
    );
    let request = authed_request(&client, session, &url).query(&[("startItemId", episode_id), ("limit", "2")]);
    let mut items = fetch_items(request, "next episode").await?;
    // Items[0] is the starting episode itself.
    if items.len() < 2 || items[0].id != episode_id {
        return Ok(None);
    }
    Ok(Some(items.swap_remove(1)))
}

/// Marks an item watched (POST) or unwatched (DELETE) for this user.
pub async fn set_played(session: &Session, item_id: &str, played: bool) -> Result<(), String> {
    let client = reqwest::Client::new();
    let url = format!("{}/Users/{}/PlayedItems/{}", session.server_url, session.user_id, item_id);
    let request = if played { client.post(&url) } else { client.delete(&url) };
    let response = request
        .header("X-Emby-Authorization", auth_header(&session.device_id, Some(&session.access_token)))
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Failed to update watched state: {}", response.status()));
    }
    Ok(())
}

/// A skippable stretch of an item, in seconds. `kind` is one of `Intro`,
/// `Credits`, `Recap`, `Preview`, `Commercial` (Jellyfin's own media segment
/// type names, which the Intro Skipper plugin's modes are mapped onto).
#[derive(Debug, Clone, Serialize)]
pub struct SkipSegment {
    pub kind: String,
    pub start: f64,
    pub end: f64,
}

/// One entry of the Intro Skipper plugin's segment response.
#[derive(Debug, Deserialize)]
struct IntroSkipperSegment {
    #[serde(rename = "Valid")]
    valid: bool,
    #[serde(rename = "IntroStart", alias = "Start")]
    start: f64,
    #[serde(rename = "IntroEnd", alias = "End")]
    end: f64,
}

#[derive(Debug, Deserialize)]
struct MediaSegment {
    #[serde(rename = "Type")]
    segment_type: String,
    #[serde(rename = "StartTicks")]
    start_ticks: i64,
    #[serde(rename = "EndTicks")]
    end_ticks: i64,
}

#[derive(Debug, Deserialize)]
struct MediaSegmentsResponse {
    #[serde(rename = "Items")]
    items: Vec<MediaSegment>,
}

/// Skippable segments (intro, credits, ...) for an episode. Three sources,
/// tried in order (the legacy single-intro plugin endpoint sits between
/// the two below):
///
/// 1. The Intro Skipper plugin's own endpoint,
///    `/Episode/{id}/IntroSkipperSegments`, which returns a map of analysis
///    mode (`Introduction` / `Credits`) to timestamps. This is what almost
///    every install has.
/// 2. Jellyfin's native media segments API (`/MediaSegments/{id}`, 10.10+),
///    which newer Intro Skipper builds can also write into, and which other
///    segment providers use.
///
/// Neither being available is normal (plugin not installed, older server,
/// episode not analyzed yet), so this never fails -- it just returns an
/// empty list and the player shows no skip buttons.
pub async fn get_skip_segments(session: &Session, item_id: &str) -> Vec<SkipSegment> {
    let client = reqwest::Client::new();
    let auth = auth_header(&session.device_id, Some(&session.access_token));

    let plugin_url = format!("{}/Episode/{}/IntroSkipperSegments", session.server_url, item_id);
    if let Ok(response) = client.get(&plugin_url).header("X-Emby-Authorization", &auth).send().await {
        if response.status().is_success() {
            if let Ok(map) = response.json::<HashMap<String, IntroSkipperSegment>>().await {
                let segments: Vec<SkipSegment> = map
                    .into_iter()
                    .filter(|(_, seg)| seg.valid && seg.end > seg.start)
                    .map(|(mode, seg)| SkipSegment {
                        kind: match mode.as_str() {
                            "Introduction" => "Intro".to_string(),
                            other => other.to_string(),
                        },
                        start: seg.start,
                        end: seg.end,
                    })
                    .collect();
                if !segments.is_empty() {
                    return segments;
                }
            }
        }
    }

    // Older plugin builds only have the single-intro endpoint.
    let legacy_url = format!("{}/Episode/{}/IntroTimestamps", session.server_url, item_id);
    if let Ok(response) = client.get(&legacy_url).header("X-Emby-Authorization", &auth).send().await {
        if response.status().is_success() {
            if let Ok(seg) = response.json::<IntroSkipperSegment>().await {
                if seg.valid && seg.end > seg.start {
                    return vec![SkipSegment { kind: "Intro".to_string(), start: seg.start, end: seg.end }];
                }
            }
        }
    }

    let native_url = format!("{}/MediaSegments/{}", session.server_url, item_id);
    if let Ok(response) = client.get(&native_url).header("X-Emby-Authorization", &auth).send().await {
        if response.status().is_success() {
            if let Ok(parsed) = response.json::<MediaSegmentsResponse>().await {
                return parsed
                    .items
                    .into_iter()
                    .filter(|seg| seg.end_ticks > seg.start_ticks)
                    .map(|seg| SkipSegment {
                        // Jellyfin calls the end-credits segment "Outro"; the
                        // plugin calls it "Credits". Normalize on the latter.
                        kind: if seg.segment_type == "Outro" { "Credits".to_string() } else { seg.segment_type },
                        start: seg.start_ticks as f64 / 10_000_000.0,
                        end: seg.end_ticks as f64 / 10_000_000.0,
                    })
                    .collect();
            }
        }
    }

    Vec::new()
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

/// Title search across every library, restricted to top-level titles
/// (`Movie`/`Series`) -- Jellyfin's search would otherwise also match every
/// individual episode whose name happens to contain the term, burying the
/// show itself under its own episodes. The term goes through reqwest's
/// `query()` rather than the URL `format!` the other requests use, since it's
/// free-form user input that needs proper percent-encoding.
pub async fn search_items(session: &Session, term: &str) -> Result<Vec<Item>, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/Users/{}/Items?Recursive=true&IncludeItemTypes=Movie,Series&Limit=60&Fields=Overview,ProductionYear,RunTimeTicks",
        session.server_url, session.user_id
    );

    let response = authed_request(&client, session, &url)
        .query(&[("SearchTerm", term)])
        .send()
        .await
        .map_err(|e| format!("Failed to reach server: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Search failed: {}", response.status()));
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
