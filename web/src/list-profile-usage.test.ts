import { describe, expect, it } from 'vitest';
import { formatListProfileUsageStatus, listValidationNotice } from './list-profile-usage';

describe('formatListProfileUsageStatus', () => {
  it('describes a skipped format check for the list tooltip', () => {
    expect(listValidationNotice(false))
      .toBe('Формат списка не проверяется при сохранении: ссылка на него в аргументах конфигурации не найдена.');
    expect(listValidationNotice(true)).toBe('');
  });

  it('shows the independent validation decision when profile usage is unknown', () => {
    expect(formatListProfileUsageStatus({ usageKnown: false, validationRequired: false, usages: [] }))
      .toBe('Использование в профилях неизвестно: конфигурацию не удалось безопасно разобрать. Формат списка не проверяется при сохранении: ссылка на него в аргументах конфигурации не найдена.');
  });

  it('shows the no-reference validation reason when the list is not used', () => {
    expect(formatListProfileUsageStatus({ usageKnown: true, validationRequired: false, usages: [] }))
      .toBe('Не подключён ни к одному профилю. Формат списка не проверяется при сохранении: ссылка на него в аргументах конфигурации не найдена.');
  });

  it('lists profiles that use the file when usage data is available', () => {
    expect(formatListProfileUsageStatus({ usageKnown: true, validationRequired: true, usages: ['#1 Основной · include'] }))
      .toBe('Используют: #1 Основной · include');
  });
});
