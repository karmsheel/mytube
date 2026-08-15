import { convertFileSrc } from "@tauri-apps/api/core";
import { Link } from "react-router-dom";
import { formatDuration, letterTile } from "../lib/format";
import type { VideoCard as Card } from "../types";

export function VideoCard({ video }: { video: Card }) {
  const src = video.thumbnailPath ? convertFileSrc(video.thumbnailPath) : null;
  const dur = formatDuration(video.durationSec);
  return (
    <article>
      <Link to={`/watch/${video.id}`}>
        <div className="card-thumb">
          {src ? <img src={src} alt="" /> : letterTile(video.title)}
          {dur && <span className="duration">{dur}</span>}
        </div>
        <h3 className="card-title">{video.title}</h3>
      </Link>
      {video.channelSlug && video.channelName ? (
        <Link className="card-channel" to={`/channel/${video.channelSlug}`}>{video.channelName}</Link>
      ) : video.channelName ? (
        <div className="card-channel">{video.channelName}</div>
      ) : null}
    </article>
  );
}
