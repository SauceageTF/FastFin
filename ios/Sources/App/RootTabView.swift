import SwiftUI

struct RootTabView: View {
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

            NavigationStack { PlaceholderScreen(title: "Library") }
                .tabItem { Label("Library", systemImage: "square.grid.2x2") }

            NavigationStack { PlaceholderScreen(title: "Search") }
                .tabItem { Label("Search", systemImage: "magnifyingglass") }

            NavigationStack { SettingsView() }
                .tabItem { Label("Settings", systemImage: "slider.horizontal.3") }
        }
        .tint(Theme.accent)
    }
}

/// Stand-in for tabs not built yet.
private struct PlaceholderScreen: View {
    let title: String

    var body: some View {
        ZStack {
            Theme.background.ignoresSafeArea()
            Text("\(title) is coming soon")
                .foregroundStyle(Theme.textDim)
        }
        .navigationTitle(title)
    }
}
