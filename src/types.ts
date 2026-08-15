export type Source = {
  id: number;
  path: string;
  addedAt: string;
  lastScannedAt: string | null;
  available: boolean;
};

export type Channel = {
  id: number;
  slug: string;
  name: string;
  videoCount: number;
};

export type VideoCard = {
  id: number;
  title: string;
  channelName: string | null;
  channelSlug: string | null;
  durationSec: number | null;
  thumbnailPath: string | null;
  uploadDate: string | null;
};

export type VideoDetail = VideoCard & {
  channelId: number | null;
  sourceId: number;
  path: string;
  parentDir: string;
  description: string | null;
  progressSec: number | null;
};

export type Page<T> = {
  items: T[];
  page: number;
  pageSize: number;
  total: number;
};

export type ScanStats = {
  imported: number;
  updated: number;
  removed: number;
  skippedDirs: number;
};
