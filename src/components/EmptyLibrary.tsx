import { useNavigate } from "react-router-dom";
import { api } from "../api";

export function EmptyLibrary() {
  const navigate = useNavigate();
  async function add() {
    const path = await api.pickFolder();
    if (!path) return;
    await api.addSource(path);
    navigate("/");
  }
  return (
    <div className="empty">
      <p>Add a folder of videos</p>
      <button className="primary" onClick={add}>Add a folder of videos</button>
    </div>
  );
}
