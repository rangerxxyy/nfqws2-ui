import { describe, expect, it } from 'vitest';
import { findEnabledOptionByPrefix, nextEnabledOption, styledSelectMenuSide, type StyledOption } from './styled-select';

const options: StyledOption[] = [
  { value: '', label: 'Выберите', disabled: true },
  { value: 'https', label: 'HTTPS', disabled: false },
  { value: 'http', label: 'HTTP', disabled: false },
  { value: 'disabled', label: 'Недоступно', disabled: true },
];

describe('styled select navigation helpers', () => {
  it('moves between enabled options, wraps, and skips disabled options', () => {
    expect(nextEnabledOption(options, 1, 1)).toBe(2);
    expect(nextEnabledOption(options, 2, 1)).toBe(1);
    expect(nextEnabledOption(options, 1, -1)).toBe(2);
    expect(nextEnabledOption([{ value: '', label: 'Пусто', disabled: true }], 0, 1)).toBe(-1);
  });

  it('supports case-insensitive type-to-select without selecting a disabled option', () => {
    expect(findEnabledOptionByPrefix(options, 'ht', 1)).toBe(2);
    expect(findEnabledOptionByPrefix(options, 'нед', 1)).toBe(-1);
  });

  it('opens above a select near the viewport bottom when there is more room there', () => {
    expect(styledSelectMenuSide(720, 760, 220, 800)).toBe('top');
    expect(styledSelectMenuSide(120, 160, 220, 800)).toBe('bottom');
    expect(styledSelectMenuSide(350, 390, 120, 800)).toBe('bottom');
  });
});
