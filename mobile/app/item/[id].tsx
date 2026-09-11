import { useEffect, useState } from "react";
import { View, Text, ScrollView, Pressable, ActivityIndicator, StyleSheet } from "react-native";
import { Image } from "expo-image";
import { LinearGradient } from "expo-linear-gradient";
import { BlurView } from "expo-blur";
import { Ionicons } from "@expo/vector-icons";
import { useLocalSearchParams, useRouter, Stack } from "expo-router";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { useSession } from "../../lib/session";
import { Theme, THEME_COLORS } from "../../lib/theme";
import { getItem, getSeasons, getEpisodes, getSimilarItems, getBackdropUrl, getLogoUrl, getImageUrl } from "../../lib/jellyfin";
import { backdropSourceId, episodeLabel, formatRuntime, hasLogo, type Item } from "../../lib/types";
import { CarouselRow } from "../../components/CarouselRow";
import { PosterCard } from "../../components/PosterCard";

export default function ItemDetailScreen() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const { session } = useSession();
  const router = useRouter();
  const insets = useSafeAreaInsets();

  const [item, setItem] = useState<Item | null>(null);
  const [seasons, setSeasons] = useState<Item[] | null>(null);
  const [nextEpisodes, setNextEpisodes] = useState<Item[] | null>(null);
  const [similarItems, setSimilarItems] = useState<Item[] | null>(null);

  useEffect(() => {
    if (!session) return;
    let cancelled = false;
    (async () => {
      const loaded = await getItem(session, id);
      if (cancelled) return;
      setItem(loaded);

      if (loaded.Type === "Series") {
        getSeasons(session, id).then((s) => !cancelled && setSeasons(s));
      } else if (loaded.Type === "Episode" && loaded.SeriesId && loaded.SeasonId) {
        getEpisodes(session, loaded.SeriesId, loaded.SeasonId).then((episodes) => {
          if (cancelled) return;
          setNextEpisodes(episodes.filter((ep) => ep.Id !== loaded.Id && (ep.IndexNumber ?? 0) > (loaded.IndexNumber ?? 0)));
        });
      }
      if (loaded.Type === "Movie" || loaded.Type === "Series") {
        getSimilarItems(session, id).then((s) => !cancelled && setSimilarItems(s));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [session, id]);

  if (!session) return null;

  if (!item) {
    return (
      <View style={styles.center}>
        <ActivityIndicator color={Theme.text} />
      </View>
    );
  }

  const backdropId = backdropSourceId(item) ?? item.Id;
  const playLabel = item.UserData?.PlaybackPositionTicks ? "Resume" : "Play";
  const played = item.UserData?.PlayedPercentage ?? 0;

  return (
    <View style={styles.root}>
      <Stack.Screen options={{ headerShown: false }} />
      <ScrollView contentContainerStyle={{ paddingBottom: 40 }}>
        <View style={styles.hero}>
          <Image source={{ uri: getBackdropUrl(session, backdropId) }} style={StyleSheet.absoluteFill} contentFit="cover" />
          <LinearGradient colors={["rgba(10,10,14,0.15)", "rgba(10,10,14,0.1)", Theme.background]} locations={[0, 0.45, 0.96]} style={StyleSheet.absoluteFill} />
          <Pressable onPress={() => router.back()} style={[styles.backButton, { top: insets.top + 6 }]}>
            <BlurView intensity={40} tint="dark" style={StyleSheet.absoluteFill} />
            <Ionicons name="chevron-back" size={18} color={Theme.text} />
          </Pressable>
        </View>

        <View style={styles.info}>
          {hasLogo(item) ? (
            <Image source={{ uri: getLogoUrl(session, item.Id) }} style={styles.logo} contentFit="contain" />
          ) : (
            <Text style={styles.title}>{item.Name}</Text>
          )}

          <View style={styles.metaRow}>
            {item.ProductionYear ? <Text style={styles.meta}>{item.ProductionYear}</Text> : null}
            {item.RunTimeTicks ? <Text style={styles.meta}>{formatRuntime(item.RunTimeTicks)}</Text> : null}
            {item.OfficialRating ? (
              <View style={styles.ratingBadge}>
                <Text style={styles.ratingBadgeText}>{item.OfficialRating}</Text>
              </View>
            ) : null}
            {item.CommunityRating ? (
              <Text style={styles.communityRating}>{"★"} {item.CommunityRating.toFixed(1)}</Text>
            ) : null}
          </View>

          {item.Type !== "Series" ? (
            <View style={{ marginBottom: 16 }}>
              <Pressable style={styles.playButton} onPress={() => router.push(`/player/${item.Id}`)}>
                <Ionicons name="play" size={16} color="#141018" />
                <Text style={styles.playButtonText}>{playLabel}</Text>
              </Pressable>
              {played > 0 ? (
                <View style={styles.progressTrack}>
                  <View style={[styles.progressFill, { width: `${played}%` }]} />
                </View>
              ) : null}
            </View>
          ) : null}

          {item.Overview ? (
            <Text style={styles.overview} numberOfLines={4}>
              {item.Overview}
            </Text>
          ) : null}

          {item.Genres && item.Genres.length > 0 ? (
            <View style={styles.genreRow}>
              {item.Genres.map((genre) => (
                <View key={genre} style={styles.genreChip}>
                  <Text style={styles.genreText}>{genre}</Text>
                </View>
              ))}
            </View>
          ) : null}
        </View>

        {item.Type === "Series" && seasons && seasons.length > 0 ? (
          <CarouselRow title="Seasons">
            {seasons.map((season) => (
              <PosterCard
                key={season.Id}
                title={season.Name}
                imageUrl={getImageUrl(session, season.Id)}
                onPress={() => router.push(`/season/${season.Id}?seriesId=${item.Id}&name=${encodeURIComponent(season.Name)}`)}
              />
            ))}
          </CarouselRow>
        ) : null}

        {item.Type === "Episode" && nextEpisodes && nextEpisodes.length > 0 ? (
          <CarouselRow title="Next Up">
            {nextEpisodes.map((episode) => (
              <PosterCard
                key={episode.Id}
                title={episodeLabel(episode)}
                imageUrl={getImageUrl(session, episode.Id)}
                width={156}
                onPress={() => router.push(`/player/${episode.Id}`)}
              />
            ))}
          </CarouselRow>
        ) : null}

        {(item.Type === "Movie" || item.Type === "Series") && similarItems && similarItems.length > 0 ? (
          <CarouselRow title="More Like This">
            {similarItems.map((similar) => (
              <PosterCard key={similar.Id} title={similar.Name} imageUrl={getImageUrl(session, similar.Id)} onPress={() => router.push(`/item/${similar.Id}`)} />
            ))}
          </CarouselRow>
        ) : null}
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: Theme.background },
  center: { flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: Theme.background },
  hero: { height: 320 },
  backButton: { position: "absolute", left: 18, width: 44, height: 44, borderRadius: 22, alignItems: "center", justifyContent: "center", overflow: "hidden" },
  info: { paddingHorizontal: 18, marginTop: -60 },
  logo: { width: 220, height: 76, marginBottom: 10 },
  title: { fontSize: 26, fontWeight: "800", color: Theme.text, marginBottom: 10 },
  metaRow: { flexDirection: "row", alignItems: "center", gap: 8, marginBottom: 16, flexWrap: "wrap" },
  meta: { fontSize: 12.5, fontWeight: "600", color: Theme.textDim },
  ratingBadge: { borderWidth: 1, borderColor: Theme.border, borderRadius: 4, paddingHorizontal: 7, paddingVertical: 2 },
  ratingBadgeText: { fontSize: 11, fontWeight: "700", color: Theme.textDim },
  communityRating: { fontSize: 11.5, fontWeight: "700", color: THEME_COLORS.ember.accent },
  playButton: { height: 46, borderRadius: 14, backgroundColor: Theme.text, flexDirection: "row", alignItems: "center", justifyContent: "center", gap: 8 },
  playButtonText: { fontSize: 14.5, fontWeight: "700", color: "#141018" },
  progressTrack: { height: 4, borderRadius: 2, backgroundColor: Theme.border, marginTop: 8, overflow: "hidden" },
  progressFill: { height: "100%", backgroundColor: THEME_COLORS.ember.accent },
  overview: { fontSize: 13.5, lineHeight: 20, color: "#c9c7d1", marginBottom: 16 },
  genreRow: { flexDirection: "row", flexWrap: "wrap", gap: 8, marginBottom: 20 },
  genreChip: { backgroundColor: Theme.backgroundElevated, borderWidth: 1, borderColor: Theme.border, borderRadius: 999, paddingHorizontal: 12, paddingVertical: 6 },
  genreText: { fontSize: 12, color: Theme.textDim },
});
