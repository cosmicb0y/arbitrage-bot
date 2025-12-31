import { useState, useEffect } from "react";
import { useConfig, useCredentials, useSymbolMappings, useCommonMarkets } from "../hooks/useTauri";
import type { Credentials, SymbolMapping } from "../types";

function Settings() {
  const { config, updateConfig } = useConfig();
  const { credentials, saveCredentials, loading: credentialsLoading } = useCredentials();
  const { mappings, upsertMapping, removeMapping, loading: mappingsLoading } = useSymbolMappings();
  const commonMarkets = useCommonMarkets();
  const [localConfig, setLocalConfig] = useState(config);
  const [saved, setSaved] = useState(false);
  const [credentialsSaved, setCredentialsSaved] = useState(false);
  const [mappingSaved, setMappingSaved] = useState(false);
  const [activeExchange, setActiveExchange] = useState<"binance" | "coinbase" | "upbit" | "bithumb" | "bybit">("binance");
  const [editingCredentials, setEditingCredentials] = useState<Credentials>({
    binance: { api_key: "", secret_key: "" },
    coinbase: { api_key_id: "", secret_key: "" },
    upbit: { api_key: "", secret_key: "" },
    bithumb: { api_key: "", secret_key: "" },
    bybit: { api_key: "", secret_key: "" },
  });
  // Symbol mapping state
  const [newMapping, setNewMapping] = useState<SymbolMapping>({
    exchange: "",
    symbol: "",
    canonical_name: "",
    exclude: false,
    notes: "",
  });
  const [showAddMapping, setShowAddMapping] = useState(false);

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
        coinbase: { api_key_id: "", secret_key: "" },
        upbit: { api_key: "", secret_key: "" },
        bithumb: { api_key: "", secret_key: "" },
        bybit: { api_key: "", secret_key: "" },
      });
    }
  };

  const hasCredentialChanges = () => {
    if (activeExchange === "coinbase") {
      const current = editingCredentials.coinbase;
      return current.api_key_id.length > 0 || current.secret_key.length > 0;
    }
    const current = editingCredentials[activeExchange];
    return current.api_key.length > 0 || current.secret_key.length > 0;
  };

  const hasConfiguredCredentials = (exchange: typeof activeExchange) => {
    if (exchange === "coinbase") {
      return credentials.coinbase?.api_key_id;
    }
    return credentials[exchange]?.api_key;
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
          {(["binance", "coinbase", "upbit", "bithumb", "bybit"] as const).map((exchange) => (
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
              {hasConfiguredCredentials(exchange) && (
                <span className="ml-2 text-xs text-success-500">●</span>
              )}
            </button>
          ))}
        </div>

        {/* Credential Form */}
        {!credentialsLoading && (
          <div className="space-y-4 pt-2">
            {/* Coinbase-specific form */}
            {activeExchange === "coinbase" ? (
              <>
                {/* Current Status */}
                {credentials.coinbase?.api_key_id && (
                  <div className="text-sm text-gray-400">
                    Current: <span className="font-mono text-gray-300">{credentials.coinbase.api_key_id}</span>
                  </div>
                )}

                {/* API Key ID Input */}
                <div>
                  <label className="block text-sm font-medium text-gray-300 mb-2">
                    API Key ID
                  </label>
                  <input
                    type="text"
                    value={editingCredentials.coinbase.api_key_id}
                    onChange={(e) =>
                      setEditingCredentials({
                        ...editingCredentials,
                        coinbase: {
                          ...editingCredentials.coinbase,
                          api_key_id: e.target.value,
                        },
                      })
                    }
                    placeholder="organizations/{org_id}/apiKeys/{key_id}"
                    className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-primary-500 font-mono text-sm"
                  />
                  <p className="mt-1 text-xs text-gray-500">
                    Full key ID from CDP Portal (format: organizations/.../apiKeys/...)
                  </p>
                </div>

                {/* Secret Key Input (Textarea for multiline PEM) */}
                <div>
                  <label className="block text-sm font-medium text-gray-300 mb-2">
                    Secret Key (PEM)
                  </label>
                  <textarea
                    value={editingCredentials.coinbase.secret_key}
                    onChange={(e) =>
                      setEditingCredentials({
                        ...editingCredentials,
                        coinbase: {
                          ...editingCredentials.coinbase,
                          secret_key: e.target.value,
                        },
                      })
                    }
                    placeholder={"-----BEGIN EC PRIVATE KEY-----\nMHQC...\n-----END EC PRIVATE KEY-----"}
                    rows={5}
                    className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-primary-500 font-mono text-sm resize-none"
                  />
                  <p className="mt-1 text-xs text-gray-500">
                    Full PEM format with BEGIN/END headers. Must use ECDSA (ES256) key type.
                  </p>
                </div>
              </>
            ) : (
              <>
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
              </>
            )}

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

      {/* Symbol Mappings */}
      <div className="bg-dark-800 rounded-lg border border-dark-700 p-6 space-y-4">
        <div className="flex justify-between items-center">
          <div>
            <h3 className="font-medium text-white">Symbol Mappings</h3>
            <p className="text-sm text-gray-500">
              Handle cases where the same symbol represents different coins across exchanges.
            </p>
          </div>
          <button
            onClick={() => setShowAddMapping(!showAddMapping)}
            className="px-3 py-1.5 text-sm bg-primary-600 hover:bg-primary-500 text-white rounded-lg transition-colors"
          >
            {showAddMapping ? "Cancel" : "+ Add Mapping"}
          </button>
        </div>

        {mappingSaved && (
          <div className="bg-success-500/20 border border-success-500 rounded-lg p-3 text-sm text-success-500">
            Symbol mapping saved successfully!
          </div>
        )}

        {/* Add New Mapping Form */}
        {showAddMapping && (
          <div className="border border-dark-600 rounded-lg p-4 space-y-4 bg-dark-750">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Exchange
                </label>
                <select
                  value={newMapping.exchange}
                  onChange={(e) => setNewMapping({ ...newMapping, exchange: e.target.value })}
                  className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-primary-500"
                >
                  <option value="">Select exchange...</option>
                  {commonMarkets?.exchanges.map((ex) => (
                    <option key={ex} value={ex}>{ex}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Symbol (on exchange)
                </label>
                <input
                  type="text"
                  value={newMapping.symbol}
                  onChange={(e) => setNewMapping({ ...newMapping, symbol: e.target.value.toUpperCase() })}
                  placeholder="e.g., GTC"
                  className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-primary-500"
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Canonical Name
                </label>
                <input
                  type="text"
                  value={newMapping.canonical_name}
                  onChange={(e) => setNewMapping({ ...newMapping, canonical_name: e.target.value })}
                  placeholder="e.g., Gitcoin or GTC_BINANCE"
                  className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-primary-500"
                />
                <p className="mt-1 text-xs text-gray-500">
                  Unique name to distinguish this coin from others with the same symbol
                </p>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Notes (optional)
                </label>
                <input
                  type="text"
                  value={newMapping.notes || ""}
                  onChange={(e) => setNewMapping({ ...newMapping, notes: e.target.value })}
                  placeholder="e.g., Gitcoin on Binance"
                  className="w-full bg-dark-700 border border-dark-600 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:border-primary-500"
                />
              </div>
            </div>
            <div className="flex items-center justify-between">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={newMapping.exclude}
                  onChange={(e) => setNewMapping({ ...newMapping, exclude: e.target.checked })}
                  className="w-4 h-4 rounded border-dark-500 bg-dark-600 text-primary-500 focus:ring-primary-500"
                />
                <span className="text-sm text-gray-300">
                  Exclude from arbitrage (different coin, should not be matched)
                </span>
              </label>
              <button
                onClick={async () => {
                  if (newMapping.exchange && newMapping.symbol && newMapping.canonical_name) {
                    const success = await upsertMapping(newMapping);
                    if (success) {
                      setMappingSaved(true);
                      setTimeout(() => setMappingSaved(false), 2000);
                      setNewMapping({
                        exchange: "",
                        symbol: "",
                        canonical_name: "",
                        exclude: false,
                        notes: "",
                      });
                      setShowAddMapping(false);
                    }
                  }
                }}
                disabled={!newMapping.exchange || !newMapping.symbol || !newMapping.canonical_name}
                className={`px-4 py-2 text-sm rounded-lg transition-colors ${
                  newMapping.exchange && newMapping.symbol && newMapping.canonical_name
                    ? "bg-primary-600 hover:bg-primary-500 text-white"
                    : "bg-dark-600 text-gray-500 cursor-not-allowed"
                }`}
              >
                Save Mapping
              </button>
            </div>
          </div>
        )}

        {/* Existing Mappings List */}
        {!mappingsLoading && mappings.mappings.length > 0 && (
          <div className="space-y-2">
            <div className="text-sm text-gray-400 mb-2">
              {mappings.mappings.length} mapping{mappings.mappings.length !== 1 ? "s" : ""} configured
            </div>
            <div className="divide-y divide-dark-700">
              {mappings.mappings.map((mapping, idx) => (
                <div key={`${mapping.exchange}-${mapping.symbol}-${idx}`} className="flex items-center justify-between py-3">
                  <div className="flex items-center gap-4">
                    <span className="px-2 py-1 text-xs rounded bg-dark-600 text-gray-300">
                      {mapping.exchange}
                    </span>
                    <span className="font-mono text-primary-400">{mapping.symbol}</span>
                    <span className="text-gray-500">→</span>
                    <span className="text-white">{mapping.canonical_name}</span>
                    {mapping.exclude && (
                      <span className="px-2 py-0.5 text-xs rounded bg-danger-500/20 text-danger-400">
                        Excluded
                      </span>
                    )}
                    {mapping.notes && (
                      <span className="text-sm text-gray-500 italic">({mapping.notes})</span>
                    )}
                  </div>
                  <button
                    onClick={async () => {
                      await removeMapping(mapping.exchange, mapping.symbol);
                    }}
                    className="text-gray-500 hover:text-danger-400 transition-colors"
                    title="Remove mapping"
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}

        {!mappingsLoading && mappings.mappings.length === 0 && !showAddMapping && (
          <div className="text-center py-6 text-gray-500">
            <p>No symbol mappings configured.</p>
            <p className="text-sm mt-1">
              Add mappings when you notice the same symbol representing different coins across exchanges.
            </p>
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
            <li>• Bithumb</li>
            <li>• Bybit</li>
            <li>• Gate.io</li>
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
