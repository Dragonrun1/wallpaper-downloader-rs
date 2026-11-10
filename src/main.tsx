import { createRoot } from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import ManagerWindow from "./components/manager/ManagerWindow";
import ChooseWindow from "./components/choose/ChooseWindow";
import PreviewWindow from "./components/preview/PreviewWindow";
import "./styles.css";

const label = getCurrentWindow().label;
const root = createRoot(document.getElementById("root")!);

switch (label) {
    case "manager":
        root.render(<ManagerWindow />);
        break;
    case "choose":
        root.render(<ChooseWindow />);
        break;
    case "preview":
        root.render(<PreviewWindow />);
        break;
    default:
        root.render(<App />);
}
