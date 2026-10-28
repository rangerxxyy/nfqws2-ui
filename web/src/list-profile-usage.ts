export interface ListProfileUsageStatusInput {
  usageKnown?: boolean;
  validationRequired?: boolean;
  usages: string[];
}

export function listValidationNotice(validationRequired?: boolean): string {
  return validationRequired === false
    ? 'Формат списка не проверяется при сохранении: ссылка на него в аргументах конфигурации не найдена.'
    : '';
}

export function formatListProfileUsageStatus({ usageKnown, validationRequired, usages }: ListProfileUsageStatusInput): string {
  const usageStatus = usageKnown === false
    ? 'Использование в профилях неизвестно: конфигурацию не удалось безопасно разобрать.'
    : usages.length > 0
      ? `Используют: ${usages.join('; ')}`
      : validationRequired === false
        ? 'Не подключён ни к одному профилю.'
        : 'Не используется ни одним разобранным профилем.';
  const validationNotice = listValidationNotice(validationRequired);
  return validationNotice ? `${usageStatus} ${validationNotice}` : usageStatus;
}
