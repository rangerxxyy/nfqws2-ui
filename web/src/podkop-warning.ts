export type PodkopWarningPresentation = { title: string; detail: string };

export function podkopWarningPresentation(hosts: readonly string[] | undefined): PodkopWarningPresentation | null {
  const uniqueHosts = [...new Set((hosts ?? []).map((host) => host.trim()).filter(Boolean))];
  if (!uniqueHosts.length) return null;
  const one = uniqueHosts.length === 1;
  return {
    title: one ? `${uniqueHosts[0]} направляется через Podkop` : `Через Podkop направляются: ${uniqueHosts.join(', ')}`,
    detail: 'DNS вернул служебный FakeIP из диапазона 198.18.0.0/15. Проверочные запросы могут идти через туннель вместо nfqws2, поэтому результат не подходит для надёжного сравнения стратегий. Используйте сайт вне Podkop для чистого сравнения.',
  };
}
