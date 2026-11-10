// main.tsx
import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import ManagerWindow from "./components/manager/ManagerWindow";
import ChooseWindow from "./components/choose/ChooseWindow";
import PreviewWindow from "./components/preview/PreviewWindow";

const label = getCurrentWindow().label;

const el = document.createElement("div");
document.body.appendChild(el);

const root = createRoot(el);
if (label === "manager") {
    root.render(<ManagerWindow />);
} else if (label === "choose") {
    root.render(<ChooseWindow />);
} else if (label === "preview") {
    root.render(<PreviewWindow />);
} else {
    root.render(<App />);
}
