import SwiftUI

struct PlayerViewControllerRepresentable: UIViewControllerRepresentable {
    let model: PlayerModel

    func makeUIViewController(context: Context) -> PlayerViewController {
        let controller = PlayerViewController(model: model)
        model.requestPiPToggle = { [weak controller] in controller?.togglePictureInPicture() }
        return controller
    }

    func updateUIViewController(_ uiViewController: PlayerViewController, context: Context) {}
}
