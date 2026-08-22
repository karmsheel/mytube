import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { api } from "../api";
import { VideoCard } from "../components/VideoCard";
import { formatInvokeError } from "../lib/format";
import { useLibrary } from "../lib/library";
import type { Playlist, VideoCard as Card } from "../types";

export function PlaylistPage() {
  const { id } = useParams();
  const pid = Number(id);
  const navigate = useNavigate();
  const { epoch } = useLibrary();
  const [pl, setPl] = useState<Playlist | null>(null);
  const [items, setItems] = useState<Card[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  const [err, setErr] = useState<string | null>(null);

  async function load(p = 0) {
    const r = await api.getPlaylist(pid, p);
    setPl(r.playlist);
    setItems(r.videos.items);
    setTotal(r.videos.total);
    setPage(p);
  }

  useEffect(() => {
    if (!Number.isFinite(pid)) {
      setErr("Playlist not found");
      return;
    }
    load(0).catch((e) => setErr(formatInvokeError(e)));
  }, [pid, epoch]);

  if (err) return <p>{err}</p>;
  if (!pl) return <p>Loading…</p>;

  return (
    <div>
      <h1>{pl.name}</h1>
      <p className="badge">{pl.videoCount} videos</p>
      <p>
        <button
          type="button"
          onClick={async () => {
            if (!confirm(`Delete playlist “${pl.name}”?`)) return;
            await api.deletePlaylist(pl.id);
            navigate("/playlists");
          }}
        >
          Delete playlist
        </button>
      </p>
      {items.length === 0 && <p className="badge">This playlist is empty. Open a video and use Save to playlist.</p>}
      <div className="grid">
        {items.map((v) => (
          <div key={v.id} className="playlist-item">
            <VideoCard video={v} />
            <button
              type="button"
              className="save-row"
              onClick={async () => {
                await api.removeFromPlaylist(pl.id, v.id);
                await load(0);
              }}
            >
              Remove
            </button>
          </div>
        ))}
      </div>
      {items.length < total && (
        <p>
          <button
            type="button"
            onClick={async () => {
              const next = page + 1;
              const r = await api.getPlaylist(pid, next);
              setPage(next);
              setItems((cur) => [...cur, ...r.videos.items]);
            }}
          >
            Load more
          </button>
        </p>
      )}
    </div>
  );
}
