import { useState } from "react";
import Dashboard from "./components/Dashboard";
import Opportunities from "./components/Opportunities";
import Settings from "./components/Settings";
import Header from "./components/Header";

type Tab = "dashboard" | "opportunities" | "settings";

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("dashboard");

  return (
    <div className="min-h-screen bg-dark-900 text-white">
      <Header activeTab={activeTab} onTabChange={setActiveTab} />

      <main className="p-4">
        {activeTab === "dashboard" && <Dashboard />}
        {activeTab === "opportunities" && <Opportunities />}
        {activeTab === "settings" && <Settings />}
      </main>
    </div>
  );
}

export default App;
