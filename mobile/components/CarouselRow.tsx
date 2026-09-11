import { View, Text, ScrollView, StyleSheet } from "react-native";
import type { ReactNode } from "react";
import { Theme } from "../lib/theme";

export function CarouselRow({ title, onSeeAll, children }: { title: string; onSeeAll?: () => void; children: ReactNode }) {
  return (
    <View style={{ marginBottom: 4 }}>
      <View style={styles.header}>
        <Text style={styles.title}>{title}</Text>
        {onSeeAll ? <Text style={styles.seeAll} onPress={onSeeAll}>See all</Text> : null}
      </View>
      <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.row}>
        {children}
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  header: { flexDirection: "row", alignItems: "baseline", justifyContent: "space-between", paddingHorizontal: 18, marginBottom: 10 },
  title: { fontSize: 16, fontWeight: "700", color: Theme.text },
  seeAll: { fontSize: 11.5, fontWeight: "600", color: Theme.textDim },
  row: { flexDirection: "row", paddingHorizontal: 18, gap: 10 },
});
