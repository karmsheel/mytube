import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { api } from "../api";
import { VideoGrid } from "../components/VideoGrid";
import { useLibrary } from "../lib/library";
import type { Channel, VideoCard } from "../types";

export function ChannelPage() {
  const { slug } = useParams();
  const { epoch } = useLibrary();
  const [ch, setCh] = useState<Channel | null>(null);
  const [items, setItems] = useState<VideoCard[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    if (!slug) return;
    api.getChannel(slug, 0).then((r) => {
      setCh(r.channel);
      setItems(r.videos.items);
      setTotal(r.videos.total);
      setPage(0);
    }).catch(() => setErr("Folder not found"));
  }, [slug, epoch]);
  if (err) return <p>{err}</p>;
  if (!ch) return <p>Loading…</p>;
  return (
    <div>
      <h1>{ch.name}</h1>
      <VideoGrid videos={items} />
      {items.length < total && (
        <p><button onClick={async () => {
          if (!slug) return;
          const next = page + 1;
          const res = await api.getChannel(slug, next);
          setPage(next);
          setItems((cur) => [...cur, ...res.videos.items]);
        }}>Load more</button></p>
      )}
    </div>
  );
}
