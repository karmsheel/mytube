import { useEffect, useState } from "react";
import { api } from "../api";
import { VideoGrid } from "../components/VideoGrid";
import { useLibrary } from "../lib/library";
import type { VideoCard } from "../types";

export function HistoryPage() {
  const { epoch } = useLibrary();
  const [items, setItems] = useState<VideoCard[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  useEffect(() => {
    api.listHistory(0).then((p) => {
      setItems(p.items);
      setTotal(p.total);
      setPage(0);
    });
  }, [epoch]);
  return (
    <div>
      <h1>History</h1>
      {items.length === 0 ? <p>No watch history yet</p> : <VideoGrid videos={items} />}
      {items.length < total && (
        <p><button onClick={async () => {
          const next = page + 1;
          const res = await api.listHistory(next);
          setPage(next);
          setItems((cur) => [...cur, ...res.items]);
        }}>Load more</button></p>
      )}
    </div>
  );
}
