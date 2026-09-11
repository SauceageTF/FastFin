import SwiftUI

/// The floating glass icon button used over hero/backdrop imagery throughout
/// the mockup (nav bar search/avatar, item detail back/cast/more, player
/// HUD). Meets the 44pt minimum tap target regardless of the icon's size.
struct GlassCircleButton: View {
    let systemImage: String
    var diameter: CGFloat = 44
    var action: () -> Void = {}

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: diameter * 0.4, weight: .semibold))
                .foregroundStyle(Theme.text)
                .frame(width: diameter, height: diameter)
                .background(.ultraThinMaterial, in: Circle())
                .overlay(Circle().strokeBorder(Color.white.opacity(0.15), lineWidth: 1))
        }
        .buttonStyle(.plain)
    }
}
