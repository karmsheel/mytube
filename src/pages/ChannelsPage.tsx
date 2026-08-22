import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../api";
import { useLibrary } from "../lib/library";
import type { Channel } from "../types";

export function ChannelsPage() {
  const { epoch } = useLibrary();
  const [rows, setRows] = useState<Channel[]>([]);
  useEffect(() => { api.listChannels().then(setRows); }, [epoch]);
  return (
    <div>
      <h1>Folders</h1>
      {rows.length === 0 && <p className="badge">No folders yet. Add a library source with subfolders or sidecar channel names.</p>}
      {rows.map((c) => (
        <p key={c.id}><Link to={`/folder/${c.slug}`}>{c.name}</Link> <span className="badge">{c.videoCount}</span></p>
      ))}
    </div>
  );
}
