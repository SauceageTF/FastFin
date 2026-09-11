import { useState, useEffect, useRef } from "react";
import { View, Text, TextInput, FlatList, StyleSheet } from "react-native";
import { Ionicons } from "@expo/vector-icons";
import { useRouter } from "expo-router";
import { useSession } from "../../lib/session";
import { Theme } from "../../lib/theme";
import { search, getImageUrl } from "../../lib/jellyfin";
import type { Item } from "../../lib/types";
import { PosterCard } from "../../components/PosterCard";

export default function SearchScreen() {
  const { session } = useSession();
  const router = useRouter();
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Item[]>([]);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (!session || !query) {
      setResults([]);
      return;
    }
    debounceRef.current = setTimeout(() => {
      search(session, query).then(setResults).catch(() => setResults([]));
    }, 350);
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [query, session]);

  if (!session) return null;

  return (
    <View style={styles.root}>
      <View style={styles.searchBar}>
        <Ionicons name="search" size={16} color={Theme.textDim} />
        <TextInput
          style={styles.input}
          value={query}
          onChangeText={setQuery}
          placeholder="Search your library"
          placeholderTextColor={Theme.textDim}
          autoCapitalize="none"
          autoCorrect={false}
        />
      </View>

      <FlatList
        data={results}
        keyExtractor={(item) => item.Id}
        numColumns={3}
        columnWrapperStyle={{ gap: 10, paddingHorizontal: 18 }}
        contentContainerStyle={{ gap: 16, paddingBottom: 110 }}
        renderItem={({ item }) => (
          <PosterCard
            title={item.Name}
            imageUrl={getImageUrl(session, item.Id)}
            width={112}
            onPress={() => router.push(`/item/${item.Id}`)}
          />
        )}
        ListEmptyComponent={
          query ? <Text style={styles.empty}>No results for &ldquo;{query}&rdquo;</Text> : null
        }
      />
    </View>
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: Theme.background, paddingTop: 60 },
  searchBar: {
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
    marginHorizontal: 18,
    marginBottom: 18,
    height: 40,
    borderRadius: 10,
    paddingHorizontal: 12,
    backgroundColor: Theme.backgroundElevated,
  },
  input: { flex: 1, color: Theme.text, fontSize: 15 },
  empty: { color: Theme.textDim, textAlign: "center", marginTop: 40 },
});
