import { useRef, useEffect, useCallback, useState } from 'react';
import { useConsoleStore } from '../stores/consoleStore';
import { ConsoleLogItem } from '../components/ConsoleLogItem';

interface ConsolePanelProps {
  className?: string;
}

export function ConsolePanel({ className = '' }: ConsolePanelProps) {
  const { logs, clearLogs } = useConsoleStore();
  const scrollRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  // 자동 스크롤: 새 로그 추가 시 하단으로 스크롤
  useEffect(() => {
    if (autoScroll && scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  // 사용자 스크롤 감지: 하단에서 50px 이내면 자동 스크롤 활성화
  const handleScroll = useCallback(() => {
    if (!scrollRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
    const isAtBottom = scrollHeight - scrollTop - clientHeight < 50;
    setAutoScroll(isAtBottom);
  }, []);

  const handleClear = useCallback(() => {
    clearLogs();
    setAutoScroll(true);
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [clearLogs]);

  return (
    <div
      data-testid="console-panel"
      className={`wts-area-console wts-panel flex flex-col ${className}`}
    >
      <div className="wts-panel-header flex items-center justify-between">
        <span>Console</span>
        <div className="flex items-center gap-2">
          <span className="text-wts-muted text-xs">{logs.length} logs</span>
          <button
            type="button"
            className="flex h-5 w-5 items-center justify-center rounded hover:bg-wts-muted/20"
            onClick={handleClear}
            title="Clear logs"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="12"
              height="12"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M3 6h18" />
              <path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
              <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
            </svg>
          </button>
        </div>
      </div>
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        className="wts-panel-content flex-1 overflow-y-auto p-2"
      >
        {logs.length === 0 ? (
          <p className="text-wts-muted text-xs italic">No logs yet</p>
        ) : (
          logs.map((log) => <ConsoleLogItem key={log.id} log={log} />)
        )}
      </div>
    </div>
  );
}
