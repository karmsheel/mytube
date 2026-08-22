import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { api } from "../api";
import { EmptyLibrary } from "../components/EmptyLibrary";
import { VideoGrid } from "../components/VideoGrid";
import { formatInvokeError } from "../lib/format";
import { useLibrary } from "../lib/library";
import type { VideoCard } from "../types";

export function HomePage() {
  const [params] = useSearchParams();
  const q = params.get("q")?.trim() ?? "";
  const { epoch } = useLibrary();
  const [items, setItems] = useState<VideoCard[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(0);
  const [ready, setReady] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    let cancel = false;
    setErr(null);
    (async () => {
      try {
        const res = q ? await api.search(q, 0) : await api.listHome(0);
        if (!cancel) {
          setItems(res.items);
          setTotal(res.total);
          setPage(0);
        }
      } catch (e) {
        if (!cancel) setErr(formatInvokeError(e));
      } finally {
        if (!cancel) setReady(true);
      }
    })();
    return () => { cancel = true; };
  }, [q, epoch]);
  if (!ready) return <p>Loading…</p>;
  if (err) return <div className="error-banner">{err}</div>;
  if (!q && total === 0) return <EmptyLibrary />;
  return (
    <div>
      <h1>{q ? "Results" : "Home"}</h1>
      <VideoGrid videos={items} />
      {items.length < total && (
        <p><button onClick={async () => {
          try {
            const next = page + 1;
            const res = q ? await api.search(q, next) : await api.listHome(next);
            setPage(next);
            setItems((cur) => [...cur, ...res.items]);
          } catch (e) {
            setErr(formatInvokeError(e));
          }
        }}>Load more</button></p>
      )}
    </div>
  );
}
