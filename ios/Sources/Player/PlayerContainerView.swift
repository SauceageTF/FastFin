import JellyfinAPI
import SwiftUI

/// The player screen: raw video (`PlayerViewControllerRepresentable`) with
/// the custom `PlayerHUDView` drawn on top, full-bleed and landscape --
/// locks orientation on appear/disappear via `OrientationLock` since this is
/// the one screen in the app that isn't portrait-only. Presented as a
/// `.fullScreenCover` from `PlayerPresenter` (see RootTabView) rather than
/// pushed on a NavigationStack, so it fully takes over the screen with no
/// nav chrome or back-swipe gesture underneath it.
struct PlayerContainerView: View {
    let itemID: String

    @EnvironmentObject private var session: JellyfinSession
    @Environment(\.dismiss) private var dismiss
    @State private var model: PlayerModel?
    @State private var title = ""
    @State private var subtitle: String?
    @State private var errorMessage: String?
    @State private var startTicks = 0
    @State private var audioTracks: [TrackOption] = []
    @State private var subtitleTracks: [TrackOption] = []
    @State private var selectedAudioIndex: Int?
    @State private var selectedSubtitleIndex: Int?

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()

            if let model {
                // A separate view holding `model` as `@ObservedObject`, not
                // `@State` here, is deliberate: PlayerContainerView reading
                // `model.errorMessage` directly in ITS OWN body doesn't
                // actually subscribe to PlayerModel's @Published changes
                // (only @ObservedObject/@StateObject do) -- the error
                // overlay could be silently never appearing even when
                // PlayerModel correctly captured a real error.
                PlayerActiveContent(
                    model: model,
                    title: title,
                    subtitle: subtitle,
                    audioTracks: audioTracks,
                    subtitleTracks: subtitleTracks,
                    selectedAudioIndex: selectedAudioIndex,
                    selectedSubtitleIndex: selectedSubtitleIndex,
                    onSelectAudio: { switchTrack(audioIndex: $0, subtitleIndex: selectedSubtitleIndex) },
                    onSelectSubtitle: { switchTrack(audioIndex: selectedAudioIndex, subtitleIndex: $0) },
                    onClose: {
                        model.teardown()
                        dismiss()
                    }
                )
            } else if let errorMessage {
                VStack(spacing: 16) {
                    Text(errorMessage)
                        .foregroundStyle(Theme.danger)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal, 32)
                    Button("Close") { dismiss() }.foregroundStyle(Theme.accent)
                }
            } else {
                ProgressView().tint(.white)
            }
        }
        .statusBarHidden()
        .onAppear { OrientationLock.current = .landscape }
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
            startTicks = item.userData?.playbackPositionTicks ?? 0

            guard let source = try await MediaService.playbackSource(session: session, itemID: itemID, startTicks: startTicks) else {
                errorMessage = "The server didn't return a usable media source for this item."
                return
            }
            audioTracks = source.audioTracks
            subtitleTracks = source.subtitleTracks
            selectedAudioIndex = source.selectedAudioIndex
            selectedSubtitleIndex = source.selectedSubtitleIndex

            // No client-side seek here: startTicks was already sent to the
            // server above, so a transcoded HLS stream already begins at
            // that offset -- seeking again locally would double-apply it.
            model = PlayerModel(url: source.url)
        } catch {
            errorMessage = "Couldn't start playback: \(error.localizedDescription)"
        }
    }

    private func switchTrack(audioIndex: Int?, subtitleIndex: Int?) {
        guard let model else { return }
        let resumeSeconds = model.currentTime
        Task {
            guard let source = try? await MediaService.playbackSource(
                session: session,
                itemID: itemID,
                startTicks: Int(resumeSeconds * 10_000_000),
                audioStreamIndex: audioIndex,
                subtitleStreamIndex: subtitleIndex
            ) else { return }

            selectedAudioIndex = source.selectedAudioIndex
            selectedSubtitleIndex = source.selectedSubtitleIndex
            model.switchSource(url: source.url)
        }
    }
}

/// Everything that needs to react to `PlayerModel`'s @Published state --
/// split out from `PlayerContainerView` specifically so that reactivity
/// works (see the comment at its call site above).
private struct PlayerActiveContent: View {
    @ObservedObject var model: PlayerModel
    let title: String
    let subtitle: String?
    let audioTracks: [TrackOption]
    let subtitleTracks: [TrackOption]
    let selectedAudioIndex: Int?
    let selectedSubtitleIndex: Int?
    let onSelectAudio: (Int?) -> Void
    let onSelectSubtitle: (Int?) -> Void
    let onClose: () -> Void

    var body: some View {
        ZStack {
            PlayerViewControllerRepresentable(model: model)
                .ignoresSafeArea()

            // Picture in Picture floats the video in its own system-managed
            // window -- force-landscaping the rest of the app while it's
            // active would fight whatever orientation the user is actually
            // browsing in behind it.
            Color.clear
                .onChange(of: model.isPiPActive) { _, isPiPActive in
                    OrientationLock.current = isPiPActive ? .all : .landscape
                }

            PlayerHUDView(
                model: model,
                title: title,
                subtitle: subtitle,
                audioTracks: audioTracks,
                subtitleTracks: subtitleTracks,
                selectedAudioIndex: selectedAudioIndex,
                selectedSubtitleIndex: selectedSubtitleIndex,
                onSelectAudio: onSelectAudio,
                onSelectSubtitle: onSelectSubtitle,
                onClose: onClose
            )

            if let playbackError = model.errorMessage {
                VStack(spacing: 16) {
                    Text(playbackError)
                        .font(.system(size: 14))
                        .foregroundStyle(.white)
                        .multilineTextAlignment(.center)
                        .padding(.horizontal, 32)
                    Button("Close", action: onClose)
                        .foregroundStyle(Theme.accent)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color.black.opacity(0.85))
            }
        }
    }
}
