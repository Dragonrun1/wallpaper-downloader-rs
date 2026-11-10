import { useState } from "react";
import ProvidersTab from "./components/tabs/ProvidersTab";
import SettingsTab from "./components/tabs/SettingsTab";
import ChangerTab from "./components/tabs/ChangerTab";
import DownloadsTab from "./components/tabs/DownloadsTab";
import WallpapersTab from "./components/tabs/WallpapersTab";
import AboutTab from "./components/tabs/AboutTab";
import HelpTab from "./components/tabs/HelpTab";

const tabs = ["Providers", "Settings", "Changer", "Downloads", "Wallpapers", "About", "Help"] as const;

export default function App() {
    const [active, setActive] = useState(0);

    return (
        <div className="w-[694px] h-[445px] border rounded shadow flex flex-col">
            <div className="flex border-b">
                {tabs.map((t, i) => (
                    <button
                        key={t}
                        className={`px-3 py-1.5 text-sm ${active === i ? "border-b-2 border-blue-500" : ""}`}
                        onClick={() => setActive(i)}
                    >
                        {t}
                    </button>
                ))}
            </div>
            <div className="flex-1 overflow-auto p-3">
                {active === 0 && <ProvidersTab />}
                {active === 1 && <SettingsTab />}
                {active === 2 && <ChangerTab />}
                {active === 3 && <DownloadsTab />}
                {active === 4 && <WallpapersTab />}
                {active === 5 && <AboutTab />}
                {active === 6 && <HelpTab />}
            </div>
        </div>
    );
}
