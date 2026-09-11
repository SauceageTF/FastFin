import JellyfinAPI
import SwiftUI

/// Mirrors `item/[id]/+page.svelte`: backdrop hero, title-art logo in place
/// of plain text when the server has one, meta/overview/genres, then a
/// branch by type -- Seasons for a Series, Next Up for an Episode, and
/// More Like This for a Movie or Series.
struct ItemDetailView: View {
    let itemID: String

    @EnvironmentObject private var session: JellyfinSession
    @Environment(\.dismiss) private var dismiss
    @State private var item: BaseItemDto?
    @State private var seasons: [BaseItemDto]?
    @State private var nextEpisodes: [BaseItemDto]?
    @State private var similarItems: [BaseItemDto]?
    @State private var errorMessage: String?

    var body: some View {
        ZStack {
            Theme.background.ignoresSafeArea()

            if let item {
                ScrollView {
                    VStack(alignment: .leading, spacing: 22) {
                        hero(for: item)
                            .padding(.bottom, -140)

                        VStack(alignment: .leading, spacing: 16) {
                            titleBlock(for: item)
                                .border(Color.yellow, width: 2) // TEMP DIAGNOSTIC
                            metaRow(for: item)

                            if item.type != .series {
                                playButton(for: item)
                            }

                            if let overview = item.overview, !overview.isEmpty {
                                Text(overview)
                                    .font(.system(size: 13.5))
                                    .foregroundStyle(Color(hex: 0xc9c7d1))
                                    .lineLimit(4)
                                    .fixedSize(horizontal: false, vertical: true)
                            }

                            if let genres = item.genres, !genres.isEmpty {
                                genreRow(genres)
                            }

                            if let people = item.people, !people.isEmpty {
                                castRow(people)
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 18)
                        .zIndex(1)

                        if item.type == .series {
                            seasonsSection(for: item)
                        }

                        if item.type == .episode {
                            nextUpSection
                        }

                        if item.type == .movie || item.type == .series {
                            similarSection
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.bottom, 32)
                }
            } else if let errorMessage {
                Text(errorMessage).foregroundStyle(Theme.danger)
            } else {
                ProgressView().tint(Theme.text)
            }
        }
        .navigationBarHidden(true)
        .task { await load() }
    }

    // MARK: - Hero

    private func hero(for item: BaseItemDto) -> some View {
        // See HomeView.hero's comment: GeometryReader pins an explicit width
        // here rather than relying on frame(maxWidth: .infinity) to
        // propagate through the ZStack correctly.
        GeometryReader { geo in
            ZStack(alignment: .topLeading) {
                if let id = item.backdropSourceID, let url = MediaService.backdropURL(session: session, itemID: id) {
                    RemoteImage(url: url)
                } else {
                    Theme.backgroundElevated
                }

                LinearGradient(
                    colors: [Theme.background.opacity(0.15), Theme.background.opacity(0.1), Theme.background.opacity(0.96)],
                    startPoint: .top,
                    endPoint: .bottom
                )

                HStack {
                    GlassCircleButton(systemImage: "chevron.left") { dismiss() }
                    Spacer()
                }
                .padding(.horizontal, 18)
                .padding(.top, 6)
            }
            .frame(width: geo.size.width, height: 390)
            .clipped()
            .border(Color.green, width: 1) // TEMP DIAGNOSTIC
        }
        .frame(height: 390)
        .border(Color.red, width: 3) // TEMP DIAGNOSTIC
    }

    // MARK: - Title / meta

    @ViewBuilder
    private func titleBlock(for item: BaseItemDto) -> some View {
        if item.hasLogo, let logoURL = MediaService.logoURL(session: session, itemID: item.id ?? "") {
            AsyncImage(url: logoURL) { phase in
                if case let .success(image) = phase {
                    image.resizable().aspectRatio(contentMode: .fit)
                        .frame(maxWidth: 260, maxHeight: 90, alignment: .leading)
                } else {
                    fallbackTitle(item)
                }
            }
        } else {
            fallbackTitle(item)
        }
    }

    private func fallbackTitle(_ item: BaseItemDto) -> some View {
        Text(item.name ?? "")
            .font(Theme.displayFont(28, weight: .heavy))
            .foregroundStyle(Theme.text)
    }

    private func metaRow(for item: BaseItemDto) -> some View {
        HStack(spacing: 8) {
            if let year = item.productionYear { Text(String(year)) }
            if let runtime = item.formattedRuntime { Text(runtime) }
            if let rating = item.officialRating {
                Text(rating)
                    .padding(.horizontal, 7).padding(.vertical, 2)
                    .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(Theme.border))
                    .font(.system(size: 11, weight: .bold))
            }
            Spacer()
            if let community = item.communityRating {
                Text("\u{2605} \(String(format: "%.1f", community))")
                    .font(.system(size: 11.5, weight: .bold))
                    .foregroundStyle(Theme.accent)
                    .padding(.horizontal, 9).padding(.vertical, 3)
                    .background(Theme.accent.opacity(0.16))
                    .clipShape(RoundedRectangle(cornerRadius: 8))
            }
        }
        .font(.system(size: 12.5, weight: .semibold))
        .foregroundStyle(Theme.textDim)
    }

    private func playButton(for item: BaseItemDto) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            NavigationLink(value: AppRoute.player(item.id ?? "")) {
                HStack(spacing: 8) {
                    Image(systemName: "play.fill")
                    Text(item.userData?.playbackPositionTicks != nil ? "Resume" : "Play")
                        .font(Theme.displayFont(14.5, weight: .bold))
                }
                .frame(height: 46)
                .frame(maxWidth: .infinity)
            }
            .buttonStyle(.plain)
            .background(Theme.text)
            .foregroundStyle(Color(hex: 0x141018))
            .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))

            if let played = item.userData?.playedPercentage, played > 0 {
                GeometryReader { proxy in
                    ZStack(alignment: .leading) {
                        Capsule().fill(Theme.border)
                        Capsule().fill(Theme.accent).frame(width: proxy.size.width * played / 100)
                    }
                }
                .frame(height: 4)
            }
        }
    }

    private func genreRow(_ genres: [String]) -> some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(genres, id: \.self) { genre in
                    Text(genre)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(Theme.textDim)
                        .padding(.horizontal, 12).padding(.vertical, 6)
                        .background(Theme.backgroundElevated)
                        .overlay(Capsule().strokeBorder(Theme.border))
                        .clipShape(Capsule())
                }
            }
        }
    }

    private func castRow(_ people: [BaseItemPerson]) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Cast").font(Theme.displayFont(15, weight: .bold)).foregroundStyle(Theme.text)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(alignment: .top, spacing: 14) {
                    ForEach(Array(people.prefix(10).enumerated()), id: \.offset) { _, person in
                        VStack(spacing: 6) {
                            Circle()
                                .fill(Theme.backgroundElevated)
                                .frame(width: 44, height: 44)
                                .overlay(
                                    Text(initials(for: person.name))
                                        .font(.system(size: 14, weight: .bold))
                                        .foregroundStyle(Theme.textDim)
                                )
                            Text(person.name ?? "")
                                .font(.system(size: 9.5))
                                .foregroundStyle(Theme.textDim)
                                .lineLimit(1)
                        }
                        .frame(width: 52)
                    }
                }
            }
        }
    }

    private func initials(for name: String?) -> String {
        guard let name else { return "?" }
        let parts = name.split(separator: " ")
        return parts.prefix(2).compactMap(\.first).map(String.init).joined()
    }

    // MARK: - Seasons / Next Up / Similar

    private func seasonsSection(for item: BaseItemDto) -> some View {
        Group {
            if let seasons, !seasons.isEmpty {
                CarouselRow(title: "Seasons") {
                    ForEach(seasons, id: \.id) { season in
                        NavigationLink(
                            value: AppRoute.season(
                                seriesID: item.id ?? "",
                                seasonID: season.id ?? "",
                                seasonName: season.name ?? "Season"
                            )
                        ) {
                            PosterCard(
                                item: season,
                                imageURL: MediaService.imageURL(session: session, itemID: season.id ?? "")
                            )
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
    }

    private var nextUpSection: some View {
        Group {
            if let nextEpisodes, !nextEpisodes.isEmpty {
                CarouselRow(title: "Next Up") {
                    ForEach(nextEpisodes, id: \.id) { episode in
                        NavigationLink(value: AppRoute.player(episode.id ?? "")) {
                            EpisodeCard(
                                episode: episode,
                                imageURL: MediaService.imageURL(session: session, itemID: episode.id ?? "")
                            )
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
    }

    private var similarSection: some View {
        Group {
            if let similarItems, !similarItems.isEmpty {
                CarouselRow(title: "More Like This") {
                    ForEach(similarItems, id: \.id) { similar in
                        NavigationLink(value: AppRoute.item(similar.id ?? "")) {
                            PosterCard(
                                item: similar,
                                imageURL: MediaService.imageURL(session: session, itemID: similar.id ?? "")
                            )
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
    }

    // MARK: - Data

    private func load() async {
        do {
            let loaded = try await MediaService.item(session: session, id: itemID)
            item = loaded

            switch loaded.type {
            case .series:
                seasons = try? await MediaService.seasons(session: session, seriesID: itemID)
            case .episode:
                if let seriesID = loaded.seriesID, let seasonID = loaded.seasonID {
                    let seasonEpisodes = (try? await MediaService.episodes(session: session, seriesID: seriesID, seasonID: seasonID)) ?? []
                    nextEpisodes = seasonEpisodes.filter { $0.id != loaded.id && ($0.indexNumber ?? 0) > (loaded.indexNumber ?? 0) }
                }
            default:
                break
            }

            if loaded.type == .movie || loaded.type == .series {
                similarItems = try? await MediaService.similarItems(session: session, itemID: itemID)
            }
        } catch {
            errorMessage = "Couldn't load this title: \(error.localizedDescription)"
        }
    }
}
