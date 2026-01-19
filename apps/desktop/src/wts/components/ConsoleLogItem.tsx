import { memo, useState } from 'react';
import type { ConsoleLogEntry } from '../types';
import { formatLogTimestamp } from '../utils/formatters';
import { LOG_LEVEL_STYLES, LOG_CATEGORY_STYLES } from '../utils/consoleStyles';

interface ConsoleLogItemProps {
  log: ConsoleLogEntry;
}

export const ConsoleLogItem = memo(function ConsoleLogItem({
  log,
}: ConsoleLogItemProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const hasDetail = log.detail !== undefined;

  return (
    <div className="py-0.5">
      <div
        className={`flex items-start gap-2 font-mono text-xs ${LOG_LEVEL_STYLES[log.level]}`}
      >
        <span className="shrink-0 text-wts-muted">
          {formatLogTimestamp(log.timestamp)}
        </span>
        <span
          className={`shrink-0 rounded px-1 text-[10px] ${LOG_CATEGORY_STYLES[log.category]}`}
        >
          [{log.category}]
        </span>
        <span className="break-words">{log.message}</span>
        {hasDetail && (
          <button
            type="button"
            className="ml-auto shrink-0 text-wts-muted hover:text-wts-foreground"
            onClick={() => setIsExpanded(!isExpanded)}
            title={isExpanded ? 'Hide details' : 'Show details'}
          >
            {isExpanded ? '▼' : '▶'}
          </button>
        )}
      </div>
      {hasDetail && isExpanded && (
        <pre className="ml-20 mt-1 overflow-x-auto rounded bg-wts-background/50 p-2 text-[10px] text-wts-muted">
          {typeof log.detail === 'string'
            ? log.detail
            : JSON.stringify(log.detail, null, 2)}
        </pre>
      )}
    </div>
  );
});
