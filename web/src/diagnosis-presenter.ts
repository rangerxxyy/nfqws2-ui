import type { SiteDiagnosticStep, SiteTracePhase } from './api';
export type Presentation = { kind: 'success' | 'failure' | 'unknown'; text: string };
export function presentStep(step: SiteDiagnosticStep): Presentation {
  if (step.status === 'unavailable') return { kind: 'unknown', text: 'Проверка недоступна' };
  if (step.httpCode !== null && step.httpCode >= 400) return { kind: 'failure', text: `Сервер вернул HTTP ${step.httpCode}` };
  if (step.status === 'warning') return { kind: 'unknown', text: 'Результат требует внимания' };
  return step.status === 'ok' ? { kind: 'success', text: 'Ответ получен' } : { kind: 'failure', text: 'Ответ не получен' };
}
export function presentTrace(phase: SiteTracePhase, route: 'direct' | 'current'): Presentation {
  if (phase.state === 'unavailable' || phase.ok === null) return { kind: 'unknown', text: 'Этап недоступен — доступность не определена' };
  if (phase.code !== null && phase.code >= 400) return { kind: 'failure', text: `Сервер вернул HTTP ${phase.code}` };
  if (!phase.ok || phase.state === 'error') return { kind: 'failure', text: route === 'direct' ? 'Не открылся напрямую' : 'Не открылся через текущую стратегию nfqws2' };
  return { kind: 'success', text: route === 'direct' ? 'Открылся напрямую' : 'Открылся через текущую стратегию nfqws2' };
}
export function comparisonVerdict(direct: SiteDiagnosticStep, current: SiteDiagnosticStep): Presentation {
  const a = presentStep(direct); const b = presentStep(current);
  if (a.kind === 'unknown' || b.kind === 'unknown') return { kind: 'unknown', text: 'Сравнение неполное — проверьте подробности' };
  if (a.kind === 'failure' && b.kind === 'success') return { kind: 'success', text: 'Сейчас ответ получен, напрямую — нет' };
  if (a.kind === 'success' && b.kind === 'failure') return { kind: 'failure', text: 'Напрямую ответ получен, текущий доступ не сработал' };
  return b.kind === 'success' ? { kind: 'success', text: 'Ответ получен обоими способами' } : { kind: 'failure', text: 'Оба способа не дали успешного ответа. Причина может быть в сети или самом сайте' };
}
export function quicExplanation(method: 'quic_initial' | 'http3', advertised: boolean | null): string {
  if (method === 'http3') return `Проверен HTTP/3; ${advertised === true ? 'сайт его заявляет.' : advertised === false ? 'сайт его не заявляет.' : 'поддержка сайта не определена.'}`;
  return `QUIC отвечает; ${advertised === true ? 'HTTP/3 заявлен.' : advertised === false ? 'HTTP/3 не заявлен.' : 'HTTP/3 не проверен.'}`;
}
