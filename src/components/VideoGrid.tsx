import type { VideoCard as Card } from "../types";
import { VideoCard } from "./VideoCard";

export function VideoGrid({ videos }: { videos: Card[] }) {
  return (
    <div className="grid">
      {videos.map((v) => <VideoCard key={v.id} video={v} />)}
    </div>
  );
}
