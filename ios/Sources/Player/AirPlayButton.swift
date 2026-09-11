import AVKit
import SwiftUI

/// `AVRoutePickerView` is a UIKit view (no native SwiftUI equivalent) --
/// this just hosts it so `PlayerHUDView` can drop it in like any other
/// SwiftUI control. (Not `AVRoutePickerButton` -- that type doesn't exist;
/// the real iOS AirPlay picker control is `AVRoutePickerView`, caught by
/// the first CI build failing on "cannot find type in scope".)
struct AirPlayButton: UIViewRepresentable {
    func makeUIView(context: Context) -> AVRoutePickerView {
        let view = AVRoutePickerView()
        view.tintColor = .white
        view.activeTintColor = Theme.accent.uiColor
        return view
    }

    func updateUIView(_ uiView: AVRoutePickerView, context: Context) {}
}

private extension Color {
    /// `AVRoutePickerView.activeTintColor` needs a `UIColor`, and `Theme`
    /// only deals in SwiftUI `Color` -- this is a one-off bridge, not a
    /// general-purpose conversion.
    var uiColor: UIColor {
        UIColor(self)
    }
}
