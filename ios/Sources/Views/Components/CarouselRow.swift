import SwiftUI

/// A titled horizontal-scroll row -- one per library on the home screen
/// ("Recently Added in {Library}"), matching `library/+page.svelte`'s
/// `.row-header` + `Carousel.svelte` pattern. The native `ScrollView` gives
/// the swipe/bounce/peek behavior `Carousel.svelte` has to hand-roll with
/// arrow buttons on the web.
struct CarouselRow<Content: View>: View {
    let title: String
    var seeAllAction: (() -> Void)?
    @ViewBuilder var content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline) {
                Text(title)
                    .font(Theme.displayFont(16, weight: .bold))
                    .foregroundStyle(Theme.text)

                Spacer()

                if let seeAllAction {
                    Button("See all", action: seeAllAction)
                        .font(.system(size: 11.5, weight: .semibold))
                        .foregroundStyle(Theme.textDim)
                }
            }
            .padding(.horizontal, 18)

            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 10) {
                    content()
                }
                .padding(.horizontal, 18)
            }
        }
    }
}
