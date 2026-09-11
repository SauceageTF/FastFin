import SwiftUI

struct RootView: View {
    @EnvironmentObject private var session: JellyfinSession

    var body: some View {
        Group {
            if session.isRestoring {
                SplashView()
            } else if session.isAuthenticated {
                RootTabView()
            } else {
                LoginView()
            }
        }
        .animation(.easeInOut(duration: 0.2), value: session.isAuthenticated)
    }
}

private struct SplashView: View {
    var body: some View {
        ZStack {
            Theme.background.ignoresSafeArea()
            Text("FastFin")
                .font(Theme.displayFont(26, weight: .heavy))
                .foregroundStyle(Theme.text)
        }
    }
}
