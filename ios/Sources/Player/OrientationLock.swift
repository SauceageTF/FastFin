import UIKit

/// The rest of the app is portrait-only; the player screen wants landscape
/// (and free rotation) while it's on screen. `project.yml` has to list both
/// orientations app-wide (UIKit won't rotate into one that isn't listed at
/// all), so this is what actually restricts every other screen to portrait:
/// `PlayerContainerView` flips `current` to `.allButUpsideDown` on appear and
/// back to `.portrait` on disappear, and the app delegate below reads it.
enum OrientationLock {
    static var current: UIInterfaceOrientationMask = .portrait {
        didSet {
            guard let scene = UIApplication.shared.connectedScenes.first as? UIWindowScene else { return }
            scene.windows.first?.rootViewController?.setNeedsUpdateOfSupportedInterfaceOrientations()
            scene.requestGeometryUpdate(.iOS(interfaceOrientations: current))
        }
    }
}

final class AppDelegate: NSObject, UIApplicationDelegate {
    func application(
        _ application: UIApplication,
        supportedInterfaceOrientationsFor window: UIWindow?
    ) -> UIInterfaceOrientationMask {
        OrientationLock.current
    }
}
