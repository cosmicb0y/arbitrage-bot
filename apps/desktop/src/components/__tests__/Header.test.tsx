import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import Header from "../Header";
import { useStats, useBotControl } from "../../hooks/useTauri";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("../../hooks/useTauri", () => ({
  useStats: vi.fn(),
  useBotControl: vi.fn(),
}));

describe("Header", () => {
  beforeEach(() => {
    vi.mocked(useStats).mockReturnValue({
      uptime_secs: 0,
      price_updates: 0,
      opportunities_detected: 0,
      trades_executed: 0,
      is_running: false,
    });

    vi.mocked(useBotControl).mockReturnValue({
      start: vi.fn(),
      stop: vi.fn(),
    });

    vi.mocked(invoke).mockResolvedValue(undefined);
  });

  it("invokes wts_open_window when WTS button is clicked", async () => {
    render(
      <Header
        activeTab="dashboard"
        onTabChange={vi.fn()}
      />
    );

    fireEvent.click(screen.getByText("WTS 열기"));

    expect(invoke).toHaveBeenCalledWith("wts_open_window");
  });
});
