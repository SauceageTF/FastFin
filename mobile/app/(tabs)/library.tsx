import { useEffect, useState } from "react";
import { View, Text, FlatList, Pressable, ActivityIndicator, StyleSheet } from "react-native";
import { useRouter } from "expo-router";
import { useSession } from "../../lib/session";
import { Theme } from "../../lib/theme";
import { getLibraries } from "../../lib/jellyfin";
import type { Library } from "../../lib/types";

export default function LibraryScreen() {
  const { session } = useSession();
  const router = useRouter();
  const [libraries, setLibraries] = useState<Library[] | null>(null);

  useEffect(() => {
    if (!session) return;
    getLibraries(session)
      .then(setLibraries)
      .catch(() => setLibraries([]));
  }, [session]);

  if (!session) return null;

  if (!libraries) {
    return (
      <View style={styles.center}>
        <ActivityIndicator color={Theme.text} />
      </View>
    );
  }

  return (
    <FlatList
      style={styles.root}
      contentContainerStyle={{ paddingTop: 70, paddingHorizontal: 18, paddingBottom: 110 }}
      data={libraries}
      keyExtractor={(library) => library.Id}
      renderItem={({ item }) => (
        <Pressable style={styles.row} onPress={() => router.push(`/library/${item.Id}?name=${encodeURIComponent(item.Name)}`)}>
          <Text style={styles.rowText}>{item.Name}</Text>
        </Pressable>
      )}
      ListEmptyComponent={<Text style={styles.empty}>No libraries found.</Text>}
    />
  );
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: Theme.background },
  center: { flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: Theme.background },
  row: { backgroundColor: Theme.backgroundElevated, borderRadius: 12, padding: 16, marginBottom: 10 },
  rowText: { fontSize: 16, fontWeight: "600", color: Theme.text },
  empty: { color: Theme.textDim, textAlign: "center", marginTop: 40 },
});
