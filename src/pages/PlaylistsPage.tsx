import { FormEvent, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../api";
import { formatInvokeError } from "../lib/format";
import { useLibrary } from "../lib/library";
import type { Playlist } from "../types";

export function PlaylistsPage() {
  const { epoch } = useLibrary();
  const [rows, setRows] = useState<Playlist[]>([]);
  const [name, setName] = useState("");
  const [err, setErr] = useState<string | null>(null);

  async function reload() {
    setRows(await api.listPlaylists());
  }

  useEffect(() => {
    reload().catch((e) => setErr(formatInvokeError(e)));
  }, [epoch]);

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    const n = name.trim();
    if (!n) return;
    try {
      setErr(null);
      await api.createPlaylist(n);
      setName("");
      await reload();
    } catch (ex) {
      setErr(formatInvokeError(ex));
    }
  }

  return (
    <div>
      <h1>Playlists</h1>
      {err && <div className="error-banner">{err}</div>}
      <form onSubmit={onCreate} className="playlist-create">
        <input
          className="search"
          style={{ borderRadius: 8 }}
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Playlist name"
        />
        <button className="primary" type="submit">Create</button>
      </form>
      {rows.length === 0 && <p className="badge">No playlists yet. Create one above, then save videos from the watch page.</p>}
      {rows.map((p) => (
        <p key={p.id}>
          <Link to={`/playlist/${p.id}`}>{p.name}</Link>{" "}
          <span className="badge">{p.videoCount}</span>
        </p>
      ))}
    </div>
  );
}
