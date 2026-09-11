import JellyfinAPI
import SwiftUI

/// The Library tab's root: a list of the user's libraries. Tapping one
/// pushes `LibraryGridView`, mirroring `library/+page.svelte`'s "See all"
/// links through to `library/[id]/+page.svelte`.
struct LibraryView: View {
    @EnvironmentObject private var session: JellyfinSession
    @State private var libraries: [BaseItemDto]?
    @State private var errorMessage: String?

    var body: some View {
        ZStack {
            Theme.background.ignoresSafeArea()

            if let libraries {
                if libraries.isEmpty {
                    Text("No libraries found.").foregroundStyle(Theme.textDim)
                } else {
                    List(libraries, id: \.id) { library in
                        NavigationLink(value: AppRoute.library(id: library.id ?? "", name: library.name ?? "Library")) {
                            Text(library.name ?? "Library")
                                .font(.system(size: 16, weight: .semibold))
                                .foregroundStyle(Theme.text)
                        }
                        .listRowBackground(Theme.backgroundElevated)
                    }
                    .scrollContentBackground(.hidden)
                }
            } else if let errorMessage {
                Text(errorMessage).foregroundStyle(Theme.danger)
            } else {
                ProgressView().tint(Theme.text)
            }
        }
        .navigationTitle("Library")
        .appRouteDestinations()
        .task {
            do {
                libraries = try await MediaService.libraries(session: session)
            } catch {
                errorMessage = "Couldn't load libraries: \(error.localizedDescription)"
            }
        }
    }
}
