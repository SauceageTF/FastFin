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

                if let playbackError = model.errorMessage {
                    VStack(spacing: 16) {
                        Text(playbackError)
                            .font(.system(size: 14))
                            .foregroundStyle(.white)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal, 32)
                        Button("Close") {
                            model.teardown()
                            dismiss()
                        }
                        .foregroundStyle(Theme.accent)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(Color.black.opacity(0.85))
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

            let startTicks = item.userData?.playbackPositionTicks ?? 0
            guard let playbackURL = try await MediaService.playbackURL(session: session, itemID: itemID, startTicks: startTicks) else {
                errorMessage = "The server didn't return a usable media source for this item."
                return
            }

            // No client-side seek here: startTicks was already sent to the
            // server above, so a transcoded HLS stream already begins at
            // that offset -- seeking again locally would double-apply it.
            model = PlayerModel(url: playbackURL)
        } catch {
            errorMessage = "Couldn't start playback: \(error.localizedDescription)"
        }
    }
}
