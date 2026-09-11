import SwiftUI

/// One of the three theme colors from the desktop app's `theme.ts`.
enum ThemeColor: String, CaseIterable {
    case white, teal, ember

    var accent: Color {
        switch self {
        case .white: Color(hex: 0xf2f1f6)
        case .teal: Color(hex: 0x2dd4c8)
        case .ember: Color(hex: 0xff5a1f)
        }
    }

    var accentHover: Color {
        switch self {
        case .white: Color(hex: 0xd8d7db)
        case .teal: Color(hex: 0x55e0d6)
        case .ember: Color(hex: 0xff7a3d)
        }
    }
}

/// Static design tokens ported 1:1 from the desktop app's `app.css`, plus a
/// live `accent` that follows the user's chosen `ThemeColor` the same way
/// `--accent` is swapped on `documentElement` there.
enum Theme {
    static let background = Color(hex: 0x0a0a0e)
    static let backgroundElevated = Color(hex: 0x18181c)
    static let backgroundHover = Color(hex: 0x232327)
    static let text = Color(hex: 0xf2f1f6)
    static let textDim = Color(hex: 0xa9a7b5)
    static let danger = Color(hex: 0xff5c5c)
    static let border = Color(hex: 0x2c2c30)

    static var accent: Color { AppearanceStore.shared.themeColor.accent }
    static var accentHover: Color { AppearanceStore.shared.themeColor.accentHover }

    /// The brand display face (`--font-display: "Sora"`). Falls back to the
    /// system font until the real `.ttf` is bundled -- see the iOS README.
    static func displayFont(_ size: CGFloat, weight: Font.Weight = .bold) -> Font {
        .custom("Sora", size: size).weight(weight)
    }
}

extension Color {
    init(hex: UInt32, opacity: Double = 1) {
        self.init(
            .sRGB,
            red: Double((hex >> 16) & 0xff) / 255,
            green: Double((hex >> 8) & 0xff) / 255,
            blue: Double(hex & 0xff) / 255,
            opacity: opacity
        )
    }
}
