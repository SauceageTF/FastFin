import Foundation
import JellyfinAPI

enum MediaServiceError: Error {
    case notAuthenticated
}

/// Mirrors `src/lib/jellyfinClient.ts`'s exported functions, but calls the
/// Jellyfin REST API directly through the `JellyfinAPI` SDK instead of going
/// through a Rust IPC command -- there's no Tauri backend on iOS.
@MainActor
enum MediaService {
    // MARK: - Browsing

    static func libraries(session: JellyfinSession) async throws -> [BaseItemDto] {
        guard let client = session.client, let userID = session.userID else { return [] }
        let parameters = Paths.GetUserViewsParameters(userID: userID)
        let result = try await client.send(Paths.getUserViews(parameters: parameters)).value
        return result.items ?? []
    }

    /// Every item in one library, mirrors `getItems` in jellyfinClient.ts
    /// (there `get_items` fetches the whole library in one call; same
    /// simplification here rather than building out paging for a v1).
    static func items(session: JellyfinSession, libraryID: String, limit: Int = 500) async throws -> [BaseItemDto] {
        guard let client = session.client, let userID = session.userID else { return [] }
        let parameters = Paths.GetItemsParameters(
            userID: userID,
            limit: limit,
            isRecursive: true,
            parentID: libraryID,
            includeItemTypes: [.movie, .series],
            sortBy: [.sortName]
        )
        let result = try await client.send(Paths.getItems(parameters: parameters)).value
        return result.items ?? []
    }

    /// Search across every library. Used by `SearchView`.
    static func search(session: JellyfinSession, query: String, limit: Int = 50) async throws -> [BaseItemDto] {
        guard let client = session.client, let userID = session.userID, !query.isEmpty else { return [] }
        let parameters = Paths.GetItemsParameters(
            userID: userID,
            limit: limit,
            isRecursive: true,
            searchTerm: query,
            includeItemTypes: [.movie, .series],
            sortBy: [.sortName]
        )
        let result = try await client.send(Paths.getItems(parameters: parameters)).value
        return result.items ?? []
    }

    static func latestItems(session: JellyfinSession, libraryID: String, limit: Int = 16) async throws -> [BaseItemDto] {
        guard let client = session.client, let userID = session.userID else { return [] }
        let parameters = Paths.GetLatestMediaParameters(userID: userID, parentID: libraryID, limit: limit)
        return try await client.send(Paths.getLatestMedia(parameters: parameters)).value
    }

    static func resumeItems(session: JellyfinSession, limit: Int = 16) async throws -> [BaseItemDto] {
        guard let client = session.client, let userID = session.userID else { return [] }
        let parameters = Paths.GetResumeItemsParameters(userID: userID, limit: limit)
        let result = try await client.send(Paths.getResumeItems(parameters: parameters)).value
        return result.items ?? []
    }

    static func item(session: JellyfinSession, id: String) async throws -> BaseItemDto {
        guard let client = session.client else { throw MediaServiceError.notAuthenticated }
        return try await client.send(Paths.getItem(itemID: id, userID: session.userID)).value
    }

    static func seasons(session: JellyfinSession, seriesID: String) async throws -> [BaseItemDto] {
        guard let client = session.client, let userID = session.userID else { return [] }
        let parameters = Paths.GetSeasonsParameters(userID: userID)
        let result = try await client.send(Paths.getSeasons(seriesID: seriesID, parameters: parameters)).value
        return result.items ?? []
    }

    static func episodes(session: JellyfinSession, seriesID: String, seasonID: String) async throws -> [BaseItemDto] {
        guard let client = session.client, let userID = session.userID else { return [] }
        let parameters = Paths.GetEpisodesParameters(userID: userID, seasonID: seasonID)
        let result = try await client.send(Paths.getEpisodes(seriesID: seriesID, parameters: parameters)).value
        return result.items ?? []
    }

    static func similarItems(session: JellyfinSession, itemID: String, limit: Int = 16) async throws -> [BaseItemDto] {
        guard let client = session.client, let userID = session.userID else { return [] }
        let parameters = Paths.GetSimilarItemsParameters(userID: userID, limit: limit)
        let result = try await client.send(Paths.getSimilarItems(itemID: itemID, parameters: parameters)).value
        return result.items ?? []
    }

    // MARK: - Images
    //
    // Same shapes as jellyfinClient.ts's getImageUrl/getBackdropUrl/getLogoUrl
    // (server + api_key query param + maxWidth), just built through the SDK's
    // `client.url(with:queryAPIKey:)` instead of hand-formatting a string.

    static func imageURL(session: JellyfinSession, itemID: String, maxWidth: Int = 480) -> URL? {
        guard let client = session.client else { return nil }
        let request = Paths.getItemImage(itemID: itemID, imageType: "Primary", parameters: .init(maxWidth: maxWidth))
        return client.url(with: request, queryAPIKey: true)
    }

    /// Only call once `item.hasLogo` is true -- requesting a Logo image that
    /// doesn't exist 404s rather than returning a placeholder.
    static func logoURL(session: JellyfinSession, itemID: String, maxWidth: Int = 800) -> URL? {
        guard let client = session.client else { return nil }
        let request = Paths.getItemImage(itemID: itemID, imageType: "Logo", parameters: .init(maxWidth: maxWidth))
        return client.url(with: request, queryAPIKey: true)
    }

    /// Backdrops are a per-item array in Jellyfin (unlike Primary), so index
    /// 0 must be explicit -- omitting it 404s instead of defaulting.
    static func backdropURL(session: JellyfinSession, itemID: String, maxWidth: Int = 1920) -> URL? {
        guard let client = session.client else { return nil }
        let request = Paths.getItemImageByIndex(
            itemID: itemID,
            imageType: "Backdrop",
            imageIndex: 0,
            parameters: .init(maxWidth: maxWidth)
        )
        return client.url(with: request, queryAPIKey: true)
    }

    // MARK: - Playback
    //
    // Builds the HLS transcode URL directly rather than trusting
    // PlaybackInfo's negotiated `transcodingUrl`: without a full
    // DeviceProfile in the POST body describing what AVPlayer actually
    // supports, the server can't reliably tell it needs to transcode, and
    // came back with no transcoding URL at all (mediaSources?.first was
    // "playable" per the server's own judgement, but that judgement assumes
    // a client that can direct-play MKV, which AVPlayer can't). Forcing the
    // well-known `/master.m3u8` transcode endpoint with an explicit
    // H.264/AAC target sidesteps that negotiation entirely and always gets
    // something AVPlayer can decode.
    static func playbackURL(session: JellyfinSession, itemID: String, startTicks: Int) -> URL? {
        guard let client = session.client, let accessToken = client.accessToken else { return nil }
        guard let joined = client.url(path: "/Videos/\(itemID)/master.m3u8") else { return nil }
        guard var components = URLComponents(url: joined, resolvingAgainstBaseURL: false) else { return nil }

        components.queryItems = [
            URLQueryItem(name: "api_key", value: accessToken),
            URLQueryItem(name: "DeviceId", value: session.deviceID),
            URLQueryItem(name: "MediaSourceId", value: itemID),
            URLQueryItem(name: "PlaySessionId", value: UUID().uuidString),
            URLQueryItem(name: "VideoCodec", value: "h264"),
            URLQueryItem(name: "AudioCodec", value: "aac"),
            URLQueryItem(name: "TranscodingMaxAudioChannels", value: "2"),
            URLQueryItem(name: "SegmentContainer", value: "ts"),
            URLQueryItem(name: "StartTimeTicks", value: String(startTicks)),
        ]
        return components.url
    }
}
