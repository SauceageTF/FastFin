import { useEffect, useState } from "react";
import { View, ActivityIndicator, Text, StyleSheet, Pressable, FlatList } from "react-native";
import { useLocalSearchParams, useRouter, Stack } from "expo-router";
import { Ionicons } from "@expo/vector-icons";
import { useSession } from "../../lib/session";
import { Theme } from "../../lib/theme";
import { getItems, getImageUrl } from "../../lib/jellyfin";
import type { Item } from "../../lib/types";
import { PosterCard } from "../../components/PosterCard";
import { useSafeAreaInsets } from "react-native-safe-area-context";

export default function LibraryGridScreen() {
  const { id, name } = useLocalSearchParams<{ id: string; name?: string }>();
  const { session } = useSession();
  const router = useRouter();
  const insets = useSafeAreaInsets();
  const [items, setItems] = useState<Item[] | null>(null);

  useEffect(() => {
    if (!session) return;
    getItems(session, id)
      .then(setItems)
      .catch(() => setItems([]));
  }, [session, id]);

  if (!session) return null;

  return (
    <View style={styles.root}>
      <Stack.Screen options={{ headerShown: false }} />
      <View style={[styles.header, { paddingTop: insets.top + 10 }]}>
        <Pressable onPress={() => router.back()} style={styles.backButton}>
          <Ionicons name="chevron-back" size={20} color={Theme.text} />
        </Pressable>
        <Text style={styles.headerTitle} numberOfLines={1}>
          {name ?? "Library"}
        </Text>
        <View style={{ width: 36 }} />
      </View>

      {!items ? (
        <View style={styles.center}>
          <ActivityIndicator color={Theme.text} />
        </View>
      ) : (
        <FlatList
          data={items}
          keyExtractor={(item) => item.Id}
          numColumns={3}
          columnWrapperStyle={{ gap: 10, paddingHorizontal: 18 }}
          contentContainerStyle={{ gap: 16, paddingBottom: 40 }}
          renderItem={({ item }) => (
            <PosterCard title={item.Name} imageUrl={getImageUrl(session, item.Id)} width={110} onPress={() => router.push(`/item/${item.Id}`)} />
          )}
          ListEmptyComponent={<Text style={styles.empty}>This library is empty.</Text>}
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
  empty: { color: Theme.textDim, textAlign: "center", marginTop: 40 },
});
