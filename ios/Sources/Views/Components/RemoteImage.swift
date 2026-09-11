import SwiftUI

/// `AsyncImage` with the app's consistent loading/failure look instead of
/// each call site rolling its own -- an elevated-color fill that crossfades
/// into the real image once it decodes, standing in for `app.css`'s
/// `.skeleton` shimmer until a real BlurHash placeholder is wired up.
struct RemoteImage: View {
    let url: URL?
    var contentMode: ContentMode = .fill

    var body: some View {
        AsyncImage(url: url, transaction: Transaction(animation: .easeOut(duration: 0.2))) { phase in
            switch phase {
            case let .success(image):
                image.resizable().aspectRatio(contentMode: contentMode)
            default:
                Theme.backgroundElevated
            }
        }
    }
}
