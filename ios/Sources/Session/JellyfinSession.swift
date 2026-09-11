import Foundation
import JellyfinAPI
import UIKit

/// Owns the `JellyfinClient` instance and the signed-in state, mirroring
/// what `jellyfinClient.ts`'s `login` / `tryRestoreSession` / `logout` plus
/// the Rust-side `session_store.rs` do together on desktop -- except here
/// there's no IPC hop, this talks to the Jellyfin server directly.
@MainActor
final class JellyfinSession: ObservableObject {
    @Published private(set) var isAuthenticated = false
    @Published private(set) var isRestoring = true
    @Published var errorMessage: String?

    private(set) var client: JellyfinClient?
    private(set) var userID: String?

    private enum Key {
        static let serverURL = "serverURL"
        static let accessToken = "accessToken"
        static let userID = "userID"
        static let deviceID = "deviceID"
    }

    private lazy var deviceID: String = {
        if let existing = KeychainStore.get(Key.deviceID) { return existing }
        let generated = UUID().uuidString
        KeychainStore.set(generated, for: Key.deviceID)
        return generated
    }()

    /// Restores a previously signed-in session from the Keychain, if any.
    /// Call once at app launch; drives the splash -> login/home decision.
    func restore() async {
        defer { isRestoring = false }
        guard
            let serverURLString = KeychainStore.get(Key.serverURL),
            let serverURL = URL(string: serverURLString),
            let accessToken = KeychainStore.get(Key.accessToken),
            let userID = KeychainStore.get(Key.userID)
        else { return }

        client = makeClient(serverURL: serverURL, accessToken: accessToken)
        self.userID = userID
        isAuthenticated = true
    }

    func signIn(serverURLString: String, username: String, password: String) async {
        errorMessage = nil

        var trimmed = serverURLString.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            errorMessage = "Enter your server address."
            return
        }
        if !trimmed.contains("://") { trimmed = "https://\(trimmed)" }
        guard let serverURL = URL(string: trimmed) else {
            errorMessage = "That server address doesn't look right."
            return
        }

        let unauthenticatedClient = makeClient(serverURL: serverURL, accessToken: nil)
        do {
            let result = try await unauthenticatedClient.signIn(username: username, password: password)
            guard let accessToken = result.accessToken, let userID = result.user?.id else {
                errorMessage = "Sign in didn't return a session."
                return
            }
            KeychainStore.set(serverURL.absoluteString, for: Key.serverURL)
            KeychainStore.set(accessToken, for: Key.accessToken)
            KeychainStore.set(userID, for: Key.userID)

            client = makeClient(serverURL: serverURL, accessToken: accessToken)
            self.userID = userID
            isAuthenticated = true
        } catch {
            errorMessage = "Couldn't sign in: \(error.localizedDescription)"
        }
    }

    func signOut() async {
        try? await client?.signOut()
        KeychainStore.remove(Key.serverURL)
        KeychainStore.remove(Key.accessToken)
        KeychainStore.remove(Key.userID)
        client = nil
        userID = nil
        isAuthenticated = false
    }

    private func makeClient(serverURL: URL, accessToken: String?) -> JellyfinClient {
        JellyfinClient(configuration: .init(
            url: serverURL,
            accessToken: accessToken,
            client: "FastFin iOS",
            deviceName: UIDevice.current.name,
            deviceID: deviceID,
            version: "0.1.0"
        ))
    }
}
