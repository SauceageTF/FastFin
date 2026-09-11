import JellyfinAPI
import SwiftUI

/// The wide 16:9 "Continue Watching" card with a title/subtitle scrim and a
/// resume progress bar baked into the thumbnail, matching the mockup and
/// `library/+page.svelte`'s `.wide-card`.
struct ContinueWatchingCard: View {
    let item: BaseItemDto
    let imageURL: URL?
    var width: CGFloat = 172

    private var subtitle: String {
        if let episodeLabel = item.episodeLabel, item.seriesName != nil {
            return episodeLabel
        }
        return item.formattedRuntime ?? ""
    }

    var body: some View {
        ZStack(alignment: .bottomLeading) {
            RemoteImage(url: imageURL)

            LinearGradient(
                colors: [.clear, .black.opacity(0.75)],
                startPoint: .init(x: 0.5, y: 0.35),
                endPoint: .bottom
            )

            VStack(alignment: .leading, spacing: 5) {
                Text(item.seriesName ?? item.name ?? "")
                    .font(.system(size: 12.5, weight: .semibold))
                    .foregroundStyle(Theme.text)
                    .lineLimit(1)
                Text(subtitle)
                    .font(.system(size: 10.5))
                    .foregroundStyle(Color(hex: 0xc9c7d1))
                    .lineLimit(1)

                if let played = item.userData?.playedPercentage {
                    GeometryReader { proxy in
                        ZStack(alignment: .leading) {
                            Capsule().fill(Color.white.opacity(0.22))
                            Capsule().fill(Theme.accent).frame(width: proxy.size.width * played / 100)
                        }
                    }
                    .frame(height: 2.5)
                }
            }
            .padding(10)
        }
        .frame(width: width, height: width * 9 / 16)
        .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .strokeBorder(Color.white.opacity(0.07), lineWidth: 1)
        )
    }
}
