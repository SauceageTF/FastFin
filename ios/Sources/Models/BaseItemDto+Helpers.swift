import Foundation
import JellyfinAPI

/// Small helpers ported from `src/lib/types.ts`'s `hasLogo` / `backdropSourceId`.
extension BaseItemDto {
    /// Whether Jellyfin has a Logo (wordmark/title-art) image for this item.
    var hasLogo: Bool {
        imageTags?["Logo"] != nil
    }

    /// The item to actually source a backdrop image from: this item's own
    /// backdrop if it has one, else its parent's (an episode falls back to
    /// its series, which usually has one when the episode doesn't).
    var backdropSourceID: String? {
        if let tags = backdropImageTags, !tags.isEmpty { return id }
        return parentBackdropItemID
    }

    var formattedRuntime: String? {
        guard let ticks = runTimeTicks, ticks > 0 else { return nil }
        let totalMinutes = Int((Double(ticks) / 10_000_000 / 60).rounded())
        let hours = totalMinutes / 60
        let minutes = totalMinutes % 60
        return hours > 0 ? "\(hours)h \(minutes)m" : "\(minutes)m"
    }

    var episodeLabel: String? {
        guard let indexNumber else { return name }
        return "E\(indexNumber)\(name.map { " \u{00B7} \($0)" } ?? "")"
    }
}
