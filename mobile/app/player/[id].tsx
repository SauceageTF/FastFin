import { useEffect, useState, useCallback } from "react";
import { View, Text, Pressable, ActivityIndicator, StyleSheet, StatusBar } from "react-native";
import { VideoView, useVideoPlayer } from "expo-video";
import { useEvent } from "expo";
import { Ionicons } from "@expo/vector-icons";
import * as ScreenOrientation from "expo-screen-orientation";
import { useLocalSearchParams, useRouter, Stack } from "expo-router";
import { useSession, type SessionInfo } from "../../lib/session";
import { Theme } from "../../lib/theme";
import { getItem, getPlaybackSource, type PlaybackSource, type TrackOption } from "../../lib/jellyfin";
import { episodeLabel } from "../../lib/types";

export default function PlayerScreen() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const { session } = useSession();
  const router = useRouter();

  const [title, setTitle] = useState("");
  const [subtitle, setSubtitle] = useState<string | null>(null);
  const [source, setSource] = useState<PlaybackSource | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [selectedAudio, setSelectedAudio] = useState<number | undefined>(undefined);
  const [selectedSubtitle, setSelectedSubtitle] = useState<number | undefined>(undefined);

  useEffect(() => {
    ScreenOrientation.lockAsync(ScreenOrientation.OrientationLock.LANDSCAPE);
    return () => {
      ScreenOrientation.lockAsync(ScreenOrientation.OrientationLock.PORTRAIT_UP);
    };
  }, []);

  const load = useCallback(
    async (audioIndex?: number, subtitleIndex?: number) => {
      if (!session) return;
      try {
        const item = await getItem(session, id);
        setTitle(item.SeriesName ?? item.Name);
        setSubtitle(item.SeriesName ? episodeLabel(item) : null);
        const startTicks = audioIndex === undefined && subtitleIndex === undefined ? item.UserData?.PlaybackPositionTicks ?? 0 : 0;

        const result = await getPlaybackSource(session, id, startTicks, audioIndex, subtitleIndex);
        if (!result) {
          setErrorMessage("The server didn't return a usable media source for this item.");
          return;
        }
        setSelectedAudio(result.selectedAudioIndex);
        setSelectedSubtitle(result.selectedSubtitleIndex);
        setSource(result);
      } catch (error) {
        setErrorMessage(`Couldn't start playback: ${error instanceof Error ? error.message : String(error)}`);
      }
    },
    [session, id]
  );

  useEffect(() => {
    load();
  }, [load]);

  if (!session) return null;

  return (
    <View style={styles.root}>
      <Stack.Screen options={{ headerShown: false }} />
      <StatusBar hidden />
      {source ? (
        <PlayerCore
          key={`${selectedAudio ?? "d"}-${selectedSubtitle ?? "d"}`}
          session={session}
          source={source}
          title={title}
          subtitle={subtitle}
          onSelectAudio={(index) => load(index, selectedSubtitle)}
          onSelectSubtitle={(index) => load(selectedAudio, index)}
          onClose={() => router.back()}
        />
      ) : errorMessage ? (
        <View style={styles.center}>
          <Text style={styles.errorText}>{errorMessage}</Text>
          <Text style={styles.diagnostic}>item {id.slice(0, 8)}</Text>
          <Pressable onPress={() => router.back()} style={{ marginTop: 16 }}>
            <Text style={{ color: "#ff7a3d", fontWeight: "700" }}>Close</Text>
          </Pressable>
        </View>
      ) : (
        <View style={styles.center}>
          <ActivityIndicator color="#fff" />
        </View>
      )}
    </View>
  );
}

