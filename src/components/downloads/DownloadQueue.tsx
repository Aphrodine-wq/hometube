import {
  ArrowCounterClockwiseIcon,
  CheckIcon,
  ClockIcon,
  SpinnerGapIcon,
  TrashIcon,
  WarningIcon,
  XIcon,
} from "@phosphor-icons/react";
import { useMemo, useState } from "react";
import type { DownloadJob } from "../../types";
import { SegmentTabContent, SegmentTabs } from "../ui/SegmentTabs";

const ACTIVE = new Set<DownloadJob["status"]>(["queued", "extracting", "downloading", "processing"]);
const RETRYABLE = new Set<DownloadJob["status"]>(["failed", "cancelled"]);

interface DownloadQueueProps {
  jobs: DownloadJob[];
  onCancel: (id: string) => void;
  onRetry: (id: string) => void;
  onClearFinished: () => void;
  onOpenLibrary: () => void;
}

function statusLabel(job: DownloadJob) {
  if (job.status === "queued") return "Waiting";
  if (job.status === "extracting") return "Reading link";
  if (job.status === "downloading") return "Downloading";
  if (job.status === "processing") return "Creating MP4";
  if (job.status === "complete") return "In your library";
  if (job.status === "cancelled") return "Cancelled";
  return "Needs attention";
}

function QueueList({
  jobs,
  onCancel,
  onRetry,
  onOpenLibrary,
}: Omit<DownloadQueueProps, "onClearFinished">) {
  if (!jobs.length) {
    return <div className="queue-empty"><ClockIcon /><strong>Nothing here yet</strong><span>Matching downloads will appear here.</span></div>;
  }
  return (
    <div className="download-jobs">
      {jobs.map((job) => (
        <article className={`download-job ${job.status}`} key={job.id}>
          <div className="job-icon" aria-hidden="true">
            {job.status === "complete" ? <CheckIcon /> : job.status === "failed" ? <WarningIcon /> : ACTIVE.has(job.status) ? <SpinnerGapIcon className={job.status === "queued" ? "" : "spin"} /> : <XIcon />}
          </div>
          <div className="job-main">
            <div className="job-title-row">
              <div>
                <strong>{job.currentTitle || (job.status === "queued" ? "Queued YouTube link" : "Reading YouTube link")}</strong>
                <span>{statusLabel(job)} · {job.quality === "best" ? "Best compatible" : job.quality}</span>
              </div>
              {job.itemCount > 1 ? <span className="playlist-count">{job.completedCount}/{job.itemCount} videos</span> : null}
            </div>
            {ACTIVE.has(job.status) ? (
              <div className="job-progress">
                <div
                  className={job.status === "processing" ? "indeterminate" : undefined}
                  role="progressbar"
                  aria-label={job.status === "processing" ? "Converting video for playback" : undefined}
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={job.status === "processing" ? undefined : Math.round(job.progress * 100)}
                >
                  <span style={{ width: job.status === "processing" ? "32%" : `${Math.round(job.progress * 100)}%` }} />
                </div>
                <small>{job.status === "processing"
                  ? "Converting for playback · keep HomeTube open"
                  : `${Math.round(job.progress * 100)}%${job.speed ? ` · ${job.speed}` : ""}${job.eta ? ` · ${job.eta} left` : ""}`}</small>
              </div>
            ) : null}
            {job.error ? <p className="job-error">{job.error}</p> : null}
          </div>
          <div className="job-actions">
            {ACTIVE.has(job.status) ? <button onClick={() => onCancel(job.id)}><XIcon /> Cancel</button> : null}
            {RETRYABLE.has(job.status) ? <button onClick={() => onRetry(job.id)}><ArrowCounterClockwiseIcon /> Retry</button> : null}
            {job.status === "complete" ? <button onClick={onOpenLibrary}><CheckIcon /> View library</button> : null}
          </div>
        </article>
      ))}
    </div>
  );
}

export function DownloadQueue(props: DownloadQueueProps) {
  const [tab, setTab] = useState("active");
  const active = useMemo(() => props.jobs.filter((job) => ACTIVE.has(job.status)), [props.jobs]);
  const history = useMemo(() => props.jobs.filter((job) => !ACTIVE.has(job.status)), [props.jobs]);

  return (
    <section className="queue-section" aria-labelledby="queue-title">
      <div className="queue-heading">
        <div><h2 id="queue-title">Download queue</h2><span className="queue-counts">{active.length} active · {history.length} in history</span></div>
        {history.length ? <button className="text-button" onClick={props.onClearFinished}><TrashIcon /> Clear history</button> : null}
      </div>
      <SegmentTabs
        value={tab}
        onValueChange={setTab}
        label="Download queue"
        tabs={[
          { value: "active", label: "Active", count: active.length },
          { value: "history", label: "History", count: history.length },
        ]}
      >
        <SegmentTabContent value="active"><QueueList {...props} jobs={active} /></SegmentTabContent>
        <SegmentTabContent value="history"><QueueList {...props} jobs={history} /></SegmentTabContent>
      </SegmentTabs>
    </section>
  );
}
