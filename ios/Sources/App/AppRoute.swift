import Foundation

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
}
