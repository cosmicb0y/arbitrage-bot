import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useConnectionCheck } from '../../hooks/useConnectionCheck';
import { useWtsStore } from '../../stores';
import { useConsoleStore } from '../../stores/consoleStore';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock stores
vi.mock('../../stores', () => ({
  useWtsStore: vi.fn(),
}));

vi.mock('../../stores/consoleStore', () => ({
  useConsoleStore: vi.fn(),
}));

describe('useConnectionCheck', () => {
  const mockSetConnectionStatus = vi.fn();
  const mockSetConnectionError = vi.fn();
  const mockAddLog = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    vi.mocked(useWtsStore).mockReturnValue({
      enabledExchanges: ['upbit'],
      setEnabledExchanges: vi.fn(),
      selectedExchange: 'upbit',
      connectionStatus: 'disconnected',
      selectedMarket: null,
      setExchange: vi.fn(),
      setMarket: vi.fn(),
      setConnectionStatus: mockSetConnectionStatus,
      lastConnectionError: null,
      setConnectionError: mockSetConnectionError,
    });

    vi.mocked(useConsoleStore).mockReturnValue({
      logs: [],
      addLog: mockAddLog,
      clearLogs: vi.fn(),
    });
  });

  it('should set status to connecting on mount', async () => {
    vi.mocked(invoke).mockResolvedValue({ success: true, latency: 50 });

    renderHook(() => useConnectionCheck());

    expect(mockSetConnectionStatus).toHaveBeenCalledWith('connecting');
  });

  it('should set status to connected on successful check', async () => {
    vi.mocked(invoke).mockResolvedValue({ success: true, latency: 100 });

    renderHook(() => useConnectionCheck());

    await waitFor(() => {
      expect(mockSetConnectionStatus).toHaveBeenCalledWith('connected');
    });

    expect(mockSetConnectionError).toHaveBeenCalledWith(null);
  });

  it('should set status to disconnected on failed check', async () => {
    vi.mocked(invoke).mockResolvedValue({
      success: false,
      error: 'Network timeout',
    });

    renderHook(() => useConnectionCheck());

    await waitFor(() => {
      expect(mockSetConnectionStatus).toHaveBeenCalledWith('disconnected');
    });

    expect(mockSetConnectionError).toHaveBeenCalledWith('Network timeout');
  });

  it('should log INFO when checking connection', async () => {
    vi.mocked(invoke).mockResolvedValue({ success: true, latency: 50 });

    renderHook(() => useConnectionCheck());

    expect(mockAddLog).toHaveBeenCalledWith(
      'INFO',
      'SYSTEM',
      '[INFO] Upbit API 연결 확인 중...'
    );
  });

  it('should log SUCCESS with latency on successful check', async () => {
    vi.mocked(invoke).mockResolvedValue({ success: true, latency: 123 });

    renderHook(() => useConnectionCheck());

    await waitFor(() => {
      expect(mockAddLog).toHaveBeenCalledWith(
        'SUCCESS',
        'SYSTEM',
        expect.stringContaining('[SUCCESS] Upbit API 연결됨')
      );
    });
  });

  it('should log ERROR on failed check', async () => {
    vi.mocked(invoke).mockResolvedValue({
      success: false,
      error: 'Connection refused',
    });

    renderHook(() => useConnectionCheck());

    await waitFor(() => {
      expect(mockAddLog).toHaveBeenCalledWith(
        'ERROR',
        'SYSTEM',
        expect.stringContaining('[ERROR] Upbit API 연결 실패')
      );
    });
  });

  it('should clear error when starting a new check', async () => {
    vi.mocked(invoke).mockResolvedValue({ success: true, latency: 50 });

    renderHook(() => useConnectionCheck());

    expect(mockSetConnectionError).toHaveBeenCalledWith(null);
  });

  it('should schedule retry with exponential backoff', async () => {
    vi.useFakeTimers();
    const timeoutSpy = vi.spyOn(window, 'setTimeout');
    vi.mocked(invoke).mockResolvedValue({
      success: false,
      error: 'Connection refused',
    });

    renderHook(() => useConnectionCheck());

    await waitFor(() => {
      expect(mockSetConnectionStatus).toHaveBeenCalledWith('disconnected');
    });

    expect(timeoutSpy).toHaveBeenCalled();
    expect(timeoutSpy.mock.calls[0][1]).toBe(1000);

    timeoutSpy.mockRestore();
    vi.useRealTimers();
  });

  it('should call invoke with correct exchange parameter', () => {
    vi.mocked(invoke).mockResolvedValue({ success: true, latency: 50 });

    renderHook(() => useConnectionCheck());

    expect(invoke).toHaveBeenCalledWith('wts_check_connection', {
      exchange: 'upbit',
    });
  });

  it('should return checkConnection function', () => {
    vi.mocked(invoke).mockResolvedValue({ success: true, latency: 50 });

    const { result } = renderHook(() => useConnectionCheck());

    expect(result.current.checkConnection).toBeInstanceOf(Function);
  });
});
