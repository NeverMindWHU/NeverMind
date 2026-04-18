import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import BubbleBall from "./BubbleBall";
import MainPanel from "./MainPanel";

function windowLabel(): string {
  if (!isTauri()) return "main";
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
}

export default function App() {
  const label = windowLabel();
  if (label === "bubble") {
    return <BubbleBall />;
  }
  return <MainPanel />;
}
