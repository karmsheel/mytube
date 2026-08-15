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
      <h1>Channels</h1>
      {rows.map((c) => (
        <p key={c.id}><Link to={`/channel/${c.slug}`}>{c.name}</Link> <span className="badge">{c.videoCount}</span></p>
      ))}
    </div>
  );
}
