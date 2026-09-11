import { Stack, SplashScreen } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { GestureHandlerRootView } from "react-native-gesture-handler";
import { SessionProvider, useSession } from "../lib/session";
import { Theme } from "../lib/theme";

SplashScreen.preventAutoHideAsync();

function SplashScreenController() {
  const { isRestoring } = useSession();
  if (!isRestoring) SplashScreen.hide();
  return null;
}

function RootNavigator() {
  const { session } = useSession();

  return (
    <Stack screenOptions={{ headerShown: false, contentStyle: { backgroundColor: Theme.background } }}>
      <Stack.Protected guard={!!session}>
        <Stack.Screen name="(tabs)" />
        <Stack.Screen name="item/[id]" />
        <Stack.Screen name="library/[id]" />
        <Stack.Screen name="season/[id]" />
        <Stack.Screen name="player/[id]" options={{ presentation: "fullScreenModal", animation: "fade" }} />
      </Stack.Protected>

      <Stack.Protected guard={!session}>
        <Stack.Screen name="login" />
      </Stack.Protected>
    </Stack>
  );
}

export default function RootLayout() {
  return (
    <GestureHandlerRootView style={{ flex: 1, backgroundColor: Theme.background }}>
      <SessionProvider>
        <StatusBar style="light" />
        <SplashScreenController />
        <RootNavigator />
      </SessionProvider>
    </GestureHandlerRootView>
  );
}
