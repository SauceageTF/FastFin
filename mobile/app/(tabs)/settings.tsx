import { View, Text, Pressable, StyleSheet } from "react-native";
import { Theme, THEME_COLORS, type ThemeColorName } from "../../lib/theme";
import { useSession } from "../../lib/session";
import { useState } from "react";

export default function SettingsScreen() {
  const { signOut } = useSession();
  const [themeColor, setThemeColor] = useState<ThemeColorName>("ember");

  return (
    <View style={styles.root}>
      <Text style={styles.sectionLabel}>THEME COLOR</Text>
      <View style={styles.swatches}>
        {(Object.keys(THEME_COLORS) as ThemeColorName[]).map((name) => (
          <Pressable key={name} onPress={() => setThemeColor(name)} style={styles.swatchWrap}>
            <View
              style={[
                styles.swatch,
                { backgroundColor: THEME_COLORS[name].accent },
                themeColor === name && styles.swatchSelected,
              ]}
            />
          </Pressable>
        ))}
      </View>

      <Pressable style={styles.signOut} onPress={signOut}>
        <Text style={styles.signOutText}>Sign Out</Text>
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: Theme.background, padding: 20, paddingTop: 70 },
  sectionLabel: { fontSize: 12, fontWeight: "700", color: Theme.textDim, letterSpacing: 0.6, marginBottom: 14 },
  swatches: { flexDirection: "row", gap: 14, marginBottom: 36 },
  swatchWrap: { padding: 3 },
  swatch: { width: 30, height: 30, borderRadius: 15 },
  swatchSelected: { borderWidth: 2, borderColor: Theme.text },
  signOut: { backgroundColor: Theme.backgroundElevated, borderRadius: 12, paddingVertical: 14, alignItems: "center" },
  signOutText: { color: Theme.danger, fontWeight: "700", fontSize: 15 },
});
