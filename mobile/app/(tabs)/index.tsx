import { useEffect, useState, useCallback } from "react";
import { View, Text, ScrollView, ActivityIndicator, StyleSheet } from "react-native";
import { useRouter } from "expo-router";
import { useSession } from "../../lib/session";
import { Theme } from "../../lib/theme";
import { getLibraries, getLatestItems, getResumeItems, getItem, getImageUrl } from "../../lib/jellyfin";
import type { Item, Library } from "../../lib/types";
import { Hero } from "../../components/Hero";
import { CarouselRow } from "../../components/CarouselRow";
import { ContinueWatchingCard } from "../../components/ContinueWatchingCard";
import { PosterCard } from "../../components/PosterCard";

export default function HomeScreen() {
  const { session } = useSession();
  const router = useRouter();

  const [libraries, setLibraries] = useState<Library[]>([]);
  const [libraryItems, setLibraryItems] = useState<Record<string, Item[]>>({});
  const [continueWatching, setContinueWatching] = useState<Item[]>([]);
  const [featured, setFeatured] = useState<Item | null>(null);
  const [heroTitleItem, setHeroTitleItem] = useState<Item | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!session) return;
    try {
      const [libs, resume] = await Promise.all([getLibraries(session), getResumeItems(session)]);
      setLibraries(libs);
      setContinueWatching(resume);

      let feat: Item | null = resume[0] ?? null;
      const itemsMap: Record<string, Item[]> = {};
      for (const lib of libs) {
        const items = await getLatestItems(session, lib.Id).catch(() => []);
        itemsMap[lib.Id] = items;
        if (!feat && items[0]) feat = items[0];
      }
      setLibraryItems(itemsMap);
      setFeatured(feat);

      if (feat?.Type === "Episode" && feat.SeriesId) {
        getItem(session, feat.SeriesId).then(setHeroTitleItem).catch(() => {});
      }
    } catch (error) {
      setErrorMessage(`Couldn't load your library: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setIsLoading(false);
    }
  }, [session]);

  useEffect(() => {
    load();
  }, [load]);

  if (!session) return null;

  if (isLoading && !featured) {
    return (
      <View style={styles.center}>
        <ActivityIndicator color={Theme.text} />
      </View>
    );
  }

  if (errorMessage && !featured) {
    return (
      <View style={styles.center}>
        <Text style={styles.error}>{errorMessage}</Text>
      </View>
    );
  }

  return (
    <View style={styles.root}>
      <ScrollView contentContainerStyle={{ paddingBottom: 110 }}>
        {featured ? <Hero item={featured} titleItem={heroTitleItem ?? featured} session={session} /> : null}

        <View style={{ paddingTop: 20, gap: 24 }}>
          {continueWatching.length > 0 ? (
            <CarouselRow title="Continue Watching">
              {continueWatching.map((item) => (
                <ContinueWatchingCard
                  key={item.Id}
                  item={item}
                  imageUrl={getImageUrl(session, item.Id)}
                  onPress={() => router.push(`/player/${item.Id}`)}
                />
              ))}
            </CarouselRow>
          ) : null}

          {libraries.map((library) =>
            libraryItems[library.Id]?.length ? (
              <CarouselRow key={library.Id} title={`Recently Added in ${library.Name}`}>
                {libraryItems[library.Id].map((item) => (
                  <PosterCard
                    key={item.Id}
                    title={item.Name}
                    imageUrl={getImageUrl(session, item.Id)}
                    onPress={() => router.push(`/item/${item.Id}`)}
                  />
                ))}
              </CarouselRow>
            ) : null
          )}
        </View>
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: Theme.background },
  center: { flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: Theme.background, padding: 32 },
  error: { color: Theme.danger, textAlign: "center" },
});