function PlayerCore({
  session,
  source,
  title,
  subtitle,
  onSelectAudio,
  onSelectSubtitle,
  onClose,
}: {
  session: SessionInfo;
  source: PlaybackSource;
  title: string;
  subtitle: string | null;
  onSelectAudio: (index: number | undefined) => void;
  onSelectSubtitle: (index: number | undefined) => void;
  onClose: () => void;
}) {
  const player = useVideoPlayer(source.url, (p) => {
    p.play();
  });

  const { isPlaying } = useEvent(player, "playingChange", { isPlaying: player.playing });
  const { currentTime } = useEvent(player, "timeUpdate", { currentTime: player.currentTime, bufferedPosition: 0, currentLiveTimestamp: null, currentOffsetFromLive: null });
  const { error } = useEvent(player, "statusChange", { status: player.status, error: undefined as { message: string } | undefined, oldStatus: undefined });
  const [duration, setDuration] = useState(0);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [isPiPActive, setIsPiPActive] = useState(false);
  const audioTracks = source.audioTracks;
  const subtitleTracks = source.subtitleTracks;

  useEffect(() => {
    const sub = player.addListener("sourceLoad", (payload: any) => {
      if (payload.duration) setDuration(payload.duration);
    });
    return () => sub.remove();
  }, [player]);

  useEffect(() => {
    if (!isPiPActive) return;
    ScreenOrientation.unlockAsync();
    return () => {
      ScreenOrientation.lockAsync(ScreenOrientation.OrientationLock.LANDSCAPE);
    };
  }, [isPiPActive]);

  function toggleControls() {
    setControlsVisible((v) => !v);
  }

  function skip(seconds: number) {
    const target = player.currentTime + seconds;
    player.currentTime = Math.max(0, duration > 0 ? Math.min(duration, target) : target);
  }

  return (
    <View style={{ flex: 1, backgroundColor: "#000" }}>
      <VideoView
        player={player}
        style={StyleSheet.absoluteFill}
        contentFit="contain"
        nativeControls={false}
        allowsPictureInPicture
        onPictureInPictureStart={() => setIsPiPActive(true)}
        onPictureInPictureStop={() => setIsPiPActive(false)}
      />

      <Pressable style={StyleSheet.absoluteFill} onPress={toggleControls} />

      {controlsVisible ? (
        <View style={styles.hud} pointerEvents="box-none">
          <View style={styles.hudTop}>
            <Pressable onPress={onClose} hitSlop={12}>
              <Ionicons name="chevron-back" size={22} color="#fff" />
            </Pressable>
            <View style={{ flex: 1 }}>
              <Text style={styles.hudTitle} numberOfLines={1}>
                {title}
              </Text>
              {subtitle ? (
                <Text style={styles.hudSubtitle} numberOfLines={1}>
                  {subtitle}
                </Text>
              ) : null}
            </View>
            {audioTracks.length > 0 || subtitleTracks.length > 0 ? (
              <TrackPicker
                audioTracks={audioTracks}
                subtitleTracks={subtitleTracks}
                onSelectAudio={onSelectAudio}
                onSelectSubtitle={onSelectSubtitle}
              />
            ) : null}
          </View>

          <View style={styles.hudCenter}>
            <Pressable onPress={() => skip(-10)} hitSlop={12}>
              <Ionicons name="play-back" size={26} color="#fff" />
            </Pressable>
            <Pressable onPress={() => (isPlaying ? player.pause() : player.play())} style={styles.playButton}>
              <Ionicons name={isPlaying ? "pause" : "play"} size={26} color="#fff" />
            </Pressable>
            <Pressable onPress={() => skip(10)} hitSlop={12}>
              <Ionicons name="play-forward" size={26} color="#fff" />
            </Pressable>
          </View>

          <View style={styles.hudBottom}>
            <Text style={styles.time}>{formatSeconds(currentTime)}</Text>
            <View style={styles.track}>
              <View style={[styles.trackFill, { width: `${duration ? (currentTime / duration) * 100 : 0}%` }]} />
            </View>
            <Text style={styles.time}>-{formatSeconds(Math.max(0, duration - currentTime))}</Text>
          </View>
        </View>
      ) : null}

      {error ? (
        <View style={styles.errorOverlay}>
          <Text style={styles.errorText}>Playback couldn&rsquo;t load this video</Text>
          <Text style={styles.diagnostic}>{error.message}</Text>
          <Pressable onPress={onClose} style={{ marginTop: 16 }}>
            <Text style={{ color: "#ff7a3d", fontWeight: "700" }}>Close</Text>
          </Pressable>
        </View>
      ) : null}
    </View>
  );
}

