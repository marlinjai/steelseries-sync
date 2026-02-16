import { useState } from "react";
import Status from "./pages/Status";
import Settings from "./pages/Settings";
import "./App.css";

type Tab = "status" | "settings" | "backups";

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("status");

  return (
    <div className="app">
      <nav className="tab-bar">
        <button
          className={`tab ${activeTab === "status" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("status")}
        >
          Status
        </button>
        <button
          className={`tab ${activeTab === "settings" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("settings")}
        >
          Settings
        </button>
        <button
          className={`tab ${activeTab === "backups" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("backups")}
        >
          Backups
        </button>
      </nav>

      <main className="content">
        {activeTab === "status" && <Status />}
        {activeTab === "settings" && <Settings />}
        {activeTab === "backups" && <div className="page"><h2>Backups</h2><p>Coming soon...</p></div>}
      </main>
    </div>
  );
}

export default App;
