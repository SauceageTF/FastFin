import { useEffect, useState } from "react";
import { View, Text, FlatList, Pressable, ActivityIndicator, StyleSheet } from "react-native";
import { Image } from "expo-image";
import { Ionicons } from "@expo/vector-icons";
import { useLocalSearchParams, useRouter, Stack } from "expo-router";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { useSession } from "../../lib/session";
import { Theme } from "../../lib/theme";
import { getEpisodes, getImageUrl } from "../../lib/jellyfin";
import { episodeLabel, formatRuntime, type Item } from "../../lib/types";

export default function SeasonEpisodesScreen() {
  const { id, seriesId, name } = useLocalSearchParams<{ id: string; seriesId: string; name?: string }>();
  const { session } = useSession();
  const router = useRouter();
  const insets = useSafeAreaInsets();
  const [episodes, setEpisodes] = useState<Item[] | null>(null);

  useEffect(() => {
    if (!session) return;
    getEpisodes(session, seriesId, id)
      .then(setEpisodes)
      .catch(() => setEpisodes([]));
  }, [session, seriesId, id]);

  if (!session) return null;

  return (
    <View style={styles.root}>
      <Stack.Screen options={{ headerShown: false }} />
      <View style={[styles.header, { paddingTop: insets.top + 10 }]}>
        <Pressable onPress={() => router.back()} style={styles.backButton}>
          <Ionicons name="chevron-back" size={20} color={Theme.text} />
        </Pressable>
        <Text style={styles.headerTitle} numberOfLines={1}>
          {name ?? "Season"}
        </Text>
        <View style={{ width: 36 }} />
      </View>

      {!episodes ? (
        <View style={styles.center}>
          <ActivityIndicator color={Theme.text} />
        </View>
      ) : (
        <FlatList
          data={episodes}
          keyExtractor={(episode) => episode.Id}
          contentContainerStyle={{ paddingHorizontal: 18, gap: 10, paddingBottom: 40 }}
          renderItem={({ item: episode }) => (
            <Pressable style={styles.row} onPress={() => router.push(`/player/${episode.Id}`)}>
              <Image source={{ uri: getImageUrl(session, episode.Id) }} style={styles.thumb} contentFit="cover" />
              <View style={{ flex: 1 }}>
                <Text style={styles.name} numberOfLines={1}>
                  {episodeLabel(episode)}
                </Text>
                {episode.RunTimeTicks ? <Text style={styles.runtime}>{formatRuntime(episode.RunTimeTicks)}</Text> : null}
              </View>
              <Ionicons name="play" size={18} color={Theme.textDim} />
            </Pressable>
          )}
        />
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: Theme.background },
  center: { flex: 1, alignItems: "center", justifyContent: "center" },
  header: { flexDirection: "row", alignItems: "center", justifyContent: "space-between", paddingHorizontal: 12, paddingBottom: 12 },
  backButton: { width: 36, height: 36, alignItems: "center", justifyContent: "center" },
  headerTitle: { flex: 1, textAlign: "center", fontSize: 16, fontWeight: "700", color: Theme.text },
  row: { flexDirection: "row", alignItems: "center", gap: 12, backgroundColor: Theme.backgroundElevated, borderRadius: 12, padding: 10 },
  thumb: { width: 92, height: 52, borderRadius: 8, backgroundColor: Theme.background },
  name: { fontSize: 13.5, fontWeight: "600", color: Theme.text },
  runtime: { fontSize: 11.5, color: Theme.textDim, marginTop: 4 },
});
