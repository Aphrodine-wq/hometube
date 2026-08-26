import {
  ArrowClockwiseIcon as RotateCw,
  ArrowCounterClockwiseIcon as RotateCcw,
  ArrowsInSimpleIcon as Minimize2,
  ArrowsOutSimpleIcon as Maximize2,
  ArrowsClockwiseIcon as RefreshCw,
  PauseIcon as Pause,
  PictureInPictureIcon as PictureInPicture2,
  PlayIcon as Play,
  ScreencastIcon as Cast,
  SkipBackIcon as SkipBack,
  SkipForwardIcon as SkipForward,
  SpeakerHighIcon as Volume2,
  SpeakerSlashIcon as VolumeX,
  SpinnerGapIcon as LoaderCircle,
  WarningIcon as AlertTriangle,
  XIcon as X,
} from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { bridge } from "../lib/bridge";
import type { CastDeviceInfo, CastStatus, ConversionJob, MediaItem } from "../types";

interface PlayerOverlayProps {
  item: MediaItem;
  nextItem: MediaItem | null;
  previousItem: MediaItem | null;
  onNext: () => void;
  onPrevious: () => void;
  onClose: () => void;
  onConversionComplete: (mediaId: string, outputPath: string) => void;
  onError: (message: string) => void;
}

