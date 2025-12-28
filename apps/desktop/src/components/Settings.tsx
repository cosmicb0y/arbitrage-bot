import { useState, useEffect } from "react";
import { useConfig, useCredentials } from "../hooks/useTauri";
import type { Credentials } from "../types";

function Settings() {
  const { config, updateConfig } = useConfig();
  const { credentials, saveCredentials, loading: credentialsLoading } = useCredentials();
  const [localConfig, setLocalConfig] = useState(config);
  const [saved, setSaved] = useState(false);
  const [credentialsSaved, setCredentialsSaved] = useState(false);
  const [activeExchange, setActiveExchange] = useState<"binance" | "coinbase" | "upbit">("binance");
  const [editingCredentials, setEditingCredentials] = useState<Credentials>({
    binance: { api_key: "", secret_key: "" },
    coinbase: { api_key: "", secret_key: "" },
    upbit: { api_key: "", secret_key: "" },
  });

  // Update local state when config loads
  useEffect(() => {
    setLocalConfig(config);
  }, [config]);

  const handleSave = async () => {
    await updateConfig(localConfig);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const handleSaveCredentials = async () => {
    const success = await saveCredentials(editingCredentials);
    if (success) {
      setCredentialsSaved(true);
      setTimeout(() => setCredentialsSaved(false), 2000);
      // Clear editing form after save
      setEditingCredentials({
        binance: { api_key: "", secret_key: "" },
        coinbase: { api_key: "", secret_key: "" },
        upbit: { api_key: "", secret_key: "" },
      });
    }
  };

  const hasCredentialChanges = () => {
    const current = editingCredentials[activeExchange];
    return current.api_key.length > 0 || current.secret_key.length > 0;
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

      {/* API Credentials */}
      <div className="bg-dark-800 rounded-lg border border-dark-700 p-6 space-y-4">
        <h3 className="font-medium text-white">API Credentials</h3>
        <p className="text-sm text-gray-500">
          Enter your exchange API keys to enable balance queries and trading. Keys are stored in a local .env file.
        </p>

        {credentialsSaved && (
          <div className="bg-success-500/20 border border-success-500 rounded-lg p-3 text-sm text-success-500">
            Credentials saved successfully!
          </div>
        )}

        {/* Exchange Tabs */}
        <div className="flex space-x-2 border-b border-dark-700">
          {(["binance", "coinbase", "upbit"] as const).map((exchange) => (
            <button
              key={exchange}
              onClick={() => setActiveExchange(exchange)}
              className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
                activeExchange === exchange
                  ? "border-primary-500 text-primary-400"
                  : "border-transparent text-gray-400 hover:text-white"
              }`}
            >
              {exchange.charAt(0).toUpperCase() + exchange.slice(1)}
              {credentials[exchange]?.api_key && (
                <span className="ml-2 text-xs text-success-500">●</span>
              )}
            </button>
          ))}
        </div>

        {/* Credential Form */}
        {!credentialsLoading && (
          <div className="space-y-4 pt-2">
            {/* Current Status */}
            {credentials[activeExchange]?.api_key && (
              <div className="text-sm text-gray-400">
                Current: <span className="font-mono text-gray-300">{credentials[activeExchange].api_key}</span>
              </div>
            )}

            {/* API Key Input */}
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                API Key
              </label>
              <input
                type="text"
                value={editingCredentials[activeExchange].api_key}
                onChange={(e) =>
                  setEditingCredentials({
                    ...editingCredentials,
                    [activeExchange]: {
                      ...editingCredentials[activeExchange],
                      api_key: e.target.value,
                    },
                  })
                }
                placeholder="Enter new API key..."
                className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-primary-500 font-mono text-sm"
              />
            </div>

            {/* Secret Key Input */}
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Secret Key
              </label>
              <input
                type="password"
                value={editingCredentials[activeExchange].secret_key}
                onChange={(e) =>
                  setEditingCredentials({
                    ...editingCredentials,
                    [activeExchange]: {
                      ...editingCredentials[activeExchange],
                      secret_key: e.target.value,
                    },
                  })
                }
                placeholder="Enter new secret key..."
                className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-primary-500 font-mono text-sm"
              />
            </div>

            {/* Save Button */}
            <button
              onClick={handleSaveCredentials}
              disabled={!hasCredentialChanges()}
              className={`w-full font-medium py-2 px-4 rounded-lg transition-colors ${
                hasCredentialChanges()
                  ? "bg-primary-600 hover:bg-primary-500 text-white"
                  : "bg-dark-600 text-gray-500 cursor-not-allowed"
              }`}
            >
              Save {activeExchange.charAt(0).toUpperCase() + activeExchange.slice(1)} Credentials
            </button>
          </div>
        )}
      </div>

      {/* Info Cards */}
      <div className="grid grid-cols-2 gap-4">
        <div className="bg-dark-800 rounded-lg border border-dark-700 p-4">
          <h3 className="font-medium text-primary-500 mb-2">Supported Exchanges</h3>
          <ul className="text-sm text-gray-400 space-y-1">
            <li>• Binance</li>
            <li>• Coinbase</li>
            <li>• Upbit</li>
          </ul>
        </div>
        <div className="bg-dark-800 rounded-lg border border-dark-700 p-4">
          <h3 className="font-medium text-primary-500 mb-2">Security Note</h3>
          <p className="text-sm text-gray-400">
            API keys are stored locally in .env file. Never share your secret keys.
          </p>
        </div>
      </div>
    </div>
  );
}

export default Settings;
