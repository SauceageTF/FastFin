import { View, Text, Pressable, StyleSheet } from "react-native";
import { Image } from "expo-image";
import { LinearGradient } from "expo-linear-gradient";
import { Ionicons } from "@expo/vector-icons";
import { BlurView } from "expo-blur";
import { useRouter } from "expo-router";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { Theme } from "../lib/theme";
import { getBackdropUrl, getLogoUrl } from "../lib/jellyfin";
import type { SessionInfo } from "../lib/session";
import { backdropSourceId, formatRuntime, hasLogo, type Item } from "../lib/types";

/** The featured hero at the top of Home. `titleItem` is the show itself when
 * `item` is an episode (an episode's own logo/name is almost never set --
 * showing its own episode title in place of the show's wordmark is exactly
 * wrong), while `item` still drives the backdrop and the play action. */
export function Hero({ item, titleItem, session }: { item: Item; titleItem: Item; session: SessionInfo }) {
  const router = useRouter();
  const insets = useSafeAreaInsets();
  const backdropId = backdropSourceId(item) ?? item.Id;

  function primaryAction() {
    if (item.Type === "Series") {
      router.push(`/item/${item.Id}`);
    } else {
      router.push(`/player/${item.Id}`);
    }
  }

  const primaryLabel = item.Type === "Series" ? "View Episodes" : item.UserData?.PlaybackPositionTicks ? "Resume" : "Play";

  return (
    <View style={styles.hero}>
      <Pressable style={StyleSheet.absoluteFill} onPress={primaryAction}>
        <Image source={{ uri: getBackdropUrl(session, backdropId) }} style={StyleSheet.absoluteFill} contentFit="cover" transition={250} />
      </Pressable>
      <LinearGradient colors={["transparent", Theme.background]} locations={[0.3, 1]} style={StyleSheet.absoluteFill} pointerEvents="none" />

      <View style={[styles.topBar, { top: insets.top + 6 }]} pointerEvents="box-none">
        <Text style={styles.brand}>FastFin</Text>
        <Pressable style={styles.iconButton}>
          <BlurView intensity={40} tint="dark" style={StyleSheet.absoluteFill} />
          <Ionicons name="search" size={18} color={Theme.text} />
        </Pressable>
      </View>

      <View style={styles.content} pointerEvents="box-none">
        {hasLogo(titleItem) ? (
          <Image source={{ uri: getLogoUrl(session, titleItem.Id) }} style={styles.logo} contentFit="contain" />
        ) : (
          <Text style={styles.title} numberOfLines={2}>
            {titleItem.Name}
          </Text>
        )}

        <View style={styles.metaRow}>
          {item.ProductionYear ? <Text style={styles.meta}>{item.ProductionYear}</Text> : null}
          {item.RunTimeTicks ? <Text style={styles.meta}>{formatRuntime(item.RunTimeTicks)}</Text> : null}
        </View>

        <View style={styles.buttonRow}>
          <Pressable style={styles.primaryButton} onPress={primaryAction}>
            <Ionicons name="play" size={14} color="#141018" />
            <Text style={styles.primaryButtonText}>{primaryLabel}</Text>
          </Pressable>
          <Pressable style={styles.infoButton} onPress={() => router.push(`/item/${item.Id}`)}>
            <BlurView intensity={40} tint="dark" style={StyleSheet.absoluteFill} />
            <Ionicons name="information-circle-outline" size={20} color={Theme.text} />
          </Pressable>
        </View>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  hero: { height: 480, overflow: "hidden" },
  topBar: { position: "absolute", left: 18, right: 18, flexDirection: "row", alignItems: "center", justifyContent: "space-between" },
  brand: { fontSize: 19, fontWeight: "800", color: Theme.text },
  iconButton: { width: 44, height: 44, borderRadius: 22, alignItems: "center", justifyContent: "center", overflow: "hidden" },
  content: { position: "absolute", left: 18, right: 18, bottom: 18 },
  logo: { width: 260, height: 90, marginBottom: 12 },
  title: { fontSize: 28, fontWeight: "800", color: Theme.text, marginBottom: 12 },
  metaRow: { flexDirection: "row", gap: 10, marginBottom: 18 },
  meta: { fontSize: 12, fontWeight: "600", color: "#c9c7d1" },
  buttonRow: { flexDirection: "row", gap: 10 },
  primaryButton: { flex: 1, height: 44, borderRadius: 13, backgroundColor: Theme.text, flexDirection: "row", alignItems: "center", justifyContent: "center", gap: 8 },
  primaryButtonText: { fontSize: 14, fontWeight: "700", color: "#141018" },
  infoButton: { width: 44, height: 44, borderRadius: 13, alignItems: "center", justifyContent: "center", overflow: "hidden" },
});
