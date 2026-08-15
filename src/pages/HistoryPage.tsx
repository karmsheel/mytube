import { useEffect, useState } from "react";
import { api } from "../api";
import { VideoGrid } from "../components/VideoGrid";
import type { VideoCard } from "../types";

export function HistoryPage() {
  const [items, setItems] = useState<VideoCard[]>([]);
  useEffect(() => { api.listHistory(0).then((p) => setItems(p.items)); }, []);
  return (
    <div>
      <h1>History</h1>
      {items.length === 0 ? <p>No watch history yet</p> : <VideoGrid videos={items} />}
    </div>
  );
}
