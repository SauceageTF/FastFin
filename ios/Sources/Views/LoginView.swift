import SwiftUI

struct LoginView: View {
    @EnvironmentObject private var session: JellyfinSession

    @State private var serverURL = ""
    @State private var username = ""
    @State private var password = ""
    @State private var isSigningIn = false
    @FocusState private var focusedField: Field?

    private enum Field { case server, username, password }

    var body: some View {
        ZStack {
            backdrop

            ScrollView {
                VStack(spacing: 0) {
                    Spacer(minLength: 120)
                    card
                    Spacer(minLength: 40)
                }
                .frame(maxWidth: .infinity, minHeight: UIScreen.main.bounds.height - 40)
            }
        }
        .scrollDismissesKeyboard(.interactively)
    }

    private var backdrop: some View {
        ZStack {
            Theme.background
            RadialGradient(
                colors: [Color(hex: 0xff8a4c), Color(hex: 0xb23a12), Theme.background],
                center: .topTrailing,
                startRadius: 20,
                endRadius: 700
            )
            .opacity(0.5)
            LinearGradient(
                colors: [Theme.background.opacity(0.55), Theme.background.opacity(0.9), Theme.background],
                startPoint: .top,
                endPoint: .bottom
            )
        }
        .ignoresSafeArea()
    }

    private var card: some View {
        VStack(spacing: 14) {
            RoundedRectangle(cornerRadius: 16, style: .continuous)
                .fill(LinearGradient(colors: [Color(hex: 0xff8a4c), Color(hex: 0xff5a1f)], startPoint: .topLeading, endPoint: .bottomTrailing))
                .frame(width: 52, height: 52)
                .overlay(Image(systemName: "play.fill").foregroundStyle(Theme.text))
                .padding(.bottom, 4)

            Text("FastFin")
                .font(Theme.displayFont(22, weight: .heavy))
                .foregroundStyle(Theme.text)

            Text("Sign in to your Jellyfin server")
                .font(.system(size: 13))
                .foregroundStyle(Theme.textDim)
                .padding(.bottom, 10)

            field(label: "Server", text: $serverURL, placeholder: "https://media.home.arpa", field: .server, keyboard: .URL)
            field(label: "Username", text: $username, placeholder: "Username", field: .username)
            secureField(label: "Password", text: $password, field: .password)

            if let error = session.errorMessage {
                Text(error)
                    .font(.system(size: 12.5, weight: .semibold))
                    .foregroundStyle(Theme.danger)
                    .multilineTextAlignment(.center)
                    .padding(.top, 2)
            }

            Button {
                signIn()
            } label: {
                ZStack {
                    if isSigningIn {
                        ProgressView().tint(.white)
                    } else {
                        Text("Sign In").font(Theme.displayFont(15, weight: .bold))
                    }
                }
                .frame(maxWidth: .infinity)
                .frame(height: 48)
            }
            .buttonStyle(.plain)
            .background(LinearGradient(colors: [Color(hex: 0xff8a4c), Color(hex: 0xff5a1f)], startPoint: .topLeading, endPoint: .bottomTrailing))
            .foregroundStyle(.white)
            .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
            .disabled(isSigningIn || serverURL.isEmpty || username.isEmpty)
            .opacity(isSigningIn || serverURL.isEmpty || username.isEmpty ? 0.6 : 1)
            .padding(.top, 6)
        }
        .padding(24)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 24, style: .continuous)
                .strokeBorder(Color.white.opacity(0.14), lineWidth: 1)
        )
        .padding(.horizontal, 24)
    }

    private func field(label: String, text: Binding<String>, placeholder: String, field: Field, keyboard: UIKeyboardType = .default) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label.uppercased())
                .font(.system(size: 11, weight: .semibold))
                .tracking(0.6)
                .foregroundStyle(Theme.textDim)

            TextField("", text: text, prompt: Text(placeholder).foregroundStyle(Color.white.opacity(0.4)))
                .focused($focusedField, equals: field)
                .keyboardType(keyboard)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .foregroundStyle(Theme.text)
                .frame(height: 44)
                .padding(.horizontal, 14)
                .background(Color.white.opacity(0.07))
                .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Color.white.opacity(0.12)))
                .clipShape(RoundedRectangle(cornerRadius: 12))
        }
    }

    private func secureField(label: String, text: Binding<String>, field: Field) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label.uppercased())
                .font(.system(size: 11, weight: .semibold))
                .tracking(0.6)
                .foregroundStyle(Theme.textDim)

            SecureField("", text: text, prompt: Text("Password").foregroundStyle(Color.white.opacity(0.4)))
                .focused($focusedField, equals: field)
                .foregroundStyle(Theme.text)
                .frame(height: 44)
                .padding(.horizontal, 14)
                .background(Color.white.opacity(0.07))
                .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Color.white.opacity(0.12)))
                .clipShape(RoundedRectangle(cornerRadius: 12))
        }
    }

    private func signIn() {
        focusedField = nil
        isSigningIn = true
        Task {
            await session.signIn(serverURLString: serverURL, username: username, password: password)
            isSigningIn = false
        }
    }
}
