const LOG_TAIL_THRESHOLD_PX = 24;

export function shouldFollowLogTail(hasRenderedContent: boolean, scrollTop: number, clientHeight: number, scrollHeight: number): boolean {
  if (!hasRenderedContent) return true;
  return scrollHeight - scrollTop - clientHeight <= LOG_TAIL_THRESHOLD_PX;
}
