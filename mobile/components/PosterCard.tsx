import { View, Text, Pressable, StyleSheet } from "react-native";
import { Image } from "expo-image";
import { Theme } from "../lib/theme";

export function PosterCard({ title, imageUrl, width = 104, onPress }: { title: string; imageUrl: string; width?: number; onPress: () => void }) {
  return (
    <Pressable onPress={onPress} style={{ width }}>
      <Image source={{ uri: imageUrl }} style={[styles.poster, { width, height: width * 1.5 }]} contentFit="cover" transition={200} />
      <Text style={styles.title} numberOfLines={1}>
        {title}
      </Text>
    </Pressable>
  );
}

const styles = StyleSheet.create({
  poster: { borderRadius: 9, backgroundColor: Theme.backgroundElevated },
  title: { marginTop: 6, fontSize: 11.5, fontWeight: "600", color: Theme.text },
});
