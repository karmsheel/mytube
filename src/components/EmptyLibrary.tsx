import { useState } from "react";
import { api } from "../api";
import { formatInvokeError } from "../lib/format";
import { useLibrary } from "../lib/library";

export function EmptyLibrary() {
  const { bump } = useLibrary();
  const [err, setErr] = useState<string | null>(null);
  async function add() {
    try {
      const path = await api.pickFolder();
      if (!path) return;
      await api.addSource(path);
      bump();
    } catch (e) {
      setErr(formatInvokeError(e));
    }
  }
  return (
    <div className="empty">
      {err && <div className="error-banner">{err}</div>}
      <p>Add a folder of videos</p>
      <button className="primary" onClick={add}>Add a folder of videos</button>
    </div>
  );
}
