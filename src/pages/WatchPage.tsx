import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api } from "../api";
import { VideoCard } from "../components/VideoCard";
import { resumePosition } from "../lib/format";
import type { VideoCard as Card, VideoDetail } from "../types";

/** Tauri 2 serializes AppError::NotFound as `{ NotFound: path }`, not a string. */
function isNotFoundError(e: unknown): boolean {
  if (e != null && typeof e === "object" && "NotFound" in e) return true;
  if (typeof e === "string") {
    return e.toLowerCase().includes("not found") || e.includes("NotFound");
  }
  try {
    return JSON.stringify(e).includes("NotFound");
  } catch {
    return false;
  }
}

export function WatchPage() {
  const { id } = useParams();
  const vid = Number(id);
  const videoEl = useRef<HTMLVideoElement | null>(null);
  const started = useRef(false);
  const posRef = useRef(0);
  const [meta, setMeta] = useState<VideoDetail | null>(null);
  const [src, setSrc] = useState<string | null>(null);
  const [more, setMore] = useState<Card[]>([]);
  const [banner, setBanner] = useState<string | null>(null);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    started.current = false;
    posRef.current = 0;
    setBanner(null);
    setSrc(null);
    setMeta(null);
    setMore([]);
    setOpen(false);
    let cancelled = false;
    (async () => {
      try {
        const v = await api.getVideo(vid);
        if (cancelled) return;
        setMeta(v);
        const path = await api.videoUrl(vid);
        if (cancelled) return;
        setSrc(convertFileSrc(path));
        const extra = await api.listMore(vid);
        if (!cancelled) setMore(extra);
      } catch (e) {
        if (cancelled) return;
        setBanner(
          isNotFoundError(e)
            ? "File not found. Rescan from Library."
            : "Can't play this format",
        );
        try {
          const v = await api.getVideo(vid);
          if (!cancelled) setMeta(v);
        } catch {
          /* keep banner */
        }
        try {
          const extra = await api.listMore(vid);
          if (!cancelled) setMore(extra);
        } catch {
          /* ignore */
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [vid]);

  const playSrc = src && meta && meta.id === vid ? src : null;

  useEffect(() => {
    if (!playSrc) return;
    const timer = window.setInterval(() => {
      const el = videoEl.current;
      if (!el || el.paused || !started.current) return;
      posRef.current = el.currentTime;
      api.setProgress(vid, el.currentTime).catch(() => {});
    }, 5000);
    return () => {
      window.clearInterval(timer);
      if (started.current) {
        api.setProgress(vid, posRef.current).catch(() => {});
      }
    };
  }, [playSrc, vid]);

  function save(el: HTMLVideoElement) {
    if (!meta || !started.current) return;
    posRef.current = el.currentTime;
    api.setProgress(meta.id, el.currentTime).catch(() => {});
  }

  const moreTitle = meta?.channelSlug ? "More from this channel" : "More from this folder";

  return (
    <div className="watch">
      <div>
        {banner && <div className="error-banner">{banner}{meta?.path ? ` ${meta.path}` : ""}</div>}
        {playSrc && (
          <video
            ref={videoEl}
            className="player"
            src={playSrc}
            controls
            onPlay={() => {
              if (!started.current && meta) {
                started.current = true;
                api.startWatch(meta.id).catch(() => {});
              }
            }}
            onLoadedMetadata={(e) => {
              const el = e.currentTarget;
              const pos = resumePosition(meta?.progressSec ?? 0, meta?.durationSec ?? el.duration);
              el.currentTime = pos;
              posRef.current = pos;
            }}
            onTimeUpdate={(e) => { posRef.current = e.currentTarget.currentTime; }}
            onPause={(e) => save(e.currentTarget)}
            onError={() => setBanner((b) => b ?? "Can't play this format")}
          />
        )}
        {meta && (
          <>
            <h1>{meta.title}</h1>
            {meta.channelSlug && meta.channelName && (
              <p><Link to={`/channel/${meta.channelSlug}`}>{meta.channelName}</Link></p>
            )}
            {meta.uploadDate && <p className="badge">{meta.uploadDate}</p>}
            {meta.description && (
              <p className={open ? "desc open" : "desc"} onClick={() => setOpen((v) => !v)}>
                {meta.description}
              </p>
            )}
          </>
        )}
      </div>
      <aside>
        <h2>{moreTitle}</h2>
        <div className="more-list">
          {more.map((v) => <VideoCard key={v.id} video={v} />)}
        </div>
      </aside>
    </div>
  );
}
