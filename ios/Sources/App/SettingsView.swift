import SwiftUI

/// Mirrors the theme-color swatches and sign-out action from the desktop
/// app's `Header.svelte` settings popover.
struct SettingsView: View {
    @EnvironmentObject private var session: JellyfinSession
    @EnvironmentObject private var appearance: AppearanceStore

    var body: some View {
        ZStack {
            Theme.background.ignoresSafeArea()

            List {
                Section {
                    HStack(spacing: 14) {
                        ForEach(ThemeColor.allCases, id: \.self) { color in
                            Button {
                                appearance.themeColor = color
                            } label: {
                                Circle()
                                    .fill(color.accent)
                                    .frame(width: 30, height: 30)
                                    .overlay(
                                        Circle()
                                            .strokeBorder(Theme.text, lineWidth: appearance.themeColor == color ? 2 : 0)
                                            .padding(-3)
                                    )
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    .padding(.vertical, 4)
                } header: {
                    Text("Theme Color")
                }
                .listRowBackground(Theme.backgroundElevated)

                Section {
                    Button(role: .destructive) {
                        Task { await session.signOut() }
                    } label: {
                        Text("Sign Out")
                    }
                }
                .listRowBackground(Theme.backgroundElevated)
            }
            .scrollContentBackground(.hidden)
        }
        .navigationTitle("Settings")
    }
}
