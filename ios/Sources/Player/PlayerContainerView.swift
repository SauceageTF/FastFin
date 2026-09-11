import JellyfinAPI
import SwiftUI

/// The player screen: raw video (`PlayerViewControllerRepresentable`) with
/// the custom `PlayerHUDView` drawn on top, full-bleed and landscape --
/// locks orientation on appear/disappear via `OrientationLock` since this is
/// the one screen in the app that isn't portrait-only.
struct PlayerContainerView: View {
    let itemID: String

    @EnvironmentObject private var session: JellyfinSession
    @Environment(\.dismiss) private var dismiss
    @State private var model: PlayerModel?
    @State private var title = ""
    @State private var subtitle: String?
    @State private var errorMessage: String?

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            if let model {
                PlayerViewControllerRepresentable(model: model)
                    .ignoresSafeArea()
                PlayerHUDView(model: model, title: title, subtitle: subtitle) {
                    model.teardown()
                    dismiss()
                }
            } else if let errorMessage {
                Text(errorMessage).foregroundStyle(Theme.danger)
            } else {
                ProgressView().tint(.white)
            }
        }
        .navigationBarHidden(true)
        .statusBarHidden()
        .onAppear { OrientationLock.current = .allButUpsideDown }
        .onDisappear {
            OrientationLock.current = .portrait
            model?.teardown()
        }
        .task { await load() }
    }

    private func load() async {
        do {
            let item = try await MediaService.item(session: session, id: itemID)
            title = item.seriesName ?? item.name ?? "FastFin"
            subtitle = item.episodeLabel.flatMap { item.seriesName != nil ? $0 : nil }

            guard let streamURL = MediaService.streamURL(session: session, itemID: itemID) else {
                errorMessage = "Couldn't build a playback URL."
                return
            }

            let startSeconds = Double(item.userData?.playbackPositionTicks ?? 0) / 10_000_000
            model = PlayerModel(url: streamURL, startSeconds: startSeconds)
        } catch {
            errorMessage = "Couldn't start playback: \(error.localizedDescription)"
        }
    }
}
