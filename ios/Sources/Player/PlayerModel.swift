import AVFoundation
import Combine

/// The shared playback state SwiftUI's `PlayerHUDView` binds to and UIKit's
/// `PlayerViewController` drives. `AVPlayer` itself is plain Foundation, so
/// it lives here rather than in the view controller -- only the *layer*
/// (`AVPlayerLayer`) and the PiP controller need a real UIKit view, which is
/// what `PlayerViewController` is for.
@MainActor
final class PlayerModel: NSObject, ObservableObject {
    let player: AVPlayer

    @Published private(set) var isPlaying = false
    @Published private(set) var currentTime: Double = 0
    @Published private(set) var duration: Double = 0
    @Published var isPiPActive = false
    @Published var isPiPPossible = false

    /// Set if the item fails to become playable, or playback stalls with an
    /// error -- surfaced by `PlayerContainerView` instead of leaving a
    /// silent black screen with no indication anything went wrong.
    @Published private(set) var errorMessage: String?

    /// Wired up by `PlayerViewControllerRepresentable` to the UIKit
    /// `AVPictureInPictureController`, which SwiftUI has no direct access to.
    var requestPiPToggle: (() -> Void)?

    private var timeObserverToken: Any?
    private var durationObservation: NSKeyValueObservation?
    private var itemStatusObservation: NSKeyValueObservation?
    private var playerStatusObservation: NSKeyValueObservation?
    private var failureObserver: NSObjectProtocol?

    init(url: URL, startSeconds: Double = 0) {
        player = AVPlayer()
        super.init()

        playerStatusObservation = player.observe(\.status, options: [.new]) { [weak self] observedPlayer, _ in
            Task { @MainActor in
                if observedPlayer.status == .failed {
                    self?.report(observedPlayer.error, context: "playback failed")
                }
            }
        }

        timeObserverToken = player.addPeriodicTimeObserver(forInterval: CMTime(seconds: 0.5, preferredTimescale: 600), queue: .main) { [weak self] time in
            Task { @MainActor in
                guard let self else { return }
                self.currentTime = time.seconds
                self.isPlaying = self.player.rate > 0
            }
        }

        try? AVAudioSession.sharedInstance().setCategory(.playback, mode: .moviePlayback)
        try? AVAudioSession.sharedInstance().setActive(true)

        load(url: url, startSeconds: startSeconds)
    }

    deinit {
        if let token = timeObserverToken { player.removeTimeObserver(token) }
        if let failureObserver { NotificationCenter.default.removeObserver(failureObserver) }
    }

    /// Swaps the currently playing source -- used both by the initial load
    /// and by switching audio/subtitle tracks, which rebuild the transcode
    /// URL with new stream indices rather than switching an in-place HLS
    /// rendition. Reuses the same `AVPlayer` (and hence the same
    /// `AVPlayerLayer`/PiP controller already attached to it) instead of
    /// tearing the whole player down.
    func switchSource(url: URL, startSeconds: Double = 0) {
        errorMessage = nil
        duration = 0
        currentTime = 0
        load(url: url, startSeconds: startSeconds)
    }

    private func load(url: URL, startSeconds: Double) {
        let item = AVPlayerItem(url: url)

        if let failureObserver { NotificationCenter.default.removeObserver(failureObserver) }
        durationObservation = item.observe(\.duration, options: [.new]) { [weak self] observedItem, _ in
            let seconds = observedItem.duration.seconds
            guard seconds.isFinite, seconds > 0 else { return }
            Task { @MainActor in self?.duration = seconds }
        }
        itemStatusObservation = item.observe(\.status, options: [.new]) { [weak self] observedItem, _ in
            Task { @MainActor in
                if observedItem.status == .failed {
                    self?.report(observedItem.error, context: "couldn't load this video")
                }
            }
        }
        failureObserver = NotificationCenter.default.addObserver(
            forName: .AVPlayerItemFailedToPlayToEndTime,
            object: item,
            queue: .main
        ) { [weak self] notification in
            let error = notification.userInfo?[AVPlayerItemFailedToPlayToEndTimeErrorKey] as? Error
            Task { @MainActor in self?.report(error, context: "playback stopped") }
        }

        player.replaceCurrentItem(with: item)
        if startSeconds > 0 {
            player.seek(to: CMTime(seconds: startSeconds, preferredTimescale: 600))
        }
        player.play()
    }

    func togglePlayPause() {
        if isPlaying { player.pause() } else { player.play() }
    }

    func skip(by seconds: Double) {
        let target = max(0, min(duration, currentTime + seconds))
        player.seek(to: CMTime(seconds: target, preferredTimescale: 600))
    }

    func seek(to seconds: Double) {
        player.seek(to: CMTime(seconds: seconds, preferredTimescale: 600))
    }

    func teardown() {
        player.pause()
        try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
    }

    private func report(_ error: Error?, context: String) {
        guard errorMessage == nil else { return } // keep the first, most useful error
        let detail = (error as NSError?).map { " (\($0.domain) \($0.code): \($0.localizedDescription))" } ?? ""
        errorMessage = "Playback \(context)\(detail)"
    }
}
