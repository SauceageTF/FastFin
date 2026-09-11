import Combine
import Foundation

/// Persists the user's chosen accent (`ThemeColor`), mirroring the desktop
/// app's `themeColor` writable store in `theme.ts` (same default: `ember`).
final class AppearanceStore: ObservableObject {
    static let shared = AppearanceStore()

    private let storageKey = "fastfin-theme-color"

    @Published var themeColor: ThemeColor {
        didSet {
            UserDefaults.standard.set(themeColor.rawValue, forKey: storageKey)
        }
    }

    private init() {
        let stored = UserDefaults.standard.string(forKey: storageKey).flatMap(ThemeColor.init(rawValue:))
        themeColor = stored ?? .ember
    }
}
