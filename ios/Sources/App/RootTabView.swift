import SwiftUI

struct RootTabView: View {
    @EnvironmentObject private var playerPresenter: PlayerPresenter

    init() {
        let appearance = UITabBarAppearance()
        appearance.configureWithTransparentBackground()
        appearance.backgroundEffect = UIBlurEffect(style: .systemUltraThinMaterialDark)
        UITabBar.appearance().standardAppearance = appearance
        UITabBar.appearance().scrollEdgeAppearance = appearance
    }

    var body: some View {
        TabView {
            NavigationStack { HomeView() }
                .tabItem { Label("Home", systemImage: "house.fill") }

            NavigationStack { LibraryView() }
                .tabItem { Label("Library", systemImage: "square.grid.2x2") }

            NavigationStack { SearchView() }
                .tabItem { Label("Search", systemImage: "magnifyingglass") }

            NavigationStack { SettingsView() }
                .tabItem { Label("Settings", systemImage: "slider.horizontal.3") }
        }
        .tint(Theme.accent)
        .fullScreenCover(item: $playerPresenter.request) { request in
            PlayerContainerView(itemID: request.id)
        }
    }
}
