import { invoke } from "@tauri-apps/api/core";
import type { Channel, Page, Playlist, ScanStats, Source, VideoCard, VideoDetail } from "./types";

export const api = {
  pickFolder: () => invoke<string | null>("pick_folder"),
  addSource: (path: string) =>
    invoke<{ source: Source; stats: ScanStats }>("add_source", { path }),
  removeSource: (id: number) => invoke<void>("remove_source", { id }),
  listSources: () => invoke<Source[]>("list_sources"),
  rescan: () => invoke<ScanStats>("rescan"),
  listHome: (page: number) => invoke<Page<VideoCard>>("list_home", { page }),
  search: (query: string, page: number) =>
    invoke<Page<VideoCard>>("search", { query, page }),
  listChannels: () => invoke<Channel[]>("list_channels"),
  getChannel: (slug: string, page: number) =>
    invoke<{ channel: Channel; videos: Page<VideoCard> }>("get_channel", {
      slug,
      page,
    }),
  getVideo: (id: number) => invoke<VideoDetail>("get_video", { id }),
  listMore: (videoId: number) => invoke<VideoCard[]>("list_more", { videoId }),
  videoUrl: (id: number) => invoke<string>("video_url", { id }),
  setProgress: (id: number, positionSec: number) =>
    invoke<void>("set_progress", { id, positionSec }),
  startWatch: (id: number) => invoke<void>("start_watch", { id }),
  listHistory: (page: number) => invoke<Page<VideoCard>>("list_history", { page }),
  listPlaylists: () => invoke<Playlist[]>("list_playlists"),
  createPlaylist: (name: string) => invoke<Playlist>("create_playlist", { name }),
  deletePlaylist: (id: number) => invoke<void>("delete_playlist", { id }),
  getPlaylist: (id: number, page: number) =>
    invoke<{ playlist: Playlist; videos: Page<VideoCard> }>("get_playlist", { id, page }),
  addToPlaylist: (playlistId: number, videoId: number) =>
    invoke<Playlist>("add_to_playlist", { playlistId, videoId }),
  removeFromPlaylist: (playlistId: number, videoId: number) =>
    invoke<Playlist>("remove_from_playlist", { playlistId, videoId }),
};
