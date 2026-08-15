import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { api } from "../api";
import { EmptyLibrary } from "../components/EmptyLibrary";

export function HomePage() {
  const [params] = useSearchParams();
  const q = params.get("q")?.trim() ?? "";
  const [empty, setEmpty] = useState(false);
  const [ready, setReady] = useState(false);
  useEffect(() => {
    let cancel = false;
    (async () => {
      const page = q ? await api.search(q, 0) : await api.listHome(0);
      if (!cancel) {
        setEmpty(!q && page.total === 0);
        setReady(true);
      }
    })();
    return () => { cancel = true; };
  }, [q]);
  if (!ready) return <p>Loading…</p>;
  if (empty) return <EmptyLibrary />;
  return <p>{q ? "Results" : "Home"}</p>;
}
