import JellyfinAPI
import SwiftUI

/// The Search tab: a query field (native `.searchable`) plus a poster grid
/// of results across every library.
struct SearchView: View {
    @EnvironmentObject private var session: JellyfinSession
    @State private var query = ""
    @State private var results: [BaseItemDto] = []
    @State private var isSearching = false
    @State private var searchTask: Task<Void, Never>?

    private let columns = [GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10)]

    var body: some View {
        ZStack {
            Theme.background.ignoresSafeArea()

            if query.isEmpty {
                Text("Search your library").foregroundStyle(Theme.textDim)
            } else if results.isEmpty && !isSearching {
                Text("No results for \u{201C}\(query)\u{201D}").foregroundStyle(Theme.textDim)
            } else {
                ScrollView {
                    LazyVGrid(columns: columns, spacing: 16) {
                        ForEach(results, id: \.id) { item in
                            NavigationLink(value: AppRoute.item(item.id ?? "")) {
                                PosterCard(item: item, imageURL: MediaService.imageURL(session: session, itemID: item.id ?? ""), width: 112)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(18)
                }
            }
        }
        .navigationTitle("Search")
        .searchable(text: $query, placement: .navigationBarDrawer(displayMode: .always))
        .onChange(of: query) { _, newValue in
            searchTask?.cancel()
            guard !newValue.isEmpty else {
                results = []
                return
            }
            searchTask = Task {
                isSearching = true
                // Debounce -- avoid firing a request on every keystroke.
                try? await Task.sleep(for: .milliseconds(350))
                guard !Task.isCancelled else { return }
                results = (try? await MediaService.search(session: session, query: newValue)) ?? []
                isSearching = false
            }
        }
    }
}
