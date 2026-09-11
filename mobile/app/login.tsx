import { useState } from "react";
import { View, Text, TextInput, Pressable, StyleSheet, ActivityIndicator, KeyboardAvoidingView, Platform, ScrollView } from "react-native";
import { LinearGradient } from "expo-linear-gradient";
import { BlurView } from "expo-blur";
import { Ionicons } from "@expo/vector-icons";
import { Theme } from "../lib/theme";
import { useSession } from "../lib/session";

export default function LoginScreen() {
  const { signIn, errorMessage } = useSession();
  const [serverUrl, setServerUrl] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [isSigningIn, setIsSigningIn] = useState(false);

  async function handleSignIn() {
    setIsSigningIn(true);
    await signIn(serverUrl, username, password);
    setIsSigningIn(false);
  }

  const canSubmit = serverUrl.length > 0 && username.length > 0 && !isSigningIn;

  return (
    <View style={styles.root}>
      <LinearGradient
        colors={["#b23a12", "#0a0a0e", "#0a0a0e"]}
        start={{ x: 1, y: 0 }}
        end={{ x: 0.3, y: 1 }}
        style={StyleSheet.absoluteFill}
      />

      <KeyboardAvoidingView behavior={Platform.OS === "ios" ? "padding" : undefined} style={{ flex: 1 }}>
        <ScrollView contentContainerStyle={styles.scrollContent} keyboardShouldPersistTaps="handled">
          <BlurView intensity={40} tint="dark" style={styles.card}>
            <LinearGradient colors={["#ff8a4c", "#ff5a1f"]} style={styles.logoMark}>
              <Ionicons name="play" size={20} color={Theme.text} />
            </LinearGradient>

            <Text style={styles.title}>FastFin</Text>
            <Text style={styles.subtitle}>Sign in to your Jellyfin server</Text>

            <Field label="Server" value={serverUrl} onChangeText={setServerUrl} placeholder="https://media.home.arpa" autoCapitalize="none" keyboardType="url" />
            <Field label="Username" value={username} onChangeText={setUsername} placeholder="Username" autoCapitalize="none" />
            <Field label="Password" value={password} onChangeText={setPassword} placeholder="Password" secureTextEntry />

            {errorMessage ? <Text style={styles.error}>{errorMessage}</Text> : null}

            <Pressable onPress={handleSignIn} disabled={!canSubmit} style={{ opacity: canSubmit ? 1 : 0.6 }}>
              <LinearGradient colors={["#ff8a4c", "#ff5a1f"]} style={styles.button}>
                {isSigningIn ? <ActivityIndicator color="#fff" /> : <Text style={styles.buttonText}>Sign In</Text>}
              </LinearGradient>
            </Pressable>
          </BlurView>
        </ScrollView>
      </KeyboardAvoidingView>
    </View>
  );
}

function Field(props: {
  label: string;
  value: string;
  onChangeText: (value: string) => void;
  placeholder: string;
  secureTextEntry?: boolean;
  autoCapitalize?: "none" | "sentences";
  keyboardType?: "default" | "url";
}) {
  return (
    <View style={{ marginBottom: 14 }}>
      <Text style={styles.fieldLabel}>{props.label.toUpperCase()}</Text>
      <TextInput
        style={styles.input}
        value={props.value}
        onChangeText={props.onChangeText}
        placeholder={props.placeholder}
        placeholderTextColor="rgba(255,255,255,0.35)"
        secureTextEntry={props.secureTextEntry}
        autoCapitalize={props.autoCapitalize ?? "sentences"}
        autoCorrect={false}
        keyboardType={props.keyboardType ?? "default"}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: Theme.background },
  scrollContent: { flexGrow: 1, alignItems: "center", justifyContent: "center", padding: 24 },
  card: {
    width: "100%",
    maxWidth: 360,
    borderRadius: 24,
    padding: 28,
    overflow: "hidden",
    borderWidth: 1,
    borderColor: "rgba(255,255,255,0.14)",
  },
  logoMark: { width: 52, height: 52, borderRadius: 16, alignItems: "center", justifyContent: "center", marginBottom: 14, alignSelf: "center" },
  title: { fontSize: 22, fontWeight: "800", color: Theme.text, textAlign: "center" },
  subtitle: { fontSize: 13, color: Theme.textDim, marginTop: 6, marginBottom: 24, textAlign: "center" },
  fieldLabel: { fontSize: 11, fontWeight: "600", color: Theme.textDim, letterSpacing: 0.6, marginBottom: 6, alignSelf: "flex-start" },
  input: {
    width: "100%",
    height: 44,
    borderRadius: 12,
    paddingHorizontal: 14,
    color: Theme.text,
    backgroundColor: "rgba(255,255,255,0.07)",
    borderWidth: 1,
    borderColor: "rgba(255,255,255,0.12)",
  },
  button: { width: "100%", height: 48, borderRadius: 14, alignItems: "center", justifyContent: "center", marginTop: 6 },
  buttonText: { fontSize: 15, fontWeight: "700", color: "#fff" },
  error: { color: Theme.danger, fontSize: 12.5, fontWeight: "600", textAlign: "center", marginTop: 4, marginBottom: 4 },
});
