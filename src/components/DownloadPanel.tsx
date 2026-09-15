import { WarningIcon, XIcon } from "@phosphor-icons/react";
import { useEffect, useState, type FormEvent } from "react";
import { useDownloadPreview } from "../hooks/useDownloadPreview";
import { bridge } from "../lib/bridge";
import { loadDownloadQuality, saveDownloadQuality } from "../lib/uiPreferences";
import type { DownloadJob, DownloadQuality, YoutubeSearchItem } from "../types";
import { DownloadComposer } from "./downloads/DownloadComposer";
import { DownloadPreviewCard } from "./downloads/DownloadPreviewCard";
import { DownloadQueue } from "./downloads/DownloadQueue";
import { YoutubeSearch } from "./downloads/YoutubeSearch";

interface DownloadPanelProps {
  onOpenLibrary: () => void;
}

export default function DownloadPanel({ onOpenLibrary }: DownloadPanelProps) {
  const [url, setUrl] = useState("");
  const [quality, setQuality] = useState<DownloadQuality>(() => loadDownloadQuality());
  const [jobs, setJobs] = useState<DownloadJob[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const preview = useDownloadPreview(url);

  useEffect(() => {
    let disposed = false;
    let cleanup: () => void = () => undefined;
    void bridge.downloadQueue()
      .then((snapshot) => { if (!disposed) setJobs(snapshot.jobs); })
      .catch((loadError) => { if (!disposed) setError(String(loadError)); });
    void bridge.listen<DownloadJob>("download-progress", (next) => {
      if (disposed) return;
      setJobs((current) => current.some((job) => job.id === next.id)
        ? current.map((job) => job.id === next.id ? next : job)
        : [next, ...current]);
    }).then((unlisten) => {
      if (disposed) unlisten();
      else cleanup = unlisten;
    });
    return () => { disposed = true; cleanup(); };
  }, []);

  const changeQuality = (next: DownloadQuality) => {
    setQuality(next);
    saveDownloadQuality(next);
  };

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!preview.isValidUrl) {
      setError("Paste a complete youtube.com or youtu.be link.");
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      const job = await bridge.enqueueDownload({ url: url.trim(), quality });
      setJobs((current) => [job, ...current.filter((item) => item.id !== job.id)]);
      setUrl("");
    } catch (submitError) {
      setError(String(submitError));
    } finally {
      setSubmitting(false);
    }
  };

  const cancel = async (jobId: string) => {
    try { await bridge.cancelDownload(jobId); } catch (nextError) { setError(String(nextError)); }
  };

  const retry = async (jobId: string) => {
    try {
      const job = await bridge.retryDownload(jobId);
      setJobs((current) => current.map((item) => item.id === job.id ? job : item));
    } catch (nextError) {
      setError(String(nextError));
    }
  };

  const clearFinished = async () => {
    try {
      await bridge.clearFinishedDownloads();
      setJobs((current) => current.filter((job) => !["complete", "failed", "cancelled"].includes(job.status)));
    } catch (nextError) {
      setError(String(nextError));
    }
  };

  const downloadResult = async (item: YoutubeSearchItem) => {
    try {
      const job = await bridge.enqueueDownload({ url: item.url, quality });
      setJobs((current) => [job, ...current.filter((entry) => entry.id !== job.id)]);
    } catch (enqueueError) {
      setError(String(enqueueError));
    }
  };

  return (
    <section className="discover-page download-page">
      <div className="page-heading discover-heading">
        <span className="eyebrow">Add to HomeTube</span>
        <h1>Bring YouTube home</h1>
        <p>Preview a public video or playlist, choose a quality, and add it to your offline library.</p>
      </div>

      <div className="download-workbench">
        <DownloadComposer
          url={url}
          onUrlChange={setUrl}
          quality={quality}
          onQualityChange={changeQuality}
          onSubmit={submit}
          submitting={submitting}
        />
        <div className="download-note" id="download-help">Public videos only · Downloads run one at a time · Playlists keep their channel grouping</div>
        <DownloadPreviewCard
          state={preview.state}
          quality={quality}
          onRetry={preview.retry}
        />
      </div>

      {error ? (
        <div className="download-alert" role="alert">
          <WarningIcon weight="fill" />
          <span>{error}</span>
          <button onClick={() => setError(null)} aria-label="Dismiss error"><XIcon weight="bold" /></button>
        </div>
      ) : null}

      <YoutubeSearch
        queuedUrls={jobs.filter((job) => job.status !== "failed" && job.status !== "cancelled").map((job) => job.url)}
        onDownload={downloadResult}
      />

      <DownloadQueue
        jobs={jobs}
        onCancel={(id) => void cancel(id)}
        onRetry={(id) => void retry(id)}
        onClearFinished={() => void clearFinished()}
        onOpenLibrary={onOpenLibrary}
      />
    </section>
  );
}
