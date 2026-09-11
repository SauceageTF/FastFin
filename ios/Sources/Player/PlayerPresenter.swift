import Foundation

struct PlayerRequest: Identifiable {
    let id: String
}

/// Lets any view trigger the player as a full-screen cover from a plain
/// button action, without each of Home/ItemDetail/SeasonEpisodes needing
/// its own `@State` + `.fullScreenCover`. One instance lives at the app
/// root; `RootTabView` is the only place that actually presents the cover.
@MainActor
final class PlayerPresenter: ObservableObject {
    @Published var request: PlayerRequest?

    func play(_ itemID: String) {
        request = PlayerRequest(id: itemID)
    }
}
