import JellyfinAPI
import SwiftUI

/// Mirrors `library/+page.svelte`: a featured hero (resume item, or the
/// first library's newest item) followed by a Continue Watching carousel
/// and one "Recently Added in {Library}" carousel per library.
struct HomeView: View {
    @EnvironmentObject private var session: JellyfinSession

    @State private var libraries: [BaseItemDto] = []
    @State private var libraryItems: [String: [BaseItemDto]] = [:]
    @State private var continueWatching: [BaseItemDto] = []
    @State private var featured: BaseItemDto?
    @State private var isLoading = true
    @State private var errorMessage: String?

    var body: some View {
        ZStack {
            Theme.background.ignoresSafeArea()

            ScrollView {
                VStack(alignment: .leading, spacing: 26) {
                    hero
                        .frame(maxWidth: .infinity, alignment: .leading)

                    if !continueWatching.isEmpty {
                        CarouselRow(title: "Continue Watching") {
                            ForEach(continueWatching, id: \.id) { item in
                                NavigationLink(value: AppRoute.item(item.id ?? "")) {
                                    ContinueWatchingCard(item: item, imageURL: imageURL(for: item))
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }

                    ForEach(libraries, id: \.id) { library in
                        if let items = libraryItems[library.id ?? ""], !items.isEmpty {
                            CarouselRow(title: "Recently Added in \(library.name ?? "Library")") {
                                ForEach(items, id: \.id) { item in
                                    NavigationLink(value: AppRoute.item(item.id ?? "")) {
                                        PosterCard(item: item, imageURL: imageURL(for: item))
                                    }
                                    .buttonStyle(.plain)
                                }
                            }
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.bottom, 32)
            }

            if isLoading && featured == nil {
                ProgressView().tint(Theme.text)
            } else if let errorMessage, featured == nil {
                Text(errorMessage)
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.danger)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 32)
            }
        }
        .navigationBarHidden(true)
        .appRouteDestinations()
        .task { await load() }
    }

    // MARK: - Hero

    @ViewBuilder
    private var hero: some View {
        // GeometryReader pins an explicit, unambiguous width for everything
        // inside instead of relying on flexible frame(maxWidth: .infinity)
        // propagating correctly through the ZStack -- that propagation is
        // what fixed the carousels below, but visibly didn't resolve here,
        // so this pins it directly rather than guessing at a second
        // flexible-layout fix.
        GeometryReader { geo in
            ZStack(alignment: .bottomLeading) {
                if let featured, let backdrop = MediaService.backdropURL(session: session, itemID: featured.backdropSourceID ?? featured.id ?? "") {
                    RemoteImage(url: backdrop)
                } else {
                    Theme.backgroundElevated
                }

                LinearGradient(colors: [.clear, Theme.background], startPoint: .init(x: 0.5, y: 0.3), endPoint: .bottom)

                topBar
                    .padding(.horizontal, 18)
                    .padding(.top, 6)
                    .frame(maxHeight: .infinity, alignment: .top)

                if let featured {
                    heroContent(for: featured)
                        .padding(18)
                }
            }
            .frame(width: geo.size.width, height: 480)
            .clipped()
        }
        .frame(height: 480)
    }

    private var topBar: some View {
        HStack {
            Text("FastFin")
                .font(Theme.displayFont(19, weight: .heavy))
                .foregroundStyle(Theme.text)
            Spacer()
            GlassCircleButton(systemImage: "magnifyingglass")
        }
    }

    @ViewBuilder
    private func heroContent(for item: BaseItemDto) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            if item.hasLogo, let logoURL = MediaService.logoURL(session: session, itemID: item.id ?? "") {
                AsyncImage(url: logoURL) { phase in
                    if case let .success(image) = phase {
                        image.resizable().aspectRatio(contentMode: .fit)
                            .frame(maxWidth: 260, maxHeight: 90, alignment: .leading)
                    } else {
                        EmptyView()
                    }
                }
            } else {
                Text(item.name ?? "")
                    .font(Theme.displayFont(28, weight: .heavy))
                    .foregroundStyle(Theme.text)
                    .lineLimit(2)
            }

            HStack(spacing: 10) {
                if let year = item.productionYear {
                    Text(String(year))
                }
                if let runtime = item.formattedRuntime {
                    Text(runtime)
                }
            }
            .font(.system(size: 12, weight: .semibold))
            .foregroundStyle(Color(hex: 0xc9c7d1))

            HStack(spacing: 10) {
                // Series aren't directly playable -- send that tap to the
                // detail page to pick a season/episode instead, matching
                // library/+page.svelte's "View Episodes" vs "Play" hero button.
                NavigationLink(value: item.type == .series ? AppRoute.item(item.id ?? "") : AppRoute.player(item.id ?? "")) {
                    HStack(spacing: 8) {
                        Image(systemName: "play.fill")
                        Text(item.type == .series ? "View Episodes" : (item.userData?.playbackPositionTicks != nil ? "Resume" : "Play"))
                            .font(Theme.displayFont(14, weight: .bold))
                    }
                    .frame(maxWidth: .infinity)
                    .frame(height: 44)
                }
                .buttonStyle(.plain)
                .background(Theme.text)
                .foregroundStyle(Color(hex: 0x141018))
                .clipShape(RoundedRectangle(cornerRadius: 13, style: .continuous))

                NavigationLink(value: AppRoute.item(item.id ?? "")) {
                    Image(systemName: "info.circle")
                }
                .buttonStyle(.plain)
                .frame(width: 44, height: 44)
                .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 13, style: .continuous))
                .foregroundStyle(Theme.text)
            }
        }
    }

    // MARK: - Data

    private func imageURL(for item: BaseItemDto) -> URL? {
        MediaService.imageURL(session: session, itemID: item.id ?? "")
    }

    private func load() async {
        do {
            async let librariesTask = MediaService.libraries(session: session)
            async let resumeTask = MediaService.resumeItems(session: session)
            let (loadedLibraries, resumeItems) = try await (librariesTask, resumeTask)

            libraries = loadedLibraries
            continueWatching = resumeItems

            if let first = resumeItems.first {
                featured = first
            }

            for library in loadedLibraries {
                guard let id = library.id else { continue }
                let items = (try? await MediaService.latestItems(session: session, libraryID: id)) ?? []
                libraryItems[id] = items
                if featured == nil, let first = items.first {
                    featured = first
                }
            }
        } catch {
            errorMessage = "Couldn't load your library: \(error.localizedDescription)"
        }
        isLoading = false
    }
}
