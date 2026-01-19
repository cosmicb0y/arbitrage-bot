import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { WtsApp } from '../index';
import { useWtsStore } from '../stores';

// Mock the store
vi.mock('../stores', () => ({
  useWtsStore: vi.fn(),
}));

// Mock the hooks
vi.mock('../hooks', () => ({
  useConnectionCheck: vi.fn(() => ({ checkConnection: vi.fn() })),
}));

describe('WtsApp', () => {
  beforeEach(() => {
    vi.mocked(useWtsStore).mockReturnValue({
      enabledExchanges: ['upbit'],
      setEnabledExchanges: vi.fn(),
      selectedExchange: 'upbit',
      connectionStatus: 'disconnected',
      selectedMarket: null,
      setExchange: vi.fn(),
      setMarket: vi.fn(),
      setConnectionStatus: vi.fn(),
      lastConnectionError: null,
      setConnectionError: vi.fn(),
    });
  });

  it('renders WtsApp with WtsWindow', () => {
    render(<WtsApp />);
    // WTS title is now just "WTS" in the new layout
    expect(screen.getByText('WTS')).toBeTruthy();
  });
});
