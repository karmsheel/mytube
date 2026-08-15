import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { api } from "../api";
import { EmptyLibrary } from "../components/EmptyLibrary";
import { VideoGrid } from "../components/VideoGrid";
import type { VideoCard } from "../types";

export function HomePage() {
  const [params] = useSearchParams();
  const q = params.get("q")?.trim() ?? "";
  const [items, setItems] = useState<VideoCard[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  const [ready, setReady] = useState(false);
  useEffect(() => {
    let cancel = false;
    setReady(false);
    (async () => {
      const res = q ? await api.search(q, 0) : await api.listHome(0);
      if (!cancel) {
        setItems(res.items);
        setTotal(res.total);
        setPage(0);
        setReady(true);
      }
    })();
    return () => { cancel = true; };
  }, [q]);
  if (!ready) return <p>Loading…</p>;
  if (!q && total === 0) return <EmptyLibrary />;
  return (
    <div>
      <h1>{q ? "Results" : "Home"}</h1>
      <VideoGrid videos={items} />
      {items.length < total && (
        <p><button onClick={async () => {
          const next = page + 1;
          const res = q ? await api.search(q, next) : await api.listHome(next);
          setPage(next);
          setItems((cur) => [...cur, ...res.items]);
        }}>Load more</button></p>
      )}
    </div>
  );
}
