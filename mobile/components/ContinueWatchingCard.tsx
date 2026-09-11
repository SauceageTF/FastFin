import { View, Text, Pressable, StyleSheet } from "react-native";
import { Image } from "expo-image";
import { LinearGradient } from "expo-linear-gradient";
import { Theme, THEME_COLORS } from "../lib/theme";
import type { Item } from "../lib/types";
import { episodeLabel, formatRuntime } from "../lib/types";

export function ContinueWatchingCard({ item, imageUrl, onPress, width = 172 }: { item: Item; imageUrl: string; onPress: () => void; width?: number }) {
  const subtitle = item.SeriesName ? episodeLabel(item) : formatRuntime(item.RunTimeTicks);
  const played = item.UserData?.PlayedPercentage ?? 0;

  return (
    <Pressable onPress={onPress} style={[styles.card, { width, height: (width * 9) / 16 }]}>
      <Image source={{ uri: imageUrl }} style={StyleSheet.absoluteFill} contentFit="cover" transition={200} />
      <LinearGradient colors={["transparent", "rgba(0,0,0,0.78)"]} start={{ x: 0.5, y: 0.35 }} end={{ x: 0.5, y: 1 }} style={StyleSheet.absoluteFill} />
      <View style={styles.textBlock}>
        <Text style={styles.name} numberOfLines={1}>
          {item.SeriesName ?? item.Name}
        </Text>
        <Text style={styles.subtitle} numberOfLines={1}>
          {subtitle}
        </Text>
        <View style={styles.progressTrack}>
          <View style={[styles.progressFill, { width: `${played}%` }]} />
        </View>
      </View>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  card: { borderRadius: 14, overflow: "hidden", backgroundColor: Theme.backgroundElevated },
  textBlock: { position: "absolute", left: 10, right: 10, bottom: 8 },
  name: { fontSize: 12.5, fontWeight: "600", color: Theme.text },
  subtitle: { fontSize: 10.5, color: "#c9c7d1", marginTop: 2, marginBottom: 5 },
  progressTrack: { height: 2.5, borderRadius: 2, backgroundColor: "rgba(255,255,255,0.22)" },
  progressFill: { height: "100%", borderRadius: 2, backgroundColor: THEME_COLORS.ember.accent },
});
