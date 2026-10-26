import { describe, expect, it } from 'vitest';
import { findEnabledOptionByPrefix, nextEnabledOption, styledSelectMenuMaxHeight, styledSelectMenuSide, styledSelectMenuWidth, type StyledOption } from './styled-select';

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

  it('uses the available height for long menus up to the compact-list maximum', () => {
    expect(styledSelectMenuMaxHeight(520)).toBe(520);
    expect(styledSelectMenuMaxHeight(800)).toBe(544);
    expect(styledSelectMenuMaxHeight(72)).toBe(72);
    expect(styledSelectMenuMaxHeight(-12)).toBe(0);
  });

  it('widens only menus with a requested minimum and clamps them to the viewport', () => {
    expect(styledSelectMenuWidth(225, 360, 1200)).toBe(360);
    expect(styledSelectMenuWidth(225, 360, 340)).toBe(324);
    expect(styledSelectMenuWidth(225, 0, 1200)).toBe(225);
  });
});
