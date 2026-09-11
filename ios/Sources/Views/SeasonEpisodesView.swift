import JellyfinAPI
import SwiftUI

/// Episode list for one season, pushed from `ItemDetailView`'s Seasons row
/// -- mirrors `item/[id]/season/[seasonId]/+page.svelte`.
struct SeasonEpisodesView: View {
    let seriesID: String
    let seasonID: String
    let seasonName: String

    @EnvironmentObject private var session: JellyfinSession
    @EnvironmentObject private var playerPresenter: PlayerPresenter
    @State private var episodes: [BaseItemDto]?

    var body: some View {
        ZStack {
            Theme.background.ignoresSafeArea()

            ScrollView {
                LazyVStack(spacing: 10) {
                    if let episodes {
                        ForEach(episodes, id: \.id) { episode in
                            Button { playerPresenter.play(episode.id ?? "") } label: {
                                row(for: episode)
                            }
                            .buttonStyle(.plain)
                        }
                    } else {
                        ProgressView().tint(Theme.text).padding(.top, 40)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(18)
            }
        }
        .navigationTitle(seasonName)
        .navigationBarTitleDisplayMode(.inline)
        .task { episodes = (try? await MediaService.episodes(session: session, seriesID: seriesID, seasonID: seasonID)) ?? [] }
    }

    private func row(for episode: BaseItemDto) -> some View {
        HStack(spacing: 12) {
            RemoteImage(url: MediaService.imageURL(session: session, itemID: episode.id ?? ""))
                .frame(width: 92, height: 52)
                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))

            VStack(alignment: .leading, spacing: 4) {
                Text(episode.episodeLabel ?? episode.name ?? "")
                    .font(.system(size: 13.5, weight: .semibold))
                    .foregroundStyle(Theme.text)
                    .lineLimit(1)
                if let runtime = episode.formattedRuntime {
                    Text(runtime).font(.system(size: 11.5)).foregroundStyle(Theme.textDim)
                }
            }

            Spacer()
            Image(systemName: "play.fill").foregroundStyle(Theme.textDim)
        }
        .padding(10)
        .background(Theme.backgroundElevated)
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
    }
}
