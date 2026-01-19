import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { MarketSelector } from '../../components/MarketSelector';
import type { Market } from '../../types';

describe('MarketSelector', () => {
  const mockMarkets: Market[] = [
    { code: 'KRW-BTC', base: 'BTC', quote: 'KRW', displayName: '비트코인' },
    { code: 'KRW-ETH', base: 'ETH', quote: 'KRW', displayName: '이더리움' },
    { code: 'KRW-XRP', base: 'XRP', quote: 'KRW', displayName: '리플' },
  ];

  const defaultProps = {
    markets: mockMarkets,
    selectedMarket: null as string | null,
    onSelect: vi.fn(),
    disabled: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('렌더링', () => {
    it('트리거 버튼이 렌더링된다', () => {
      render(<MarketSelector {...defaultProps} />);
      expect(screen.getByRole('button')).toBeTruthy();
    });

    it('마켓이 선택되지 않았을 때 "마켓 선택" 텍스트가 표시된다', () => {
      render(<MarketSelector {...defaultProps} selectedMarket={null} />);
      expect(screen.getByText('마켓 선택')).toBeTruthy();
    });

    it('선택된 마켓이 있을 때 마켓 코드가 표시된다', () => {
      render(<MarketSelector {...defaultProps} selectedMarket="KRW-BTC" />);
      expect(screen.getByText('KRW-BTC')).toBeTruthy();
    });

    it('disabled 상태에서 버튼이 비활성화된다', () => {
      render(<MarketSelector {...defaultProps} disabled />);
      expect(screen.getByRole('button')).toHaveProperty('disabled', true);
    });
  });

  describe('드롭다운 동작', () => {
    it('버튼 클릭 시 드롭다운이 열린다', () => {
      render(<MarketSelector {...defaultProps} />);

      fireEvent.click(screen.getByRole('button'));

      expect(screen.getByPlaceholderText('마켓 검색...')).toBeTruthy();
    });

    it('드롭다운에 모든 마켓이 표시된다', () => {
      render(<MarketSelector {...defaultProps} />);

      fireEvent.click(screen.getByRole('button'));

      expect(screen.getByText('KRW-BTC')).toBeTruthy();
      expect(screen.getByText('KRW-ETH')).toBeTruthy();
      expect(screen.getByText('KRW-XRP')).toBeTruthy();
    });

    it('마켓의 displayName이 함께 표시된다', () => {
      render(<MarketSelector {...defaultProps} />);

      fireEvent.click(screen.getByRole('button'));

      expect(screen.getByText('비트코인')).toBeTruthy();
      expect(screen.getByText('이더리움')).toBeTruthy();
    });

    it('마켓 클릭 시 onSelect가 호출된다', () => {
      const onSelect = vi.fn();
      render(<MarketSelector {...defaultProps} onSelect={onSelect} />);

      fireEvent.click(screen.getByRole('button'));

      // 마켓 항목 찾기 - 드롭다운 내 li 요소
      const btcOption = screen.getByText('KRW-BTC').closest('li');
      fireEvent.click(btcOption!);

      expect(onSelect).toHaveBeenCalledWith('KRW-BTC');
    });

    it('마켓 선택 후 드롭다운이 닫힌다', () => {
      render(<MarketSelector {...defaultProps} />);

      fireEvent.click(screen.getByRole('button'));

      const btcOption = screen.getByText('KRW-BTC').closest('li');
      fireEvent.click(btcOption!);

      expect(screen.queryByPlaceholderText('마켓 검색...')).toBeNull();
    });

    it('disabled 상태에서 버튼 클릭해도 드롭다운이 열리지 않는다', () => {
      render(<MarketSelector {...defaultProps} disabled />);

      fireEvent.click(screen.getByRole('button'));

      expect(screen.queryByPlaceholderText('마켓 검색...')).toBeNull();
    });
  });

  describe('검색 필터링', () => {
    it('검색어 입력 시 마켓이 필터링된다', () => {
      render(<MarketSelector {...defaultProps} />);

      fireEvent.click(screen.getByRole('button'));
      fireEvent.change(screen.getByPlaceholderText('마켓 검색...'), {
        target: { value: 'BTC' },
      });

      expect(screen.getByText('KRW-BTC')).toBeTruthy();
      expect(screen.queryByText('KRW-ETH')).toBeNull();
      expect(screen.queryByText('KRW-XRP')).toBeNull();
    });

    it('displayName으로 필터링이 가능하다', () => {
      render(<MarketSelector {...defaultProps} />);

      fireEvent.click(screen.getByRole('button'));
      fireEvent.change(screen.getByPlaceholderText('마켓 검색...'), {
        target: { value: '이더' },
      });

      expect(screen.getByText('KRW-ETH')).toBeTruthy();
      expect(screen.queryByText('KRW-BTC')).toBeNull();
    });

    it('대소문자 구분 없이 검색된다', () => {
      render(<MarketSelector {...defaultProps} />);

      fireEvent.click(screen.getByRole('button'));
      fireEvent.change(screen.getByPlaceholderText('마켓 검색...'), {
        target: { value: 'btc' },
      });

      expect(screen.getByText('KRW-BTC')).toBeTruthy();
    });

    it('검색 결과가 없을 때 빈 목록이 표시된다', () => {
      render(<MarketSelector {...defaultProps} />);

      fireEvent.click(screen.getByRole('button'));
      fireEvent.change(screen.getByPlaceholderText('마켓 검색...'), {
        target: { value: 'NOTFOUND' },
      });

      expect(screen.queryByText('KRW-BTC')).toBeNull();
      expect(screen.queryByText('KRW-ETH')).toBeNull();
    });
  });

  describe('키보드 탐색', () => {
    it('ESC 키로 드롭다운이 닫힌다', () => {
      render(<MarketSelector {...defaultProps} />);

      fireEvent.click(screen.getByRole('button'));
      expect(screen.getByPlaceholderText('마켓 검색...')).toBeTruthy();

      fireEvent.keyDown(screen.getByPlaceholderText('마켓 검색...'), {
        key: 'Escape',
      });

      expect(screen.queryByPlaceholderText('마켓 검색...')).toBeNull();
    });

    it('화살표 아래 키로 다음 항목으로 이동한다', () => {
      render(<MarketSelector {...defaultProps} />);

      fireEvent.click(screen.getByRole('button'));
      fireEvent.keyDown(screen.getByPlaceholderText('마켓 검색...'), {
        key: 'ArrowDown',
      });

      // 첫 번째 항목이 하이라이트됨
      const items = screen.getAllByRole('option');
      expect(items[0].getAttribute('data-highlighted')).toBe('true');
    });

    it('Enter 키로 하이라이트된 항목이 선택된다', () => {
      const onSelect = vi.fn();
      render(<MarketSelector {...defaultProps} onSelect={onSelect} />);

      fireEvent.click(screen.getByRole('button'));
      fireEvent.keyDown(screen.getByPlaceholderText('마켓 검색...'), {
        key: 'ArrowDown',
      });
      fireEvent.keyDown(screen.getByPlaceholderText('마켓 검색...'), {
        key: 'Enter',
      });

      expect(onSelect).toHaveBeenCalledWith('KRW-BTC');
    });
  });

  describe('선택 상태 표시', () => {
    it('선택된 마켓이 하이라이트 스타일을 가진다', () => {
      render(<MarketSelector {...defaultProps} selectedMarket="KRW-BTC" />);

      fireEvent.click(screen.getByRole('button'));

      // 드롭다운 내의 옵션 요소 찾기
      const btcOptions = screen.getAllByRole('option');
      const selectedOption = btcOptions.find(
        (opt) => opt.textContent?.includes('KRW-BTC')
      );
      expect(selectedOption?.classList.contains('bg-wts-tertiary')).toBe(true);
    });
  });

  describe('외부 클릭', () => {
    it('드롭다운 외부 클릭 시 드롭다운이 닫힌다', () => {
      render(
        <div>
          <MarketSelector {...defaultProps} />
          <div data-testid="outside">Outside</div>
        </div>
      );

      fireEvent.click(screen.getByRole('button'));
      expect(screen.getByPlaceholderText('마켓 검색...')).toBeTruthy();

      fireEvent.mouseDown(screen.getByTestId('outside'));

      expect(screen.queryByPlaceholderText('마켓 검색...')).toBeNull();
    });
  });
});
