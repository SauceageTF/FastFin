import AVKit
import UIKit

/// The UIKit half of the player: owns the `AVPlayerLayer` (video surface)
/// and `AVPictureInPictureController`, both of which need a real view/layer
/// hierarchy that SwiftUI doesn't give direct access to. The custom HUD is
/// drawn by SwiftUI on top of this, not by this controller -- this class
/// only ever shows raw video, matching the desktop app's split between the
/// mpv-driven video surface and a separate HUD overlay window.
final class PlayerViewController: UIViewController {
    private let model: PlayerModel
    private var playerLayer: AVPlayerLayer!
    private var pipController: AVPictureInPictureController?

    init(model: PlayerModel) {
        self.model = model
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .black

        playerLayer = AVPlayerLayer(player: model.player)
        playerLayer.videoGravity = .resizeAspect
        view.layer.addSublayer(playerLayer)

        if AVPictureInPictureController.isPictureInPictureSupported() {
            let controller = AVPictureInPictureController(playerLayer: playerLayer)
            controller?.delegate = self
            pipController = controller
            model.isPiPPossible = true
        }
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        playerLayer.frame = view.bounds
    }

    func togglePictureInPicture() {
        guard let pipController else { return }
        if pipController.isPictureInPictureActive {
            pipController.stopPictureInPicture()
        } else {
            pipController.startPictureInPicture()
        }
    }
}

extension PlayerViewController: AVPictureInPictureControllerDelegate {
    func pictureInPictureControllerDidStartPictureInPicture(_ pictureInPictureController: AVPictureInPictureController) {
        model.isPiPActive = true
    }

    func pictureInPictureControllerDidStopPictureInPicture(_ pictureInPictureController: AVPictureInPictureController) {
        model.isPiPActive = false
    }

    func pictureInPictureController(
        _ pictureInPictureController: AVPictureInPictureController,
        restoreUserInterfaceForPictureInPictureStopWithCompletionHandler completionHandler: @escaping (Bool) -> Void
    ) {
        // The player screen is still on screen underneath (PiP just shrinks
        // the same layer into a floating window) -- nothing extra to
        // restore, so hand control straight back.
        completionHandler(true)
    }
}
