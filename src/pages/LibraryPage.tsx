import { useEffect, useState } from "react";
import { api } from "../api";
import type { Source } from "../types";

export function LibraryPage() {
  const [sources, setSources] = useState<Source[]>([]);
  const [err, setErr] = useState<string | null>(null);
  async function reload() {
    setSources(await api.listSources());
  }
  useEffect(() => { reload().catch((e) => setErr(String(e))); }, []);
  return (
    <div>
      <h1>Library</h1>
      {err && <div className="error-banner">{err}</div>}
      <p>
        <button className="primary" onClick={async () => {
          try {
            const path = await api.pickFolder();
            if (!path) return;
            await api.addSource(path);
            await reload();
          } catch (e) { setErr(String(e)); }
        }}>Add folder</button>{" "}
        <button onClick={async () => { await api.rescan(); await reload(); }}>Rescan</button>
      </p>
      {sources.map((s) => (
        <div className="source-row" key={s.id}>
          <div className="path">{s.path}</div>
          <span className="badge">{s.available ? "Available" : "Unavailable"}</span>
          <button onClick={async () => { await api.removeSource(s.id); await reload(); }}>Remove</button>
        </div>
      ))}
    </div>
  );
}
