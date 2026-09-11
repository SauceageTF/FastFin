import JellyfinAPI
import SwiftUI

/// The 2:3 poster + title card used in every "Recently Added" / "More Like
/// This" carousel.
struct PosterCard: View {
    let item: BaseItemDto
    let imageURL: URL?
    var width: CGFloat = 104

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            RemoteImage(url: imageURL)
                .frame(width: width, height: width * 1.5)
                .clipShape(RoundedRectangle(cornerRadius: 9, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 9, style: .continuous)
                        .strokeBorder(Color.white.opacity(0.07), lineWidth: 1)
                )

            Text(item.name ?? "")
                .font(.system(size: 11.5, weight: .semibold))
                .foregroundStyle(Theme.text)
                .lineLimit(1)
        }
        .frame(width: width)
    }
}
