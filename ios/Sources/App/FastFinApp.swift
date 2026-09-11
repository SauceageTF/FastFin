import SwiftUI

@main
struct FastFinApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var session = JellyfinSession()
    @StateObject private var appearance = AppearanceStore.shared
    @StateObject private var playerPresenter = PlayerPresenter()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(session)
                .environmentObject(appearance)
                .environmentObject(playerPresenter)
                .preferredColorScheme(.dark)
                .task { await session.restore() }
        }
    }
}