function TrackPicker({
  audioTracks,
  subtitleTracks,
  onSelectAudio,
  onSelectSubtitle,
}: {
  audioTracks: TrackOption[];
  subtitleTracks: TrackOption[];
  onSelectAudio: (index: number | undefined) => void;
  onSelectSubtitle: (index: number | undefined) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <View>
      <Pressable onPress={() => setOpen((v) => !v)} hitSlop={12}>
        <Ionicons name="chatbox-ellipses-outline" size={20} color="#fff" />
      </Pressable>
      {open ? (
        <View style={styles.menu}>
          {audioTracks.length > 0 ? (
            <>
              <Text style={styles.menuHeader}>Audio</Text>
              {audioTracks.map((track) => (
                <Pressable key={track.index} style={styles.menuItem} onPress={() => { onSelectAudio(track.index); setOpen(false); }}>
                  <Text style={styles.menuItemText}>{track.title}</Text>
                </Pressable>
              ))}
            </>
          ) : null}
          {subtitleTracks.length > 0 ? (
            <>
              <Text style={styles.menuHeader}>Subtitles</Text>
              <Pressable style={styles.menuItem} onPress={() => { onSelectSubtitle(undefined); setOpen(false); }}>
                <Text style={styles.menuItemText}>Off</Text>
              </Pressable>
              {subtitleTracks.map((track) => (
                <Pressable key={track.index} style={styles.menuItem} onPress={() => { onSelectSubtitle(track.index); setOpen(false); }}>
                  <Text style={styles.menuItemText}>{track.title}</Text>
                </Pressable>
              ))}
            </>
          ) : null}
        </View>
      ) : null}
    </View>
  );
}

function formatSeconds(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const total = Math.floor(seconds);
  const minutes = Math.floor(total / 60);
  const secs = total % 60;
  return `${minutes}:${secs.toString().padStart(2, "0")}`;
}

const styles = StyleSheet.create({
  root: { flex: 1, backgroundColor: "#000" },
  center: { flex: 1, alignItems: "center", justifyContent: "center", padding: 32 },
  errorText: { color: "#fff", fontSize: 14, textAlign: "center" },
  diagnostic: { color: "rgba(255,255,255,0.6)", fontSize: 10, textAlign: "center", marginTop: 10, fontFamily: "monospace" },
  errorOverlay: { position: "absolute", left: 0, right: 0, top: 0, bottom: 0, alignItems: "center", justifyContent: "center", backgroundColor: "rgba(0,0,0,0.85)", padding: 32 },
  hud: { position: "absolute", left: 0, right: 0, top: 0, bottom: 0, justifyContent: "space-between", padding: 20 },
  hudTop: { flexDirection: "row", alignItems: "center", gap: 14 },
  hudTitle: { color: "#fff", fontSize: 14.5, fontWeight: "700" },
  hudSubtitle: { color: "rgba(255,255,255,0.75)", fontSize: 11, marginTop: 2 },
  hudCenter: { flexDirection: "row", alignItems: "center", justifyContent: "center", gap: 44 },
  playButton: { width: 60, height: 60, borderRadius: 30, backgroundColor: "rgba(255,255,255,0.15)", alignItems: "center", justifyContent: "center" },
  hudBottom: { flexDirection: "row", alignItems: "center", gap: 12 },
  time: { color: "rgba(255,255,255,0.9)", fontSize: 11.5, width: 40 },
  track: { flex: 1, height: 3, borderRadius: 2, backgroundColor: "rgba(255,255,255,0.28)" },
  trackFill: { height: "100%", borderRadius: 2, backgroundColor: "#2dd4c8" },
  menu: { position: "absolute", top: 28, right: 0, backgroundColor: "rgba(20,20,24,0.95)", borderRadius: 12, padding: 8, minWidth: 180 },
  menuHeader: { color: Theme.textDim, fontSize: 11, fontWeight: "700", marginTop: 6, marginBottom: 4, paddingHorizontal: 8 },
  menuItem: { paddingHorizontal: 8, paddingVertical: 8, borderRadius: 8 },
  menuItemText: { color: "#fff", fontSize: 13 },
});
