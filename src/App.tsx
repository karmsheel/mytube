import { useEffect } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { api } from "./api";
import { Shell } from "./components/Shell";
import { HomePage } from "./pages/HomePage";
import { HistoryPage } from "./pages/HistoryPage";
import { ChannelsPage } from "./pages/ChannelsPage";
import { ChannelPage } from "./pages/ChannelPage";
import { WatchPage } from "./pages/WatchPage";
import { LibraryPage } from "./pages/LibraryPage";

export default function App() {
  useEffect(() => { api.rescan().catch(() => {}); }, []);
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Shell />}>
          <Route path="/" element={<HomePage />} />
          <Route path="/history" element={<HistoryPage />} />
          <Route path="/channels" element={<ChannelsPage />} />
          <Route path="/channel/:slug" element={<ChannelPage />} />
          <Route path="/watch/:id" element={<WatchPage />} />
          <Route path="/library" element={<LibraryPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
