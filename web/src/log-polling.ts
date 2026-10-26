export function shouldPollLogs(sectionVisible: boolean, autoRefresh: boolean, requestPending: boolean): boolean {
  return sectionVisible && autoRefresh && !requestPending;
}
