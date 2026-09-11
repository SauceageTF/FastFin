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

    /// Wired up by `PlayerViewControllerRepresentable` to the UIKit
    /// `AVPictureInPictureController`, which SwiftUI has no direct access to.
    var requestPiPToggle: (() -> Void)?

    private var timeObserverToken: Any?
    private var itemStatusObservation: NSKeyValueObservation?

    init(url: URL, startSeconds: Double = 0) {
        player = AVPlayer(url: url)
        super.init()

        if startSeconds > 0 {
            player.seek(to: CMTime(seconds: startSeconds, preferredTimescale: 600))
        }

        itemStatusObservation = player.currentItem?.observe(\.duration, options: [.new]) { [weak self] item, _ in
            let seconds = item.duration.seconds
            guard seconds.isFinite, seconds > 0 else { return }
            Task { @MainActor in self?.duration = seconds }
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

        player.play()
    }

    deinit {
        if let token = timeObserverToken { player.removeTimeObserver(token) }
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
}
