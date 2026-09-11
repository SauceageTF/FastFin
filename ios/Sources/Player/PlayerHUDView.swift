import SwiftUI

/// The custom overlay drawn on top of `PlayerViewControllerRepresentable`'s
/// raw video surface -- scrubber, transport controls, PiP/track buttons --
/// matching the approved mockup rather than the stock `AVPlayerViewController`
/// chrome, the same way the desktop app draws its own HUD over mpv instead
/// of using a bundled player UI.
struct PlayerHUDView: View {
    @ObservedObject var model: PlayerModel
    let title: String
    let subtitle: String?
    let onClose: () -> Void

    @State private var controlsVisible = true
    @State private var hideTask: Task<Void, Never>?
    @State private var isScrubbing = false
    @State private var scrubTime: Double = 0

    var body: some View {
        ZStack {
            Color.black.opacity(0.001) // full-area tap target
                .onTapGesture { toggleControls() }

            if controlsVisible {
                VStack {
                    topBar
                    Spacer()
                    centerControls
                    Spacer()
                    bottomBar
                }
                .padding(20)
                .background(
                    VStack(spacing: 0) {
                        LinearGradient(colors: [.black.opacity(0.65), .clear], startPoint: .top, endPoint: .bottom)
                            .frame(height: 90)
                        Spacer()
                        LinearGradient(colors: [.clear, .black.opacity(0.72)], startPoint: .top, endPoint: .bottom)
                            .frame(height: 110)
                    }
                    .ignoresSafeArea()
                )
                .transition(.opacity)
            }
        }
        .onAppear { scheduleAutoHide() }
        .onChange(of: model.isPlaying) { _, _ in scheduleAutoHide() }
    }

    private var topBar: some View {
        HStack(alignment: .center, spacing: 14) {
            Button(action: onClose) {
                Image(systemName: "chevron.left")
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundStyle(.white)
            }

            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.system(size: 14.5, weight: .bold)).foregroundStyle(.white)
                if let subtitle {
                    Text(subtitle).font(.system(size: 11)).foregroundStyle(.white.opacity(0.75))
                }
            }

            Spacer()

            if model.isPiPPossible {
                Button { model.requestPiPToggle?() } label: {
                    Image(systemName: "pip.enter")
                        .font(.system(size: 18))
                        .foregroundStyle(.white)
                }
            }

            AirPlayButton()
                .frame(width: 22, height: 22)
        }
    }

    private var centerControls: some View {
        HStack(spacing: 44) {
            skipButton(systemImage: "gobackward.10") { model.skip(by: -10) }

            Button { model.togglePlayPause() } label: {
                Image(systemName: model.isPlaying ? "pause.fill" : "play.fill")
                    .font(.system(size: 24))
                    .foregroundStyle(.white)
                    .frame(width: 60, height: 60)
                    .background(.ultraThinMaterial, in: Circle())
                    .overlay(Circle().strokeBorder(Color.white.opacity(0.16)))
            }

            skipButton(systemImage: "goforward.10") { model.skip(by: 10) }
        }
    }

    private func skipButton(systemImage: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: 22))
                .foregroundStyle(.white)
        }
    }

    private var bottomBar: some View {
        HStack(spacing: 12) {
            Text(timeLabel(displayTime))
                .font(.system(size: 11.5))
                .foregroundStyle(.white.opacity(0.9))
                .frame(width: 40, alignment: .leading)

            GeometryReader { proxy in
                let fraction = model.duration > 0 ? displayTime / model.duration : 0
                ZStack(alignment: .leading) {
                    Capsule().fill(Color.white.opacity(0.28)).frame(height: 3)
                    Capsule().fill(Theme.accent).frame(width: proxy.size.width * fraction, height: 3)
                    Circle().fill(.white)
                        .frame(width: 11, height: 11)
                        .offset(x: proxy.size.width * fraction - 5.5)
                }
                .frame(maxHeight: .infinity, alignment: .center)
                .contentShape(Rectangle())
                .gesture(
                    DragGesture(minimumDistance: 0)
                        .onChanged { value in
                            isScrubbing = true
                            let fraction = min(max(0, value.location.x / proxy.size.width), 1)
                            scrubTime = fraction * model.duration
                        }
                        .onEnded { _ in
                            model.seek(to: scrubTime)
                            isScrubbing = false
                        }
                )
            }
            .frame(height: 20)

            Text("-\(timeLabel(max(0, model.duration - displayTime)))")
                .font(.system(size: 11.5))
                .foregroundStyle(.white.opacity(0.9))
                .frame(width: 46, alignment: .trailing)
        }
    }

    private var displayTime: Double { isScrubbing ? scrubTime : model.currentTime }

    private func timeLabel(_ seconds: Double) -> String {
        guard seconds.isFinite, seconds >= 0 else { return "0:00" }
        let total = Int(seconds)
        let minutes = total / 60
        let secs = total % 60
        return String(format: "%d:%02d", minutes, secs)
    }

    private func toggleControls() {
        withAnimation(.easeInOut(duration: 0.2)) { controlsVisible.toggle() }
        if controlsVisible { scheduleAutoHide() }
    }

    private func scheduleAutoHide() {
        hideTask?.cancel()
        guard model.isPlaying else { return }
        hideTask = Task {
            try? await Task.sleep(for: .seconds(4))
            guard !Task.isCancelled else { return }
            withAnimation(.easeInOut(duration: 0.25)) { controlsVisible = false }
        }
    }
}
