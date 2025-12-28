import { useState } from "react";
import Dashboard from "./components/Dashboard";
import Opportunities from "./components/Opportunities";
import Markets from "./components/Markets";
import Wallets from "./components/Wallets";
import Settings from "./components/Settings";
import Header from "./components/Header";
import { useCommonMarkets } from "./hooks/useTauri";

type Tab = "dashboard" | "opportunities" | "markets" | "wallets" | "settings";

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("dashboard");

  // Subscribe to common markets at app level to ensure we don't miss initial data
  useCommonMarkets();

  return (
    <div className="min-h-screen bg-dark-900 text-white">
      <Header activeTab={activeTab} onTabChange={setActiveTab} />

      <main className="p-4">
        {activeTab === "dashboard" && <Dashboard />}
        {activeTab === "opportunities" && <Opportunities />}
        {activeTab === "markets" && <Markets />}
        {activeTab === "wallets" && <Wallets />}
        {activeTab === "settings" && <Settings />}
      </main>
    </div>
  );
}

export default App;
