import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { api } from "../api";
import { VideoGrid } from "../components/VideoGrid";
import type { Channel, VideoCard } from "../types";

export function ChannelPage() {
  const { slug } = useParams();
  const [ch, setCh] = useState<Channel | null>(null);
  const [items, setItems] = useState<VideoCard[]>([]);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    if (!slug) return;
    api.getChannel(slug, 0).then((r) => {
      setCh(r.channel);
      setItems(r.videos.items);
    }).catch(() => setErr("Channel not found"));
  }, [slug]);
  if (err) return <p>{err}</p>;
  if (!ch) return <p>Loading…</p>;
  return (
    <div>
      <h1>{ch.name}</h1>
      <VideoGrid videos={items} />
    </div>
  );
}
