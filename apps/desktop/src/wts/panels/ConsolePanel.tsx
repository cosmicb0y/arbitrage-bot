interface ConsolePanelProps {
  className?: string;
}

export function ConsolePanel({ className = '' }: ConsolePanelProps) {
  return (
    <div
      data-testid="console-panel"
      className={`wts-area-console wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header">Console</div>
      <div className="wts-panel-content flex-1 overflow-y-auto">
        <p className="text-wts-muted text-xs">
          콘솔 로그가 여기에 표시됩니다 (Story 1.6에서 구현)
        </p>
      </div>
    </div>
  );
}
