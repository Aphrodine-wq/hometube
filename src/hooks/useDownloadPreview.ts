import { useEffect, useRef, useState } from "react";
import { bridge } from "../lib/bridge";
import type { DownloadPreview } from "../types";

export type DownloadPreviewState =
  | { status: "idle"; preview: null; error: null }
  | { status: "loading"; preview: DownloadPreview | null; error: null }
  | { status: "ready"; preview: DownloadPreview; error: null }
  | { status: "error"; preview: null; error: string };

function looksLikeYoutubeUrl(value: string) {
  try {
    const url = new URL(value.trim());
    const host = url.hostname.toLowerCase();
    return ["http:", "https:"].includes(url.protocol)
      && (host === "youtu.be" || host === "youtube.com" || host.endsWith(".youtube.com"));
  } catch {
    return false;
  }
}

export function useDownloadPreview(url: string) {
  const requestToken = useRef(0);
  const [retryKey, setRetryKey] = useState(0);
  const [state, setState] = useState<DownloadPreviewState>({
    status: "idle",
    preview: null,
    error: null,
  });

  useEffect(() => {
    const value = url.trim();
    const token = ++requestToken.current;
    if (!value || !looksLikeYoutubeUrl(value)) {
      setState({ status: "idle", preview: null, error: null });
      return;
    }

    const timer = window.setTimeout(() => {
      setState((current) => ({
        status: "loading",
        preview: current.status === "ready" ? current.preview : null,
        error: null,
      }));
      void bridge.previewDownload(value)
        .then((preview) => {
          if (requestToken.current === token) {
            setState({ status: "ready", preview, error: null });
          }
        })
        .catch((error) => {
          if (requestToken.current === token) {
            setState({ status: "error", preview: null, error: String(error) });
          }
        });
    }, 500);

    return () => window.clearTimeout(timer);
  }, [url, retryKey]);

  return {
    state,
    ...state,
    retry: () => setRetryKey((value) => value + 1),
    isValidUrl: looksLikeYoutubeUrl(url),
  };
}
