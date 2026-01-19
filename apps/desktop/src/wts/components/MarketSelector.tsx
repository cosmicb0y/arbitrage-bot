import { useState, useMemo, useRef, useEffect, useCallback } from 'react';
import type { Market } from '../types';

interface MarketSelectorProps {
  markets: readonly Market[];
  selectedMarket: string | null;
  onSelect: (marketCode: string) => void;
  disabled?: boolean;
}

export function MarketSelector({
  markets,
  selectedMarket,
  onSelect,
  disabled = false,
}: MarketSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [highlightedIndex, setHighlightedIndex] = useState(-1);
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // 검색 필터링
  const filteredMarkets = useMemo(() => {
    if (!searchQuery) return [...markets];
    const query = searchQuery.toLowerCase();
    return markets.filter(
      (m) =>
        m.code.toLowerCase().includes(query) ||
        m.base.toLowerCase().includes(query) ||
        m.displayName?.toLowerCase().includes(query)
    );
  }, [markets, searchQuery]);

  // 드롭다운 열기
  const openDropdown = useCallback(() => {
    if (!disabled) {
      setIsOpen(true);
      setSearchQuery('');
      setHighlightedIndex(-1);
    }
  }, [disabled]);

  // 드롭다운 닫기
  const closeDropdown = useCallback(() => {
    setIsOpen(false);
    setSearchQuery('');
    setHighlightedIndex(-1);
  }, []);

  // 마켓 선택
  const handleSelect = useCallback(
    (marketCode: string) => {
      onSelect(marketCode);
      closeDropdown();
    },
    [onSelect, closeDropdown]
  );

  // 키보드 탐색
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') {
        closeDropdown();
        return;
      }

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setHighlightedIndex((prev) =>
          prev < filteredMarkets.length - 1 ? prev + 1 : prev
        );
        return;
      }

      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setHighlightedIndex((prev) => (prev > 0 ? prev - 1 : prev));
        return;
      }

      if (e.key === 'Enter' && highlightedIndex >= 0) {
        e.preventDefault();
        const market = filteredMarkets[highlightedIndex];
        if (market) {
          handleSelect(market.code);
        }
        return;
      }
    },
    [closeDropdown, filteredMarkets, highlightedIndex, handleSelect]
  );

  // 외부 클릭 감지
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        closeDropdown();
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen, closeDropdown]);

  // 드롭다운 열릴 때 검색 입력에 포커스
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen]);

  return (
    <div ref={containerRef} className="relative">
      {/* 트리거 버튼 */}
      <button
        type="button"
        onClick={() => (isOpen ? closeDropdown() : openDropdown())}
        disabled={disabled}
        className={`
          px-2 py-0.5 text-xs font-mono rounded border
          transition-colors duration-150
          ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}
          ${
            isOpen
              ? 'border-wts-accent bg-wts-tertiary text-wts-text'
              : 'border-wts-border bg-wts-secondary text-wts-text hover:border-wts-accent'
          }
        `}
      >
        {selectedMarket || '마켓 선택'}
      </button>

      {/* 드롭다운 */}
      {isOpen && (
        <div className="absolute right-0 top-full mt-1 z-50 min-w-[180px] max-h-[300px] overflow-hidden rounded border border-wts-border bg-wts-secondary shadow-lg">
          {/* 검색 입력 */}
          <div className="p-1 border-b border-wts-border">
            <input
              ref={inputRef}
              type="text"
              placeholder="마켓 검색..."
              value={searchQuery}
              onChange={(e) => {
                setSearchQuery(e.target.value);
                setHighlightedIndex(-1);
              }}
              onKeyDown={handleKeyDown}
              className="w-full px-2 py-1 text-xs bg-wts-primary text-wts-text border border-wts-border rounded focus:outline-none focus:border-wts-accent"
            />
          </div>

          {/* 마켓 목록 */}
          <ul className="max-h-[240px] overflow-y-auto">
            {filteredMarkets.map((market, index) => (
              <li
                key={market.code}
                role="option"
                data-highlighted={highlightedIndex === index ? 'true' : 'false'}
                onClick={() => handleSelect(market.code)}
                className={`
                  px-2 py-1.5 cursor-pointer text-xs
                  flex items-center justify-between gap-2
                  transition-colors duration-100
                  ${
                    selectedMarket === market.code
                      ? 'bg-wts-tertiary text-wts-text'
                      : highlightedIndex === index
                        ? 'bg-wts-tertiary/50 text-wts-text'
                        : 'text-wts-text hover:bg-wts-tertiary/30'
                  }
                `}
              >
                <span className="font-mono font-medium">{market.code}</span>
                {market.displayName && (
                  <span className="text-wts-muted truncate">
                    {market.displayName}
                  </span>
                )}
              </li>
            ))}
            {filteredMarkets.length === 0 && (
              <li className="px-2 py-2 text-xs text-wts-muted text-center">
                검색 결과 없음
              </li>
            )}
          </ul>
        </div>
      )}
    </div>
  );
}
