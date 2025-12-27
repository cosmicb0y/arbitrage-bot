import { useState } from "react";
import { useConfig } from "../hooks/useTauri";

function Settings() {
  const { config, updateConfig } = useConfig();
  const [localConfig, setLocalConfig] = useState(config);
  const [saved, setSaved] = useState(false);

  // Update local state when config loads
  useState(() => {
    setLocalConfig(config);
  });

  const handleSave = async () => {
    await updateConfig(localConfig);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div className="max-w-2xl space-y-6">
      <h2 className="text-lg font-semibold">Settings</h2>

      {saved && (
        <div className="bg-success-500/20 border border-success-500 rounded-lg p-3 text-sm text-success-500">
          Settings saved successfully!
        </div>
      )}

      <div className="bg-dark-800 rounded-lg border border-dark-700 p-6 space-y-6">
        {/* Execution Mode */}
        <div>
          <label className="block text-sm font-medium text-gray-300 mb-2">
            Execution Mode
          </label>
          <select
            value={localConfig.mode}
            onChange={(e) =>
              setLocalConfig({ ...localConfig, mode: e.target.value })
            }
            className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-primary-500"
          >
            <option value="alert">Alert Only</option>
            <option value="manual">Manual Approval</option>
            <option value="auto">Auto Execute</option>
          </select>
          <p className="mt-1 text-sm text-gray-500">
            {localConfig.mode === "alert" &&
              "Only send alerts when opportunities are found."}
            {localConfig.mode === "manual" &&
              "Show approval dialog before executing trades."}
            {localConfig.mode === "auto" &&
              "Automatically execute profitable trades."}
          </p>
        </div>

        {/* Minimum Premium */}
        <div>
          <label className="block text-sm font-medium text-gray-300 mb-2">
            Minimum Premium (bps)
          </label>
          <input
            type="number"
            value={localConfig.min_premium_bps}
            onChange={(e) =>
              setLocalConfig({
                ...localConfig,
                min_premium_bps: parseInt(e.target.value) || 0,
              })
            }
            className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-primary-500"
          />
          <p className="mt-1 text-sm text-gray-500">
            Minimum price difference required to trigger an opportunity. 30 bps =
            0.3%
          </p>
        </div>

        {/* Max Slippage */}
        <div>
          <label className="block text-sm font-medium text-gray-300 mb-2">
            Maximum Slippage (bps)
          </label>
          <input
            type="number"
            value={localConfig.max_slippage_bps}
            onChange={(e) =>
              setLocalConfig({
                ...localConfig,
                max_slippage_bps: parseInt(e.target.value) || 0,
              })
            }
            className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-primary-500"
          />
          <p className="mt-1 text-sm text-gray-500">
            Maximum acceptable slippage when executing trades. 50 bps = 0.5%
          </p>
        </div>

        {/* Dry Run */}
        <div className="flex items-center justify-between">
          <div>
            <label className="block text-sm font-medium text-gray-300">
              Dry Run Mode
            </label>
            <p className="text-sm text-gray-500">
              Simulate trades without actually executing them
            </p>
          </div>
          <button
            onClick={() =>
              setLocalConfig({ ...localConfig, dry_run: !localConfig.dry_run })
            }
            className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
              localConfig.dry_run ? "bg-primary-600" : "bg-dark-600"
            }`}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                localConfig.dry_run ? "translate-x-6" : "translate-x-1"
              }`}
            />
          </button>
        </div>

        {/* Save Button */}
        <div className="pt-4 border-t border-dark-700">
          <button
            onClick={handleSave}
            className="w-full bg-primary-600 hover:bg-primary-500 text-white font-medium py-2 px-4 rounded-lg transition-colors"
          >
            Save Settings
          </button>
        </div>
      </div>

      {/* Info Cards */}
      <div className="grid grid-cols-2 gap-4">
        <div className="bg-dark-800 rounded-lg border border-dark-700 p-4">
          <h3 className="font-medium text-primary-500 mb-2">Supported Exchanges</h3>
          <ul className="text-sm text-gray-400 space-y-1">
            <li>• Binance</li>
            <li>• Coinbase</li>
            <li>• Kraken</li>
            <li>• OKX</li>
          </ul>
        </div>
        <div className="bg-dark-800 rounded-lg border border-dark-700 p-4">
          <h3 className="font-medium text-primary-500 mb-2">Trading Pairs</h3>
          <ul className="text-sm text-gray-400 space-y-1">
            <li>• BTC/USDT</li>
            <li>• ETH/USDT</li>
            <li>• SOL/USDT</li>
          </ul>
        </div>
      </div>
    </div>
  );
}

export default Settings;