function formatTime(seconds: number) {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const total = Math.floor(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const remaining = total % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(remaining).padStart(2, "0")}`
    : `${minutes}:${String(remaining).padStart(2, "0")}`;
}

export function PlayerOverlay({ item, nextItem, previousItem, onNext, onPrevious, onClose, onConversionComplete, onError }: PlayerOverlayProps) {
  const shell = useRef<HTMLDivElement>(null);
  const video = useRef<HTMLVideoElement>(null);
  const lastSaved = useRef(0);
  const hideTimer = useRef<number | null>(null);
  const [playbackError, setPlaybackError] = useState(false);
  const [job, setJob] = useState<ConversionJob | null>(null);
  const [source, setSource] = useState<string | null>(null);
  const [playing, setPlaying] = useState(false);
  const [buffering, setBuffering] = useState(false);
  const [currentTime, setCurrentTime] = useState(item.progressSecs);
  const [duration, setDuration] = useState(item.durationSecs);
  const [volume, setVolume] = useState(1);
  const [muted, setMuted] = useState(false);
  const [rate, setRate] = useState(1);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [castOpen, setCastOpen] = useState(false);
  const [discovering, setDiscovering] = useState(false);
  const [devices, setDevices] = useState<CastDeviceInfo[]>([]);
  const [connectingId, setConnectingId] = useState<string | null>(null);
  const [castStatus, setCastStatus] = useState<CastStatus | null>(null);
  const [ended, setEnded] = useState(false);
  const [countdown, setCountdown] = useState<number | null>(null);
  const castFinishedFor = useRef<string | null>(null);
  const advancing = useRef(false);

  const casting = castStatus?.connected === true;
  const shownTime = casting ? castStatus.currentTime : currentTime;
  const shownDuration = (casting ? castStatus.duration : duration) || item.durationSecs;
  const shownPlaying = casting ? castStatus.state === "playing" || castStatus.state === "buffering" : playing;
  const shownVolume = casting ? castStatus.volume : volume;
  const shownMuted = casting ? castStatus.muted : muted;

  useEffect(() => {
    let disposed = false;
    setSource(null);
    setPlaybackError(false);
    setEnded(false);
    setCountdown(null);
    castFinishedFor.current = null;
    advancing.current = false;
    lastSaved.current = item.progressSecs;
    setCurrentTime(item.progressSecs);
    setDuration(item.durationSecs);
    void bridge.playbackUrl(item.id, item.conversionPath || item.path)
      .then((url) => {
        if (!disposed) setSource(url);
      })
      .catch((error) => {
        if (disposed) return;
        setPlaybackError(true);
        onError(`Could not open ${item.title}: ${String(error)}`);
      });
    return () => { disposed = true; };
  }, [item.id, item.conversionPath, item.path, item.progressSecs, item.durationSecs, item.title, onError]);

  useEffect(() => {
    let disposed = false;
    let cleanup: () => void = () => undefined;
    void bridge.listen<ConversionJob>("conversion-progress", (next) => {
      if (disposed || next.mediaId !== item.id) return;
      setJob(next);
      if (next.status === "complete" && next.outputPath) {
        setPlaybackError(false);
        onConversionComplete(item.id, next.outputPath);
      }
      if (next.status === "failed") onError(next.error || "Conversion failed");
    }).then((unlisten) => {
      if (disposed) unlisten();
      else cleanup = unlisten;
    });
    return () => { disposed = true; cleanup(); };
  }, [item.id, onConversionComplete, onError]);

  useEffect(() => {
    if (!casting) return;
    let disposed = false;
    let timer: number | null = null;
    const poll = () => {
      void bridge.castStatus()
        .then((status) => {
          if (disposed) return;
          setCastStatus(status);
          if (status.connected && Math.abs(status.currentTime - lastSaved.current) >= 5) {
            lastSaved.current = status.currentTime;
            void bridge.saveProgress(item.id, status.currentTime, status.duration || item.durationSecs);
          }
        })
        .catch((error) => {
          if (!disposed) {
            setCastStatus(null);
            onError(`Cast connection lost: ${String(error)}`);
          }
        })
        .finally(() => {
          if (!disposed) timer = window.setTimeout(poll, 1500);
        });
    };
    timer = window.setTimeout(poll, 1500);
    return () => {
      disposed = true;
      if (timer !== null) window.clearTimeout(timer);
    };
  }, [casting, item.durationSecs, item.id, onError]);

  useEffect(() => {
    if (castStatus?.idleReason !== "finished" || castStatus.mediaId !== item.id || castFinishedFor.current === item.id) return;
    castFinishedFor.current = item.id;
    setEnded(true);
    setCountdown(nextItem ? 10 : null);
    void bridge.saveProgress(item.id, castStatus.duration || item.durationSecs, castStatus.duration || item.durationSecs);
  }, [castStatus, item.durationSecs, item.id, nextItem]);

  useEffect(() => {
    if (hideTimer.current !== null) window.clearTimeout(hideTimer.current);
    if (shownPlaying && !castOpen) {
      hideTimer.current = window.setTimeout(() => setControlsVisible(false), 2600);
    } else {
      setControlsVisible(true);
    }
    return () => {
      if (hideTimer.current !== null) window.clearTimeout(hideTimer.current);
    };
  }, [shownPlaying, castOpen]);

  const persist = () => {
    const position = casting ? castStatus.currentTime : video.current?.currentTime;
    const total = casting ? castStatus.duration : video.current?.duration;
    if (position === undefined || total === undefined || !Number.isFinite(total)) return;
    void bridge.saveProgress(item.id, position, total);
  };

  const startConversion = async () => {
    try {
      setJob(await bridge.startConversion(item.id));
    } catch (error) {
      onError(String(error));
    }
  };

  const revealControls = () => {
    setControlsVisible(true);
    if (hideTimer.current !== null) window.clearTimeout(hideTimer.current);
    if (shownPlaying && !castOpen) {
      hideTimer.current = window.setTimeout(() => setControlsVisible(false), 2600);
    }
  };

  const togglePlayback = async () => {
    try {
      if (casting) {
        setCastStatus(await bridge.setCastPlaying(castStatus.state !== "playing"));
      } else if (video.current?.paused) {
        await video.current.play();
      } else {
        video.current?.pause();
      }
    } catch (error) {
      onError(String(error));
    }
  };

  const seekTo = async (next: number) => {
    const position = Math.max(0, Math.min(next, shownDuration || next));
    try {
      if (casting) setCastStatus(await bridge.seekCast(position));
      else if (video.current) {
        video.current.currentTime = position;
        setCurrentTime(position);
      }
    } catch (error) {
      onError(String(error));
    }
  };

  const changeVolume = async (next: number) => {
    const level = Math.max(0, Math.min(next, 1));
    try {
      if (casting) setCastStatus(await bridge.setCastVolume(level));
      else if (video.current) {
        video.current.volume = level;
        video.current.muted = false;
        setVolume(level);
        setMuted(false);
      }
    } catch (error) {
      onError(String(error));
    }
  };

  const toggleMute = async () => {
    try {
      if (casting) setCastStatus(await bridge.setCastMuted(!shownMuted));
      else if (video.current) {
        video.current.muted = !video.current.muted;
        setMuted(video.current.muted);
      }
    } catch (error) {
      onError(String(error));
    }
  };

  const toggleFullscreen = async () => {
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await shell.current?.requestFullscreen();
    } catch (error) {
      onError(`Fullscreen is unavailable: ${String(error)}`);
    }
  };

  const openPictureInPicture = async () => {
    try {
      if (!video.current || !("requestPictureInPicture" in video.current)) throw new Error("Picture-in-picture is unavailable");
      await video.current.requestPictureInPicture();
    } catch (error) {
      onError(String(error));
    }
  };

  const openCastPicker = async () => {
    setCastOpen(true);
    setDiscovering(true);
    try {
      setDevices(await bridge.discoverCastDevices());
    } catch (error) {
      onError(String(error));
      setDevices([]);
    } finally {
      setDiscovering(false);
    }
  };

  const connectCast = async (device: CastDeviceInfo) => {
    setConnectingId(device.id);
    try {
      video.current?.pause();
      setCastStatus(await bridge.startCast(device, item.id, currentTime));
      setCastOpen(false);
    } catch (error) {
      const message = String(error);
      if (message.includes("compatible H.264/AAC")) setPlaybackError(true);
      onError(message);
    } finally {
      setConnectingId(null);
    }
  };

  const stopCasting = async () => {
    const resumeAt = castStatus?.currentTime || 0;
    try {
      await bridge.stopCast();
      setCastStatus(null);
      if (video.current) {
        video.current.currentTime = resumeAt;
        setCurrentTime(resumeAt);
      }
    } catch (error) {
      onError(String(error));
    }
  };

  const advance = async () => {
    if (!nextItem || advancing.current) return;
    advancing.current = true;
    try {
      if (casting) setCastStatus(await bridge.loadNextCast(nextItem.id));
      onNext();
    } catch (error) {
      advancing.current = false;
      setCountdown(null);
      onError(`Could not play next: ${String(error)}`);
    }
  };

  const goPrevious = async () => {
    if (!previousItem || advancing.current) return;
    advancing.current = true;
    try {
      if (casting) setCastStatus(await bridge.loadNextCast(previousItem.id));
      onPrevious();
    } catch (error) {
      advancing.current = false;
      onError(`Could not play previous: ${String(error)}`);
    }
  };

  const replay = async () => {
    setEnded(false);
    setCountdown(null);
    await seekTo(0);
    await togglePlayback();
  };

  useEffect(() => {
    if (countdown === null) return;
    if (countdown <= 0) {
      void advance();
      return;
    }
    const timer = window.setTimeout(() => setCountdown((current) => current === null ? null : current - 1), 1000);
    return () => window.clearTimeout(timer);
  });

  const close = () => {
    persist();
    if (casting) void bridge.stopCast();
    onClose();
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) return;
      if (event.key === " " || event.key.toLowerCase() === "k") {
        event.preventDefault();
        void togglePlayback();
      } else if (event.key === "ArrowLeft" || event.key.toLowerCase() === "j") {
        event.preventDefault();
        void seekTo(shownTime - 10);
      } else if (event.key === "ArrowRight" || event.key.toLowerCase() === "l") {
        event.preventDefault();
        void seekTo(shownTime + 10);
      } else if (event.key.toLowerCase() === "m") {
        void toggleMute();
      } else if (event.key.toLowerCase() === "f") {
        void toggleFullscreen();
      } else if (event.key.toLowerCase() === "n" && nextItem) {
        void advance();
      } else if (event.key === "Escape") {
        event.preventDefault();
        close();
      }
      revealControls();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  useEffect(() => () => {
    if (hideTimer.current !== null) window.clearTimeout(hideTimer.current);
  }, []);

  return (
    <div
      ref={shell}
      className={`player-overlay ${controlsVisible || !shownPlaying ? "controls-visible" : "controls-hidden"}`}
      role="dialog"
      aria-modal="true"
      aria-label={`Playing ${item.title}`}
      onMouseMove={revealControls}
      onMouseLeave={() => { if (shownPlaying && !castOpen) setControlsVisible(false); }}
    >
      <div className="player-header">
        <button onClick={close} aria-label="Close player"><Minimize2 /></button>
        <div className="player-title"><span>{casting ? `Casting to ${castStatus.deviceName}` : "Now playing"}</span><strong>{item.title}</strong></div>
        <button onClick={close} aria-label="Close player"><X /></button>
      </div>

      {source ? <video
        key={source}
        ref={video}
        src={source}
        autoPlay={!casting}
        playsInline
        onClick={() => void togglePlayback()}
        onDoubleClick={() => void toggleFullscreen()}
        onLoadedMetadata={(event) => {
          setDuration(event.currentTarget.duration);
          event.currentTarget.volume = volume;
          event.currentTarget.playbackRate = rate;
          if (item.progressSecs > 0 && item.progressSecs < event.currentTarget.duration * 0.9) {
            event.currentTarget.currentTime = item.progressSecs;
          }
          if (casting) event.currentTarget.pause();
        }}
        onPlay={() => { setPlaying(true); setBuffering(false); revealControls(); }}
        onPause={() => { setPlaying(false); setControlsVisible(true); }}
        onWaiting={() => setBuffering(true)}
        onCanPlay={(event) => { setBuffering(false); if (casting) event.currentTarget.pause(); }}
        onTimeUpdate={(event) => {
          const position = event.currentTarget.currentTime;
          setCurrentTime(position);
          if (position - lastSaved.current >= 5) {
            lastSaved.current = position;
            persist();
          }
        }}
        onVolumeChange={(event) => {
          setVolume(event.currentTarget.volume);
          setMuted(event.currentTarget.muted);
        }}
        onEnded={() => {
          setPlaying(false);
          persist();
          setEnded(true);
          setCountdown(nextItem ? 10 : null);
        }}
        onError={() => setPlaybackError(true)}
      /> : <div className="player-loading"><LoaderCircle className="spin" /><span>Opening video</span></div>}

      {casting ? <div className="cast-screen"><Cast /><strong>Playing on {castStatus.deviceName}</strong><span>{castStatus.state}</span></div> : null}
      {(buffering || castStatus?.state === "buffering") && !playbackError ? <div className="player-buffering"><LoaderCircle className="spin" /></div> : null}
      {ended && !playbackError ? <div className="up-next">
        {nextItem ? <>
          <span className="eyebrow">Up next</span>
          <h2>{nextItem.title}</h2>
          <p>{nextItem.creator}{countdown !== null ? ` · Playing in ${countdown} seconds` : ""}</p>
          <div><button className="primary-button" onClick={() => void advance()}><Play /> Play now</button><button className="secondary-button" onClick={() => setCountdown(null)}>Cancel</button></div>
        </> : <>
          <span className="eyebrow">Queue complete</span>
          <h2>{item.title}</h2>
          <div><button className="primary-button" onClick={() => void replay()}><RotateCcw /> Replay</button><button className="secondary-button" onClick={close}>Close</button></div>
        </>}
      </div> : null}
      {!shownPlaying && !buffering && !playbackError && !castOpen ? <button className="player-center-play" onClick={() => void togglePlayback()} aria-label="Play"><Play weight="fill" /></button> : null}

      <div className="player-controls" role="group" aria-label="Playback controls">
        <input
          className="player-timeline"
          type="range"
          min={0}
          max={shownDuration || 0.1}
          step={0.1}
          value={Math.min(shownTime, shownDuration || shownTime)}
          onChange={(event) => void seekTo(Number(event.target.value))}
          aria-label="Seek"
          aria-valuetext={`${formatTime(shownTime)} of ${formatTime(shownDuration)}`}
        />
        <div className="player-control-row">
          <button onClick={() => void togglePlayback()} aria-label={shownPlaying ? "Pause" : "Play"}>{shownPlaying ? <Pause weight="fill" /> : <Play weight="fill" />}</button>
          <button onClick={() => void seekTo(shownTime - 10)} aria-label="Back 10 seconds"><RotateCcw /><small>10</small></button>
          <button onClick={() => void seekTo(shownTime + 10)} aria-label="Forward 10 seconds"><RotateCw /><small>10</small></button>
          <button onClick={() => void goPrevious()} disabled={!previousItem} aria-label="Previous video"><SkipBack /></button>
          <button onClick={() => void advance()} disabled={!nextItem} aria-label="Next video"><SkipForward /></button>
          <div className="player-volume">
            <button onClick={() => void toggleMute()} aria-label={shownMuted ? "Unmute" : "Mute"}>{shownMuted || shownVolume === 0 ? <VolumeX /> : <Volume2 />}</button>
            <input type="range" min={0} max={1} step={0.02} value={shownMuted ? 0 : shownVolume} onChange={(event) => void changeVolume(Number(event.target.value))} aria-label="Volume" aria-valuetext={`${Math.round((shownMuted ? 0 : shownVolume) * 100)} percent`} />
          </div>
          <span className="player-time">{formatTime(shownTime)} / {formatTime(shownDuration)}</span>
          <div className="player-spacer" />
          {!casting ? <select
            value={rate}
            onChange={(event) => {
              const next = Number(event.target.value);
              setRate(next);
              if (video.current) video.current.playbackRate = next;
            }}
            aria-label="Playback speed"
          >
            <option value={0.5}>0.5×</option>
            <option value={0.75}>0.75×</option>
            <option value={1}>1×</option>
            <option value={1.25}>1.25×</option>
            <option value={1.5}>1.5×</option>
            <option value={2}>2×</option>
          </select> : null}
          {!casting ? <button onClick={() => void openPictureInPicture()} aria-label="Picture in picture"><PictureInPicture2 /></button> : null}
          <button className={casting ? "active" : ""} onClick={() => casting ? void stopCasting() : void openCastPicker()} aria-label={casting ? "Stop casting" : "Cast to TV"}><Cast /></button>
          <button onClick={() => void toggleFullscreen()} aria-label="Fullscreen"><Maximize2 /></button>
        </div>
      </div>

      {castOpen ? <div className="cast-picker" role="dialog" aria-label="Cast to a device">
        <div className="cast-picker-header"><div><strong>Cast to TV</strong><span>Devices on this Wi-Fi network</span></div><button onClick={() => setCastOpen(false)} aria-label="Close device list"><X /></button></div>
        <div className="cast-device-list">
          {discovering ? <div className="cast-empty"><LoaderCircle className="spin" /><span>Looking for devices…</span></div> : devices.length ? devices.map((device) => (
            <button key={device.id} className={!device.available ? "unavailable" : ""} onClick={() => void connectCast(device)} disabled={connectingId !== null || !device.available}>
              {connectingId === device.id ? <LoaderCircle className="spin" /> : <Cast />}
              <span><strong>{device.name}</strong><small>{device.available ? device.model || "Google Cast device" : "Offline or asleep — turn it on, then scan again"}</small></span>
            </button>
          )) : <div className="cast-empty"><Cast /><span>No devices found</span><small>Make sure your TV and HomeTube are on the same Wi-Fi network.</small></div>}
        </div>
        {!discovering ? <button className="cast-refresh" onClick={() => void openCastPicker()}><RefreshCw /> Scan again</button> : null}
      </div> : null}

      {playbackError && !item.conversionPath ? (
        <div className="player-error">
          <AlertTriangle size={34} />
          <h2>This codec needs a compatible copy</h2>
          <p>Your original file stays untouched. HomeTube will create an H.264/AAC copy for local playback and casting.</p>
          <button className="primary-button" onClick={startConversion} disabled={job?.status === "queued" || job?.status === "running"}>
            {job?.status === "queued" || job?.status === "running" ? <LoaderCircle className="spin" /> : <RefreshCw />}
            {job?.status === "running" ? `Converting ${Math.round(job.progress * 100)}%` : "Create compatible copy"}
          </button>
        </div>
      ) : null}
    </div>
  );
}
