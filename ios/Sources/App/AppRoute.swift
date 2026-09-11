import SwiftUI

/// Every push destination in the app, registered once with
/// `.navigationDestination(for: AppRoute.self)` at the root of each tab's
/// `NavigationStack`. A single `case item`/`case player` (rather than
/// pushing a bare `String` and guessing from context) is what keeps a
/// "Play" button from accidentally opening the detail page instead of the
/// player, and vice versa.
enum AppRoute: Hashable {
    case item(String)
    case season(seriesID: String, seasonID: String, seasonName: String)
    case player(String)
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
            case let .player(itemID):
                PlayerContainerView(itemID: itemID)
            case let .library(id, name):
                LibraryGridView(libraryID: id, libraryName: name)
            }
        }
    }
}
