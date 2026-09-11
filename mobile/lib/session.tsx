import * as SecureStore from "expo-secure-store";
import Constants from "expo-constants";
import { createContext, useContext, useEffect, useState, useCallback, useMemo, type ReactNode } from "react";
import { Platform } from "react-native";

function randomId(): string {
  return Array.from({ length: 32 }, () => Math.floor(Math.random() * 16).toString(16)).join("");
}

export interface SessionInfo {
  serverUrl: string;
  accessToken: string;
  userId: string;
  deviceId: string;
}

interface SessionContextValue {
  session: SessionInfo | null;
  isRestoring: boolean;
  errorMessage: string | null;
  signIn: (serverUrl: string, username: string, password: string) => Promise<void>;
  signOut: () => Promise<void>;
}

const SessionContext = createContext<SessionContextValue | null>(null);

const KEYS = { serverUrl: "serverUrl", accessToken: "accessToken", userId: "userId", deviceId: "deviceId" };

async function getDeviceId(): Promise<string> {
  const existing = await SecureStore.getItemAsync(KEYS.deviceId);
  if (existing) return existing;
  const generated = randomId();
  await SecureStore.setItemAsync(KEYS.deviceId, generated);
  return generated;
}

function authHeader(deviceId: string, accessToken?: string): string {
  const fields: Record<string, string> = {
    DeviceId: deviceId,
    Device: Platform.OS === "ios" ? "iPhone" : "Android",
    Client: "FastFin",
    Version: Constants.expoConfig?.version ?? "1.0.0",
  };
  if (accessToken) fields.Token = accessToken;
  return `MediaBrowser ${Object.entries(fields).map(([k, v]) => `${k}=${v}`).join(", ")}`;
}

export function SessionProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<SessionInfo | null>(null);
  const [isRestoring, setIsRestoring] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const [serverUrl, accessToken, userId] = await Promise.all([
          SecureStore.getItemAsync(KEYS.serverUrl),
          SecureStore.getItemAsync(KEYS.accessToken),
          SecureStore.getItemAsync(KEYS.userId),
        ]);
        if (serverUrl && accessToken && userId) {
          const deviceId = await getDeviceId();
          setSession({ serverUrl, accessToken, userId, deviceId });
        }
      } finally {
        setIsRestoring(false);
      }
    })();
  }, []);

  const signIn = useCallback(async (rawServerUrl: string, username: string, password: string) => {
    setErrorMessage(null);
    let serverUrl = rawServerUrl.trim();
    if (!serverUrl) {
      setErrorMessage("Enter your server address.");
      return;
    }
    if (!serverUrl.includes("://")) serverUrl = `https://${serverUrl}`;
    serverUrl = serverUrl.replace(/\/+$/, "");

    try {
      const deviceId = await getDeviceId();
      const response = await fetch(`${serverUrl}/Users/AuthenticateByName`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: authHeader(deviceId),
        },
        body: JSON.stringify({ Username: username, Pw: password }),
      });
      if (!response.ok) {
        setErrorMessage(`Sign in failed (${response.status}). Check your server address and credentials.`);
        return;
      }
      const data = await response.json();
      const accessToken: string | undefined = data.AccessToken;
      const userId: string | undefined = data.User?.Id;
      if (!accessToken || !userId) {
        setErrorMessage("Sign in didn't return a session.");
        return;
      }
      await Promise.all([
        SecureStore.setItemAsync(KEYS.serverUrl, serverUrl),
        SecureStore.setItemAsync(KEYS.accessToken, accessToken),
        SecureStore.setItemAsync(KEYS.userId, userId),
      ]);
      setSession({ serverUrl, accessToken, userId, deviceId });
    } catch (error) {
      setErrorMessage(`Couldn't reach that server: ${error instanceof Error ? error.message : String(error)}`);
    }
  }, []);

  const signOut = useCallback(async () => {
    await Promise.all([
      SecureStore.deleteItemAsync(KEYS.serverUrl),
      SecureStore.deleteItemAsync(KEYS.accessToken),
      SecureStore.deleteItemAsync(KEYS.userId),
    ]);
    setSession(null);
  }, []);

  const value = useMemo(
    () => ({ session, isRestoring, errorMessage, signIn, signOut }),
    [session, isRestoring, errorMessage, signIn, signOut]
  );

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
}

export function useSession(): SessionContextValue {
  const ctx = useContext(SessionContext);
  if (!ctx) throw new Error("useSession must be used within a SessionProvider");
  return ctx;
}

export function authHeaderFor(session: SessionInfo): string {
  return authHeader(session.deviceId, session.accessToken);
}
