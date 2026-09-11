import JellyfinAPI
import SwiftUI

/// The poster grid for one library, mirrors `library/[id]/+page.svelte`.
struct LibraryGridView: View {
    let libraryID: String
    let libraryName: String

    @EnvironmentObject private var session: JellyfinSession
    @State private var items: [BaseItemDto]?
    @State private var errorMessage: String?

    private let columns = [GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10), GridItem(.flexible(), spacing: 10)]

    var body: some View {
        ZStack {
            Theme.background.ignoresSafeArea()

            if let items {
                if items.isEmpty {
                    Text("This library is empty.").foregroundStyle(Theme.textDim)
                } else {
                    ScrollView {
                        LazyVGrid(columns: columns, spacing: 16) {
                            ForEach(items, id: \.id) { item in
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
            } else if let errorMessage {
                Text(errorMessage).foregroundStyle(Theme.danger)
            } else {
                ProgressView().tint(Theme.text)
            }
        }
        .navigationTitle(libraryName)
        .navigationBarTitleDisplayMode(.inline)
        .task {
            do {
                items = try await MediaService.items(session: session, libraryID: libraryID)
            } catch {
                errorMessage = "Couldn't load this library: \(error.localizedDescription)"
            }
        }
    }
}
