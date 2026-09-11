import JellyfinAPI
import SwiftUI

/// The 16:9 thumbnail + title + duration/progress card used in the "Next
/// Up" and season episode carousels, matching `.next-up-card` on desktop.
struct EpisodeCard: View {
    let episode: BaseItemDto
    let imageURL: URL?
    var width: CGFloat = 156

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            ZStack(alignment: .bottomLeading) {
                RemoteImage(url: imageURL)
                    .overlay(
                        Image(systemName: "play.fill")
                            .font(.system(size: 22))
                            .foregroundStyle(.white.opacity(0.9))
                    )

                if let played = episode.userData?.playedPercentage, played > 0 {
                    GeometryReader { proxy in
                        ZStack(alignment: .leading) {
                            Rectangle().fill(Color.white.opacity(0.25))
                            Rectangle().fill(Theme.accent).frame(width: proxy.size.width * played / 100)
                        }
                    }
                    .frame(height: 3)
                    .frame(maxHeight: .infinity, alignment: .bottom)
                }
            }
            .frame(width: width, height: width * 9 / 16)
            .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(Color.white.opacity(0.08)))

            Text(episode.episodeLabel ?? episode.name ?? "")
                .font(.system(size: 12.5, weight: .semibold))
                .foregroundStyle(Theme.text)
                .lineLimit(1)

            Text(remainingLabel)
                .font(.system(size: 10.5))
                .foregroundStyle(Theme.textDim)
        }
        .frame(width: width, alignment: .leading)
    }

    private var remainingLabel: String {
        if let ticks = episode.userData?.playbackPositionTicks, ticks > 0,
           let totalTicks = episode.runTimeTicks, totalTicks > ticks
        {
            let remainingMinutes = Int((Double(totalTicks - ticks) / 10_000_000 / 60).rounded())
            return "\(remainingMinutes)m left"
        }
        return episode.formattedRuntime ?? ""
    }
}
