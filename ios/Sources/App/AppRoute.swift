import SwiftUI

/// Every push destination in the app, registered once with
/// `.navigationDestination(for: AppRoute.self)` at the root of each tab's
/// `NavigationStack`. The player is deliberately NOT a case here -- it's
/// presented as a full-screen cover via `PlayerPresenter` instead of pushed,
/// so it fully takes over the screen with no nav chrome underneath it.
enum AppRoute: Hashable {
    case item(String)
    case season(seriesID: String, seasonID: String, seasonName: String)
    case library(id: String, name: String)
}

extension View {
    /// Every tab's `NavigationStack` wires its pushes through this same
    /// switch, since each tab (Home, Library, Search) is its own stack with
    /// its own destination registration -- sharing this keeps them from
    /// drifting out of sync as routes are added.
    @ViewBuilder
    func appRouteDestinations() -> some View {
        navigationDestination(for: AppRoute.self) { route in
            switch route {
            case let .item(itemID):
                ItemDetailView(itemID: itemID)
            case let .season(seriesID, seasonID, seasonName):
                SeasonEpisodesView(seriesID: seriesID, seasonID: seasonID, seasonName: seasonName)
            case let .library(id, name):
                LibraryGridView(libraryID: id, libraryName: name)
            }
        }
    }
}
