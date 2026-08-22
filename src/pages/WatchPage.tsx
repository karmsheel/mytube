import { convertFileSrc } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { api } from "../api";
import { VideoCard } from "../components/VideoCard";
import { resumePosition } from "../lib/format";
import { formatInvokeError } from "../lib/format";
import type { Playlist, VideoCard as Card, VideoDetail } from "../types";

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
  const [saveOpen, setSaveOpen] = useState(false);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [newPl, setNewPl] = useState("");
  const [saveMsg, setSaveMsg] = useState<string | null>(null);

  useEffect(() => {
    started.current = false;
    posRef.current = 0;
    setBanner(null);
    setSrc(null);
    setMeta(null);
    setMore([]);
    setOpen(false);
    setSaveOpen(false);
    setSaveMsg(null);
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

  const moreTitle = "More from this folder";

  async function openSave() {
    setSaveOpen(true);
    setSaveMsg(null);
    try {
      setPlaylists(await api.listPlaylists());
    } catch (e) {
      setSaveMsg(formatInvokeError(e));
    }
  }

  async function addTo(pl: Playlist) {
    if (!meta) return;
    try {
      const updated = await api.addToPlaylist(pl.id, meta.id);
      setPlaylists((cur) => cur.map((p) => (p.id === updated.id ? updated : p)));
      setSaveMsg(`Saved to ${updated.name}`);
    } catch (e) {
      setSaveMsg(formatInvokeError(e));
    }
  }

  async function createAndSave() {
    const name = newPl.trim();
    if (!name || !meta) return;
    try {
      const pl = await api.createPlaylist(name);
      setNewPl("");
      const updated = await api.addToPlaylist(pl.id, meta.id);
      setPlaylists((cur) => [updated, ...cur.filter((p) => p.id !== updated.id)]);
      setSaveMsg(`Saved to ${updated.name}`);
    } catch (e) {
      setSaveMsg(formatInvokeError(e));
    }
  }

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
              <p><Link to={`/folder/${meta.channelSlug}`}>{meta.channelName}</Link></p>
            )}
            {meta.uploadDate && <p className="badge">{meta.uploadDate}</p>}
            <p>
              <button className="primary" type="button" onClick={openSave}>Save to playlist</button>
            </p>
            {saveOpen && (
              <div className="save-panel">
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    createAndSave();
                  }}
                  style={{ display: "flex", gap: 8, marginBottom: 8 }}
                >
                  <input
                    className="search"
                    style={{ borderRadius: 8, flex: 1 }}
                    value={newPl}
                    onChange={(e) => setNewPl(e.target.value)}
                    placeholder="New playlist name"
                  />
                  <button className="primary" type="submit">Create</button>
                </form>
                {playlists.length === 0 && <p className="badge">No playlists yet.</p>}
                {playlists.map((pl) => (
                  <button key={pl.id} className="save-row" type="button" onClick={() => addTo(pl)}>
                    {pl.name} <span className="badge">{pl.videoCount}</span>
                  </button>
                ))}
                {saveMsg && <p className="badge">{saveMsg}</p>}
              </div>
            )}
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
