import AVKit
import SwiftUI

/// `AVRoutePickerButton` is a UIKit view (no native SwiftUI equivalent) --
/// this just hosts it so `PlayerHUDView` can drop it in like any other
/// SwiftUI control.
struct AirPlayButton: UIViewRepresentable {
    func makeUIView(context: Context) -> AVRoutePickerButton {
        let button = AVRoutePickerButton()
        button.tintColor = .white
        button.activeTintColor = Theme.accent.uiColor
        return button
    }

    func updateUIView(_ uiView: AVRoutePickerButton, context: Context) {}
}

private extension Color {
    /// `AVRoutePickerButton.activeTintColor` needs a `UIColor`, and `Theme`
    /// only deals in SwiftUI `Color` -- this is a one-off bridge, not a
    /// general-purpose conversion.
    var uiColor: UIColor {
        UIColor(self)
    }
}
