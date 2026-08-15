import { NavLink, Outlet, useNavigate, useSearchParams } from "react-router-dom";
import { useState } from "react";

export function Shell() {
  const [params] = useSearchParams();
  const [q, setQ] = useState(params.get("q") ?? "");
  const navigate = useNavigate();
  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand"><span className="brand-mark" /> Mytube</div>
        <nav className="nav">
          <NavLink to="/" end>Home</NavLink>
          <NavLink to="/history">History</NavLink>
          <NavLink to="/channels">Channels</NavLink>
        </nav>
        <nav className="nav-bottom">
          <NavLink to="/library">Library</NavLink>
        </nav>
      </aside>
      <div className="main">
        <header className="topbar">
          <form
            onSubmit={(e) => {
              e.preventDefault();
              navigate(q.trim() ? `/?q=${encodeURIComponent(q.trim())}` : "/");
            }}
            style={{ display: "flex", width: "min(640px, 100%)" }}
          >
            <input className="search" value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search" />
            <button className="primary" style={{ borderRadius: "0 40px 40px 0" }} type="submit">
              Search
            </button>
          </form>
        </header>
        <div className="pane">
          <Outlet />
        </div>
      </div>
    </div>
  );
}
