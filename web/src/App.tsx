import { useEffect } from 'react';
import { bootPanel } from './panel';

export default function App() {
  useEffect(() => { bootPanel(); }, []);
  return (
    <>
{"\n\n  "}
{"\n  "}
{"\n  "}
{"\n  "}
    <header className={"bg-[#090e1a]/95 backdrop-blur-md border-b border-slate-800/90 shrink-0 z-40 px-4 lg:px-6 py-2.5"}>
      {"\n    "}
      <div className={"max-w-[1920px] mx-auto grid grid-cols-1 lg:grid-cols-[repeat(3,minmax(0,1fr))] 2xl:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-3"}>
        {"\n      \n      "}
        {"\n      "}
        <div id={"headerStatusBar"} className={"min-w-0 flex flex-nowrap whitespace-nowrap items-center gap-2 justify-start overflow-x-auto py-1 lg:col-span-3 2xl:col-span-1"}>
          {"\n        \n        "}
          {"\n        "}
          <div className={"status-pill flex items-center gap-2.5 bg-[#060a12] border border-slate-800/90 px-3 py-2 rounded-lg shadow-inner"}>
            {"\n          "}
            <div id={"statusIndicator"} className={"relative flex items-center justify-center w-3 h-3"}>
              {"\n            "}
              <span id={"statusRadarRing"} className={"absolute inline-flex h-full w-full rounded-full bg-slate-400"}></span>
              {"\n            "}
              <span id={"statusMainDot"} className={"relative inline-flex rounded-full h-2.5 w-2.5 bg-slate-400"}></span>
              {"\n          "}
            </div>
            {"\n          "}
            <span id={"statusTextLabel"} aria-live={"polite"} className={"text-[11px] sm:text-xs font-semibold text-slate-400 tracking-tight"}>
              {"Статус неизвестен"}
            </span>
            <span id={"statusDetail"} className={"sr-only"}></span>
            {"\n        "}
          </div>
          {"\n\n        "}
          {"\n        "}
          <div id={"headerServiceControls"} className={"shrink-0 flex items-center gap-1"}>
          <button data-action-click={"restartService"} data-action-tooltip={"Перезапустить nfqws2"} title={"Перезапустить nfqws2"} aria-label={"Перезапустить nfqws2"} className={"h-9 w-9 rounded-lg bg-slate-800/90 hover:bg-slate-700 hover:-translate-y-0.5 text-slate-100 border border-slate-700 flex items-center justify-center transition-all active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"}>
            {"\n          "}
            <svg id={"restartIcon"} aria-hidden={"true"} className={"w-[18px] h-[18px] text-blue-400 transition-transform duration-500"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"}></path>
              {"\n          "}
            </svg>
            {"\n          "}
            {"\n        "}
          </button>
          {"\n\n        "}
          {"\n        "}
          <button id={"togglePowerBtn"} data-action-click={"toggleDaemonPower"} data-action-tooltip={"Запустить nfqws2"} aria-label={"Запустить nfqws2"} className={"h-9 w-9 rounded-lg bg-emerald-500/10 hover:bg-emerald-500/20 hover:-translate-y-0.5 text-emerald-400 border border-emerald-500/30 flex items-center justify-center transition-all active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-400"}>
            {"\n          "}
            <svg aria-hidden={"true"} className={"w-[18px] h-[18px]"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <path data-power-icon={"play"} d={"M8 5.5v13L18 12 8 5.5Z"} fill={"currentColor"}></path>
              <rect data-power-icon={"stop"} className={"hidden"} x={"7"} y={"7"} width={"10"} height={"10"} rx={"1"} fill={"currentColor"}></rect>
              {"\n          "}
            </svg>
            {"\n          "}
            <span id={"togglePowerText"} className={"sr-only"}>
              {"Запустить nfqws2"}
            </span>
            {"\n        "}
          </button>
          </div>
          <div className={"metric-group flex items-center gap-2.5"}>
            {"\n          \n          "}
            {"\n          "}
            <div className={"metric-chip min-w-[150px] bg-[#060a12] border border-slate-800/90 px-3 py-2 rounded-lg flex items-center gap-2.5"}>
              {"\n            "}
              <div className={"p-1 rounded-md bg-sky-500/10 text-sky-400 border border-sky-500/20"} title={"Оперативная память роутера"}>
                {"\n              "}
                <svg className={"w-3.5 h-3.5"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  {"\n                "}
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"1.9"} d={"M3 7h18v7H3V7zm3 7v3m4-3v3m4-3v3m4-3v3M7 10h2m4 0h2"}></path>
                  {"\n              "}
                </svg>
                {"\n            "}
              </div>
              {"\n            "}
              <div>
                {"\n              "}
                <div className={"flex items-center gap-1.5 text-xs font-mono"}>
                  {"\n                "}
                  <span className={"metric-label text-slate-400 font-semibold"}>
                    {"RAM"}
                  </span>
                  {"\n                "}
                  <span id={"ramMetricText"} className={"text-slate-100 font-bold"}>
                    {"—"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <div className={"metric-track w-28 sm:w-32 bg-slate-800/90 h-1.5 rounded-full overflow-hidden mt-1.5"}>
                  {"\n                "}
                  <div id={"ramProgressBar"} className={"bg-sky-400 h-full rounded-full transition-all duration-500"} style={{ width: "0%" }}></div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"metric-chip min-w-[108px] bg-[#060a12] border border-slate-800/90 px-3 py-2 rounded-lg flex items-center gap-2.5"}>
              {"\n            "}
              <div className={"p-1 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"} title={"Загрузка процессора"}>
                {"\n              "}
                <svg className={"w-3.5 h-3.5"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  {"\n                "}
                  <rect x={"6"} y={"6"} width={"12"} height={"12"} rx={"2"} strokeWidth={"1.9"}></rect>
                  {"\n                "}
                  <rect x={"9"} y={"9"} width={"6"} height={"6"} rx={"1"} strokeWidth={"1.7"}></rect>
                  {"\n                "}
                  <path strokeLinecap={"round"} strokeWidth={"1.9"} d={"M9 3v3m6-3v3M9 18v3m6-3v3M3 9h3m-3 6h3m12-6h3m-3 6h3"}></path>
                  {"\n              "}
                </svg>
                {"\n            "}
              </div>
              {"\n            "}
              <div>
                {"\n              "}
                <div className={"flex items-center gap-1.5 text-xs font-mono"}>
                  {"\n                "}
                  <span className={"metric-label text-slate-400 font-semibold"}>
                    {"CPU"}
                  </span>
                  {"\n                "}
                  <span id={"cpuMetricText"} className={"text-emerald-400 font-bold"}>
                    {"—"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <div className={"metric-track w-16 sm:w-20 bg-slate-800/90 h-1.5 rounded-full overflow-hidden mt-1.5"}>
                  {"\n                "}
                  <div id={"cpuProgressBar"} className={"bg-emerald-400 h-full rounded-full transition-all duration-500"} style={{ width: "0%" }}></div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n        "}
          </div>
          {"\n      "}
        </div>
        {"\n\n      "}
        {"\n      "}
        <div className={"flex items-center justify-center whitespace-nowrap order-first lg:order-none lg:col-start-2 lg:row-start-2 2xl:col-start-auto 2xl:row-start-auto"}>
          {"\n        "}
          <span className={"text-xl lg:text-2xl font-bold tracking-tight text-white select-none transition-transform duration-300 hover:scale-[1.03]"}>
            {"\n          nfqws2"}
            <span className={"text-blue-400 drop-shadow-[0_0_8px_rgba(96,165,250,0.35)]"}>
              {"-UI"}
            </span>
            {"\n        "}
          </span>
          {"\n      "}
        </div>
        {"\n\n      "}
        {"\n      "}
        <div id={"headerUpdates"} className={"flex flex-nowrap items-center justify-center gap-3 lg:justify-end lg:pr-2 lg:col-start-3 lg:row-start-2 lg:-translate-x-1 2xl:col-start-auto 2xl:row-start-auto 2xl:-translate-x-1"}>
          {"\n        \n        "}
          {"\n        "}
          <button id={"updateNfqws2Button"} data-action-click={"openUpdateModal"} data-args={JSON.stringify(["core"])} title={"Обновить бинарное ядро nfqws2"} className={"version-pill px-3.5 py-2 rounded-xl flex items-center gap-2 text-xs sm:text-sm cursor-pointer"}>
            {"\n          "}
            <svg className={"w-4 h-4 text-slate-200"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <rect x={"6"} y={"6"} width={"12"} height={"12"} rx={"2"} strokeWidth={"1.9"}></rect>
              {"\n            "}
              <rect x={"9.5"} y={"9.5"} width={"5"} height={"5"} rx={"0.8"} strokeWidth={"1.7"}></rect>
              {"\n            "}
              <path strokeLinecap={"round"} strokeWidth={"1.9"} d={"M9 3v3m6-3v3M9 18v3m6-3v3M3 9h3m-3 6h3m12-6h3m-3 6h3"}></path>
              {"\n          "}
            </svg>
            {"\n          "}
            <span className={"font-bold text-white tracking-tight"}>
              {"nfqws2"}
            </span>
            {"\n          "}
            <span id={"headerCoreVersion"} className={"font-mono text-slate-400 font-medium"}>
              {"—"}
            </span>
            {"\n        "}
          </button>
          {"\n\n        "}
          {"\n        "}
          <button id={"updateUiButton"} data-action-click={"openUpdateModal"} data-args={JSON.stringify(["ui"])} title={"Обновить веб-панель nfqws2-UI"} className={"version-pill px-3.5 py-2 rounded-xl flex items-center gap-2 text-xs sm:text-sm cursor-pointer"}>
            {"\n          "}
            <svg className={"w-4 h-4 text-slate-200"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"1.9"} d={"M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"}></path>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"1.9"} d={"M3.27 6.96L12 12.01l8.73-5.05M12 22.08V12"}></path>
              {"\n          "}
            </svg>
            {"\n          "}
            <span className={"font-bold text-white"}>UI</span><span id={"headerUiVersion"} className={"font-bold text-white font-mono"}>
              {"—"}
            </span>
            {"\n        "}
          </button>
          {"\n\n        "}
          <div className={"h-5 w-[1px] bg-slate-800 mx-0.5 hidden sm:block"}></div>
          {"\n\n        "}
          {"\n        "}

          {"\n\n      "}
        </div>
        {"\n\n    "}
      </div>
      {"\n  "}
    </header>
{"\n\n  "}
{"\n  "}
{"\n  "}
{"\n  "}
    <div className={"app-main-layout flex-1 min-h-0 flex overflow-hidden max-w-[1920px] w-full mx-auto"}>
      {"\n\n    "}
      {"\n    "}
      <aside className={"w-64 lg:w-72 bg-[#090e1a] border-r border-slate-800/90 flex flex-col justify-between shrink-0 overflow-y-auto select-none"}>
        {"\n      \n      "}
        <div className={"p-3.5 space-y-5"}>
          {"\n        \n        "}
          {"\n        "}
          <div>
            {"\n          "}
            <div className={"px-3 mb-2 text-[11px] font-mono font-bold tracking-wider text-slate-500"}>
              {"\n            Управление и правила\n          "}
            </div>
            {"\n          "}
            <nav className={"space-y-1"}>
              {"\n            \n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["conf"])} id={"navBtn-conf"} className={"side-nav-btn active w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-semibold border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Конфигурация"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-[11px] font-mono px-1.5 py-0.5 rounded bg-blue-500/20 text-blue-300"}>
                  {"conf"}
                </span>
                {"\n            "}
              </button>
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["profiles"])} id={"navBtn-profiles"} className={"side-nav-btn ml-3 w-[calc(100%-0.75rem)] flex items-center gap-3 border-l-2 border-slate-700 px-3.5 py-2 rounded-r-lg text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 text-left"}><span className={"text-violet-300"}>▤</span><span>Конструктор стратегий</span></button>
              {"\n\n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["lists"])} id={"navBtn-lists"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-sky-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Списки доменов и IP"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-[11px] font-mono px-1.5 py-0.5 rounded bg-slate-800 text-slate-300"}>
                  {"5"}
                </span>
                {"\n            "}
              </button>
              {"\n\n          "}

            </nav>
            {"\n        "}
          </div>
          {"\n\n        "}
          {"\n        "}
          <div id={"diagnosticNavGroup"}>
            {"\n          "}
            <div className={"px-3 mb-2 text-[11px] font-mono font-bold tracking-wider text-slate-500"}>
              {"\n            Диагностика и автоматика\n          "}
            </div>
            {"\n          "}
            <nav className={"space-y-1"}>
              {"\n            \n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["check"])} id={"navBtn-check"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-emerald-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Проверка сайта"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-[10px] font-mono px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-400"}>
                  {"Диагноз"}
                </span>
                {"\n            "}
              </button>
              {"\n\n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["trace"])} id={"navBtn-trace"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                <span className={"flex items-center gap-3"}><span className={"text-blue-400"}>⌁</span><span>Трассировка</span></span>
                <span className={"text-[10px] font-mono px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-300"}>--debug</span>
              </button>
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["autotune"])} id={"navBtn-autotune"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-amber-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M13 10V3L4 14h7v7l9-11h-7z"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Авто-подбор стратегии"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span className={"text-[10px] font-mono px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-300"}>
                  {"TCP"}
                </span>
                {"\n            "}
              </button>
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["diagnostic16k"])} id={"navBtn-diagnostic16k"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                <span className={"flex items-center gap-3"}><span className={"text-cyan-300"}>↔</span><span>Диагностика 16 КБ</span></span>
                <span className={"text-[10px] font-mono px-1.5 py-0.5 rounded bg-cyan-500/15 text-cyan-300"}>HTTPS</span>
              </button>
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["configCheck"])} id={"navBtn-configCheck"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                <span className={"flex items-center gap-3"}><span className={"text-violet-300"}>⇄</span><span>Проверка конфигурации</span></span>
                <span className={"text-[10px] font-mono px-1.5 py-0.5 rounded bg-violet-500/15 text-violet-300"}>.conf</span>
              </button>
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["community"])} id={"navBtn-community"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}><span className={"flex items-center gap-3"}><span className={"text-teal-300"}>◎</span><span>Стратегии сообщества</span></span><span className={"text-[10px] font-mono px-1.5 py-0.5 rounded bg-teal-500/15 text-teal-300"}>ASN</span></button>
              {"\n\n            "}
              {"\n            "}
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["monitor"])} id={"navBtn-monitor"} className={"side-nav-btn w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                {"\n              "}
                <div className={"flex items-center gap-3"}>
                  {"\n                "}
                  <svg className={"w-4 h-4 shrink-0 text-cyan-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"}></path>
                  </svg>
                  {"\n                "}
                  <span>
                    {"Мониторинг"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <span id={"sideMonitorBadge"} className={"w-2 h-2 rounded-full bg-slate-400"}></span>
                {"\n            "}
              </button>
              {"\n\n            "}
              {"\n            "}
            </nav>
            {"\n        "}
          </div>
          {"\n\n        "}
          {"\n        "}

          {"\n\n      "}
          <div>
            {"\n          "}
            <div className={"px-3 pt-3 pb-1 text-[11px] font-mono font-bold tracking-wider text-slate-500"}>Система</div>
            <nav className={"space-y-1"}>

              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["appearance"])} id={"navBtn-appearance"} className={"side-nav-btn w-full flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                <svg className={"w-4 h-4 shrink-0 text-violet-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}><path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M12 3v2m0 14v2M3 12h2m14 0h2M5.64 5.64l1.42 1.42m9.88 9.88 1.42 1.42m0-12.72-1.42 1.42m-9.88 9.88-1.42 1.42M12 8a4 4 0 100 8 4 4 0 000-8z"}></path></svg><span>Настройки и оформление UI</span>
              </button>
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["about"])} id={"navBtn-about"} className={"side-nav-btn w-full flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}>
                <svg className={"w-4 h-4 shrink-0 text-sky-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}><circle cx={"12"} cy={"12"} r={"9"} strokeWidth={"2"}></circle><path strokeLinecap={"round"} strokeWidth={"2"} d={"M12 11v5m0-8h.01"}></path></svg><span>О nfqws2-UI</span>
              </button>
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["snapshots"])} id={"navBtn-snapshots"} className={"side-nav-btn w-full flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}><span className={"text-amber-300"}>◷</span><span>Резервные копии</span></button>
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["changes"])} id={"navBtn-changes"} className={"side-nav-btn w-full flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}><span className={"text-cyan-300"}>↺</span><span>Журнал изменений</span></button>
              <button data-action-click={"switchMainTab"} data-args={JSON.stringify(["logs"])} id={"navBtn-logs"} className={"side-nav-btn w-full flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-sm font-medium text-slate-400 hover:text-slate-100 hover:bg-slate-800/50 border border-transparent text-left"}><span className={"text-emerald-300"}>≡</span><span>Журнал</span></button>
            </nav>
          </div>
          {"\n\n        "}
        </div>
        {"\n\n      "}
        {"\n      "}
        {"\n\n    "}
      </aside>
      {"\n\n    "}
      {"\n    "}
      {"\n    "}
      {"\n    "}
      <main className={"flex-1 min-h-0 overflow-y-auto p-4 lg:p-6 bg-slate-950"}>
        <nav id={"simplifiedNavigation"} className={"hidden simplified-nav mb-4 rounded-xl border border-slate-800 bg-slate-900/70 p-3"} aria-label={"Упрощённая навигация"}>
          <div className={"simplified-nav-groups"} role={"group"} aria-label={"Группы разделов"}>
            {[['configuration', 'Конфигурация'], ['site-check', 'Проверка сайта'], ['tuning', 'Подбор'], ['monitoring', 'Мониторинг'], ['lists', 'Списки'], ['system', 'Система']].map(([group, label], index) => <button key={group} type={"button"} data-simplified-group={group} data-action-click={"switchSimplifiedNavGroup"} data-args={JSON.stringify([group])} aria-pressed={index === 0 ? "true" : "false"} className={"simplified-nav-button"}>{label}</button>)}
          </div>
          <div className={"simplified-nav-pages"} role={"group"} aria-label={"Страницы группы"}>
            {[['conf', 'Конфигурация'], ['profiles', 'Конструктор стратегий'], ['check', 'Проверка сайта'], ['trace', 'Трассировка'], ['diagnostic16k', 'Диагностика 16 КБ'], ['configCheck', 'Проверка конфигурации'], ['community', 'Стратегии сообщества'], ['autotune', 'Авто-подбор'], ['monitor', 'Мониторинг'], ['lists', 'Списки'], ['snapshots', 'Резервные копии'], ['changes', 'Журнал изменений'], ['logs', 'Журнал'], ['appearance', 'Настройки UI'], ['about', 'О панели']].map(([route, label]) => <button key={route} type={"button"} data-simplified-route={route} data-action-click={"switchMainTab"} data-args={JSON.stringify([route])} className={"simplified-nav-button hidden"}>{label}</button>)}
          </div>
        </nav>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-conf"} className={"view-animate"}>
          {"\n        "}
          <div className={"ui-card mb-3 flex flex-wrap items-center justify-between gap-3 rounded-xl px-4 py-3"}>
            <div className={"flex items-center gap-2.5"}>
              <span className={"config-section-indicator h-2.5 w-2.5 rounded-full"}></span>
              <span id={"currentConfigPath"} className={"font-mono text-sm font-bold tracking-tight text-white sm:text-base"}>{"Конфигурация"}</span>
              <span id={"confLinesBadge"} className={"rounded-md border border-slate-700 bg-slate-800 px-2 py-0.5 font-mono text-xs text-slate-300"}>{"— строк"}</span>
              <span id={"configWarningsBadge"} className={"hidden rounded-full border border-amber-500/30 bg-amber-500/10 px-2.5 py-1 text-xs font-semibold text-amber-300"} aria-live={"polite"}></span>
            </div>
          </div>
          <nav aria-label={"Раздел конфигурации"} className={"config-subtabs mb-4 flex flex-wrap items-center gap-2 rounded-xl border border-slate-800 bg-[#090e1a] p-1.5"}>
            <button id={"configTabEditor"} type={"button"} aria-pressed={"true"} aria-controls={"configEditorPanel"} data-action-click={"switchConfigSection"} data-args={JSON.stringify(["editor"])} className={"config-subtab config-subtab-active rounded-lg px-4 py-2 text-sm font-semibold transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"}>
              {"Конфигурация"}
            </button>
            <button id={"configTabSettings"} type={"button"} aria-pressed={"false"} aria-controls={"configSettingsPanel"} data-action-click={"switchConfigSection"} data-args={JSON.stringify(["settings"])} className={"config-subtab rounded-lg px-4 py-2 text-sm font-semibold transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"}>
              {"Настройки конфигурации"}
            </button>
            <button id={"configTabScripts"} type={"button"} aria-pressed={"false"} aria-controls={"configScriptsPanel"} data-action-click={"switchConfigSection"} data-args={JSON.stringify(["scripts"])} className={"config-subtab rounded-lg px-4 py-2 text-sm font-semibold transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"}>
              {"Скрипты"}
            </button>
            <div className={"ml-auto flex flex-wrap items-center gap-2"}>
              <button id={"copyConfigTextButton"} data-action-click={"copyConfigText"} className={"rounded-lg border border-slate-700 bg-slate-800/90 px-3 py-1.5 text-xs font-medium text-slate-200 transition-all hover:bg-slate-700"}>{"Копировать"}</button>
              <button data-action-click={"applyConfigAndRestart"} className={"rounded-lg bg-blue-600 px-4 py-1.5 text-xs font-semibold text-white shadow-md shadow-blue-600/25 transition-all hover:bg-blue-500 active:scale-95"}>{"Применить конфигурацию"}</button>
            </div>
          </nav>
          <div className={"grid grid-cols-1 xl:grid-cols-12 gap-5 items-start"}>
            {"\n          \n          "}
            {"\n          "}
            <div id={"configEditorPanel"} className={"config-section-panel config-panel-enter xl:col-span-12 ui-card rounded-2xl overflow-hidden shadow-2xl border border-slate-800 flex flex-col"}>
              {"\n            \n            "}
              {"\n            "}
              {"\n            "}
              <div className={"bg-[#070b14] px-4 py-2 border-b border-slate-800/80 flex flex-wrap items-center justify-between gap-2 text-xs font-mono"}>
                {"\n              "}

                {"\n\n              "}
                <label className={"flex items-center gap-1.5 text-xs text-slate-400 cursor-pointer select-none"}>
                  {"\n                "}
                  <input id={"configWordWrapToggle"} type={"checkbox"} data-action-change={"toggleWordWrapConf"} data-args={JSON.stringify([{"source": "checked"}])} defaultChecked={true} className={"rounded bg-slate-900 border-slate-700 text-blue-500 focus:ring-0"} />
                  {"\n                "}
                  <span>
                    {"Перенос строк"}
                  </span>
                  {"\n              "}
                </label>
                <div className={"flex items-center gap-2"}>
                  <button id={"configStructureToggle"} type={"button"} aria-pressed={"true"} aria-expanded={"true"} aria-controls={"configOutline"} data-action-click={"toggleConfigOutline"} title={"Скрыть структуру переменных конфигурации справа"} className={"config-structure-toggle-active rounded-lg border border-slate-700 bg-slate-800/90 px-3 py-1.5 text-xs font-medium text-slate-200 transition-all hover:border-blue-500/70 hover:bg-slate-700 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"}>
                    {"Структура"}
                  </button>
                  <button type={"button"} data-action-click={"requestClearConfig"} aria-label={"Очистить конфигурацию"} title={"Очистить конфигурацию"} className={"config-editor-icon-button"}>
                    <svg aria-hidden={"true"} viewBox={"0 0 24 24"} fill={"none"} stroke={"currentColor"} strokeWidth={"1.8"} strokeLinecap={"round"} strokeLinejoin={"round"}><path d={"M3 6h18M8 6V4h8v2m-9 0 1 14h10l1-14M10 11v5m4-5v5"}></path></svg>
                  </button>
                  <button id={"configEditorFullscreenToggle"} type={"button"} data-action-click={"toggleConfigEditorFullscreen"} aria-label={"Развернуть редактор"} title={"Развернуть редактор"} aria-pressed={"false"} className={"config-editor-icon-button"}>
                    <svg aria-hidden={"true"} viewBox={"0 0 24 24"} fill={"none"} stroke={"currentColor"} strokeWidth={"1.8"} strokeLinecap={"round"} strokeLinejoin={"round"}><path d={"M8 3H5a2 2 0 0 0-2 2v3m13-5h3a2 2 0 0 1 2 2v3M3 16v3a2 2 0 0 0 2 2h3m13-5v3a2 2 0 0 1-2 2h-3"}></path></svg>
                  </button>
                </div>
                {"\n            "}
              </div>
              {"\n\n            "}
              {"\n            "}
              <div id={"configEditorWorkspace"} className={"config-editor-workspace config-editor-workspace-open grid grid-cols-1 gap-3"}>
                <div className={"mobile-code-editor flex h-[740px] min-w-0 bg-[#060a12] font-mono text-[14px] leading-[1.68] overflow-hidden"}>
                  {"\n              "}
                  <div id={"confLineNumbers"} className={"line-numbers-col bg-[#05080f] border-r border-slate-800/80 py-4 px-3 w-13 overflow-hidden select-none text-xs leading-[1.68]"}>
                    {"\n                1\n              "}
                  </div>
                  {"\n              "}
                  <div className={"conf-editor-surface relative min-w-0 flex-1 overflow-hidden"}>
                    <pre id={"confSyntaxHighlight"} aria-hidden={"true"} className={"conf-syntax-highlight absolute inset-0 m-0 overflow-hidden py-4 px-5 font-mono leading-[1.68]"}></pre>
                    <textarea id={"nfqwsConfTextarea"} data-action-input={"handleConfInput"} data-action-scroll={"syncConfScroll"} data-args={JSON.stringify([{"source": "self"}])} spellCheck={"false"} className={"conf-editor-input absolute inset-0 min-w-0 w-full h-full bg-transparent py-4 px-5 outline-none resize-none leading-[1.68] font-mono overflow-y-auto focus:ring-0 border-none"}></textarea>
                  </div>
                  {"\n            "}
                </div>
                <aside id={"configOutline"} aria-label={"Структура конфигурации"} className={"config-outline max-h-[740px] overflow-y-auto rounded-xl border border-slate-800 bg-[#070b14] p-2.5"}>
                  <div className={"mb-2 px-2 text-[11px] font-bold uppercase tracking-wider text-slate-400"}>{"Структура"}</div>
                  <div id={"configOutlineEntries"} className={"space-y-0.5"}>
                    <div className={"px-2 py-2 text-xs text-slate-500"}>{"Загрузка структуры…"}</div>
                  </div>
                </aside>
              </div>
              {"\n\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <section id={"configWarningsPanel"} className={"hidden config-warnings-panel ui-card xl:col-span-12 rounded-xl border border-amber-500/20 p-3 sm:p-4"} aria-label={"Предупреждения конфигурации"}>
              <div className={"mb-2 flex items-center justify-between gap-2 border-b border-amber-500/15 pb-2"}><h3 className={"text-sm font-semibold text-amber-200"}>Предупреждения конфигурации</h3><span id={"configWarningsSummary"} className={"text-xs text-slate-400"} aria-live={"polite"}></span></div>
              <div id={"configWarningsList"} className={"grid gap-1.5"}></div>
            </section>
            <div id={"configSettingsPanel"} className={"config-settings-panel config-section-panel hidden xl:col-span-12 grid grid-cols-1 xl:grid-cols-2 gap-4"}>
              {"\n            \n            "}
              {"\n            "}
              <div className={"ui-card rounded-2xl p-4 shadow-lg"}>
                {"\n              "}
                <div className={"flex items-center justify-between pb-3 mb-3 border-b border-slate-800"}>
                  {"\n                "}
                  <div className={"flex items-center gap-2"}>
                    {"\n                  "}
                    <span className={"w-2 h-4 rounded-full bg-blue-500"}></span>
                    {"\n                  "}
                    <h3 className={"text-sm font-bold text-white"}>
                      {"Быстрые параметры основы"}
                    </h3>
                    {"\n                "}
                  </div>
                  {"\n                "}
                  <button data-action-click={"resetDefaultConf"} className={"text-xs text-slate-400 hover:text-blue-400 transition-colors"}>
                    {"\n                  Сбросить к эталону\n                "}
                  </button>
                  {"\n              "}
                </div>
                {"\n\n              "}
                <div className={"grid grid-cols-2 gap-3"}>
                  {"\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>{"ISP_INTERFACE"}</span>
                      {"\n                    "}
                      <span className={"text-blue-400"}>{"WAN"}</span>
                      {"\n                  "}
                    </div>
                    <label htmlFor={"ispInterfaceSelect"} className={"sr-only"}>{"Интерфейс провайдера (ISP_INTERFACE)"}</label>
                    <select id={"ispInterfaceSelect"} defaultValue={""} data-key={"ISP_INTERFACE"} data-action-change={"selectIspInterface"} data-args={JSON.stringify([{"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"}>
                      <option value={""}>{"Загрузка интерфейсов…"}</option>
                    </select>
                    <div id={"ispInterfaceManualEntry"} className={"hidden mt-2"}>
                      <label htmlFor={"ispInterfaceManualInput"} className={"sr-only"}>{"Введите имя интерфейса ISP_INTERFACE"}</label>
                      <input id={"ispInterfaceManualInput"} type={"text"} maxLength={15} autoComplete={"off"} spellCheck={false} placeholder={"Например, eth3 или pppoe-wan"} data-key={"ISP_INTERFACE"} data-action-change={"updateIspInterfaceManually"} data-args={JSON.stringify([{"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"} />
                    </div>
                    {"\n                "}
                  </div>
                  {"\n\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"NFQWS_EXTRA_ARGS"}
                      </span>
                      {"\n                    "}
                      <span className={"text-blue-400"}>
                        {"РЕЖИМ"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={""} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["NFQWS_EXTRA_ARGS", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-blue-300 focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"$MODE_LIST"}>
                        {"$MODE_LIST (user.list)"}
                      </option>
                      {"\n                    "}
                      <option value={"$MODE_AUTO"}>
                        {"$MODE_AUTO (auto.list)"}
                      </option>
                      {"\n                    "}
                      <option value={"$MODE_ALL"}>
                        {"$MODE_ALL (Весь трафик)"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"NFQUEUE_NUM"}
                      </span>
                      {"\n                    "}
                      <span className={"text-amber-400"}>
                        {"QUEUE"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={""} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["NFQUEUE_NUM", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"300"}>
                        {"300 (Штатная)"}
                      </option>
                      {"\n                    "}
                      <option value={"200"}>
                        {"200"}
                      </option>
                      {"\n                    "}
                      <option value={"100"}>
                        {"100"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"POLICY_NAME"}
                      </span>
                      {"\n                    "}
                      <span className={"text-indigo-400"}>
                        {"NDM"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={""} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["POLICY_NAME", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"nfqws"}>
                        {"nfqws"}
                      </option>
                      {"\n                    "}
                      <option value={"zapret"}>
                        {"zapret"}
                      </option>
                      {"\n                    "}
                      <option value={"default"}>
                        {"default"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"IPV6_ENABLED"}
                      </span>
                      {"\n                    "}
                      <span>
                        {"IPv6"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={""} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["IPV6_ENABLED", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"0"}>
                        {"0 (Выключен)"}
                      </option>
                      {"\n                    "}
                      <option value={"1"}>
                        {"1 (Включен)"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n\n                "}
                  {"\n                "}
                  <div className={"bg-[#070b14] border border-slate-800/90 rounded-xl p-2.5"}>
                    {"\n                  "}
                    <div className={"flex items-center justify-between text-[11px] font-mono text-slate-400"}>
                      {"\n                    "}
                      <span>
                        {"LOG_LEVEL"}
                      </span>
                      {"\n                    "}
                      <span>
                        {"ЛОГ"}
                      </span>
                      {"\n                  "}
                    </div>
                    {"\n                  "}
                    <select defaultValue={""} data-action-change={"updateConfVariable"} data-args={JSON.stringify(["LOG_LEVEL", {"source": "value"}])} className={"mt-1.5 w-full bg-slate-900 border border-slate-700/90 rounded-lg px-2 py-1.5 text-xs font-mono font-bold text-white focus:outline-none focus:border-blue-500"}>
                      {"\n                    "}
                      <option value={"0"}>
                        {"0 (Выкл)"}
                      </option>
                      {"\n                    "}
                      <option value={"1"}>
                        {"1 (Отладка)"}
                      </option>
                      {"\n                  "}
                    </select>
                    {"\n                "}
                  </div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n\n            "}
              {"\n            "}
              <div className={"ui-card rounded-2xl p-4 shadow-lg space-y-3 xl:col-span-2"}>
                <div className={"flex items-center justify-between gap-3 border-b border-slate-800 pb-2.5"}>
                  <div className={"flex items-center gap-2"}>
                    <span className={"h-4 w-2 rounded-full bg-cyan-400"}></span>
                    <h3 className={"text-sm font-bold text-white"}>{"Параметры nfqws2 по группам"}</h3>
                  </div>
                  <span className={"text-[11px] text-slate-400"}>{"Измените нужную группу; сохраните кнопкой сверху"}</span>
                </div>
                <p className={"text-xs leading-relaxed text-slate-400"}>{"Каждое поле редактирует отдельную переменную. Изменения попадают в черновик полной конфигурации; для записи и перезапуска нажмите «Применить конфигурацию»."}</p>
                <div id={"nfqwsArgEditors"} className={"grid grid-cols-1 gap-3 lg:grid-cols-2"}>
                  <div className={"text-xs text-slate-500"}>{"Загрузка аргументов из конфигурации…"}</div>
                </div>
              </div>
              {"\n            "}
              {"\n\n          "}
            </div>
            <section id={"configScriptsPanel"} className={"config-section-panel hidden xl:col-span-12 ui-card rounded-2xl border border-slate-800 p-4 shadow-xl"} aria-label={"Lua-скрипты nfqws2"}>
              <div className={"mb-3 flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 pb-3"}>
                <div>
                  <h3 className={"text-sm font-bold text-white"}>{"Скрипты nfqws2"}</h3>
                  <p className={"mt-1 text-xs text-slate-400"}>{"Файлы читаются из каталога Lua обнаруженной установки. Просмотр доступен только для чтения."}</p>
                </div>
                <button type={"button"} data-action-click={"refreshLuaScripts"} className={"rounded-lg border border-slate-700 bg-slate-800 px-3 py-1.5 text-xs font-medium text-slate-200 hover:bg-slate-700"}>{"Обновить список"}</button>
              </div>
              <nav id={"luaScriptTabs"} aria-label={"Lua-скрипты"} className={"mb-3 flex flex-wrap gap-1 border-b border-slate-800"}>
                <span className={"px-3 py-2 text-xs text-slate-500"}>{"Список загружается…"}</span>
              </nav>
              <div id={"luaScriptViewer"} className={"overflow-hidden rounded-xl border border-slate-800 bg-[#070b14]"}>
                <div id={"luaScriptPath"} className={"border-b border-slate-800 px-3 py-2 font-mono text-xs text-slate-400"}>{"Путь к файлу появится после загрузки"}</div>
                <pre id={"luaScriptContent"} className={"max-h-[680px] min-h-36 overflow-auto p-4 font-mono text-xs leading-5 text-slate-200 whitespace-pre"}>{"Выберите скрипт для просмотра."}</pre>
              </div>
            </section>
            {"\n\n        "}
          </div>
          {"\n      "}
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-lists"} className={"hidden view-animate space-y-5"}>
          {"\n        \n        "}
          <div id={"listEditorPanel"} className={"ui-card rounded-2xl overflow-hidden shadow-xl"}>
            {"\n          "}
            {"\n          "}
            <div id={"listFileTabs"} className={"flex items-center overflow-x-auto bg-[#080d18] border-b border-slate-800 px-2"}>
              {"\n            \n            "}
              <button data-action-click={"selectListFile"} data-args={JSON.stringify(["user.list"])} data-action-tooltip={"Домены, добавленные вручную для обхода."} data-action-tooltip-delay-ms={"3000"} aria-label={"user.list — Пользовательские домены"} id={"listTab-user.list"} className={"list-file-tab active flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-bold text-white border-b-2 border-sky-400 bg-slate-800/40 transition-all whitespace-nowrap"}>
                {"\n              "}
                <svg className={"w-4 h-4 text-slate-200"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                </svg>
                {"\n              "}
                <span className={"list-file-tab-caption"}><code>user.list</code></span>
                {"\n            "}
              </button>
              {"\n\n            "}
              <button data-action-click={"selectListFile"} data-args={JSON.stringify(["exclude.list"])} data-action-tooltip={"Домены, которые исключены из обхода."} data-action-tooltip-delay-ms={"3000"} aria-label={"exclude.list — Исключённые домены"} id={"listTab-exclude.list"} className={"list-file-tab flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-medium text-slate-400 hover:text-slate-200 border-b-2 border-transparent transition-all whitespace-nowrap"}>
                {"\n              "}
                <svg className={"w-4 h-4 text-slate-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                </svg>
                {"\n              "}
                <span className={"list-file-tab-caption"}><code>exclude.list</code></span>
                {"\n            "}
              </button>
              {"\n\n            "}
              <button data-action-click={"selectListFile"} data-args={JSON.stringify(["auto.list"])} data-action-tooltip={"Домены, которые nfqws2 добавил автоматически."} data-action-tooltip-delay-ms={"3000"} aria-label={"auto.list — Домены для автообучения"} id={"listTab-auto.list"} className={"list-file-tab flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-medium text-slate-400 hover:text-slate-200 border-b-2 border-transparent transition-all whitespace-nowrap"}>
                {"\n              "}
                <svg className={"w-4 h-4 text-slate-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                </svg>
                {"\n              "}
                <span className={"list-file-tab-caption"}><code>auto.list</code></span>
                {"\n            "}
              </button>
              {"\n\n            "}
              <button data-action-click={"selectListFile"} data-args={JSON.stringify(["ipset.list"])} data-action-tooltip={"IP-адреса и подсети, к которым применяются правила."} data-action-tooltip-delay-ms={"3000"} aria-label={"ipset.list — IP-адреса"} id={"listTab-ipset.list"} className={"list-file-tab flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-medium text-slate-400 hover:text-slate-200 border-b-2 border-transparent transition-all whitespace-nowrap"}>
                {"\n              "}
                <svg className={"w-4 h-4 text-slate-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                </svg>
                {"\n              "}
                <span className={"list-file-tab-caption"}><code>ipset.list</code></span>
                {"\n            "}
              </button>
              {"\n\n            "}
              <button data-action-click={"selectListFile"} data-args={JSON.stringify(["ipset_exclude.list"])} data-action-tooltip={"IP-адреса и подсети, исключённые из правил."} data-action-tooltip-delay-ms={"3000"} aria-label={"ipset_exclude.list — Исключённые IP-адреса"} id={"listTab-ipset_exclude.list"} className={"list-file-tab flex items-center gap-2.5 px-5 py-3.5 text-sm font-mono font-medium text-slate-400 hover:text-slate-200 border-b-2 border-transparent transition-all whitespace-nowrap"}>
                {"\n              "}
                <svg className={"w-4 h-4 text-slate-400"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                  <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"}></path>
                </svg>
                {"\n              "}
                <span className={"list-file-tab-caption"}><code>ipset_exclude.list</code></span>
                {"\n            "}
              </button>
              <div id={"customListTabs"} className={"flex items-center"}></div>
              {"\n\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"p-4 bg-[#0b111e] border-b border-slate-800 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-4"}>
              {"\n            "}
              <div className={"flex flex-wrap items-center gap-3"}>
                {"\n              "}
                <span id={"currentListFilePath"} className={"font-mono text-xs px-3 py-1 rounded-lg bg-slate-950 text-sky-400 border border-slate-800"}>
                  {"Список не выбран"}
                </span>
                <span id={"listAutoUpdateStatus"} className={"hidden text-xs font-medium text-slate-400"} aria-live={"polite"}></span>
                {"\n              "}
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"list-editor-toolbar-actions flex w-full flex-wrap items-center gap-2 lg:w-auto"}>
                <select id={"listEditorViewSelect"} aria-label={"Вид списка"} data-action-change={"setListEditorView"} data-value-source={"value"} data-menu-min-width={"150"} className={"list-editor-view-select min-h-9 min-w-36 rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm font-semibold text-slate-100"}>
                  <option value={"text"}>Текстом</option>
                  <option value={"rows"}>Списком</option>
                </select>
                {"\n              "}
                <button data-action-click={"saveActiveListFile"} data-action-tooltip={"Проверить и сохранить текущий список"} className={"list-toolbar-button list-toolbar-button-primary"}>
                  {"Применить список"}
                </button>
                <details className={"list-toolbar-menu"}>
                  <summary aria-label={"Дополнительные действия со списком"} title={"Дополнительные действия"}>⋯</summary>
                  <div className={"list-toolbar-menu-items"}>
                    <button data-action-click={"removeSelectedListLines"} data-action-tooltip={"Удалить выбранные строки или текущую строку курсора"} className={"list-toolbar-button list-toolbar-button-danger"}>{"Удалить строку/выделение"}</button>
                    <button data-action-click={"deduplicateCurrentList"} data-action-tooltip={"Удалить повторяющиеся записи из черновика"} className={"list-toolbar-button list-toolbar-button-secondary"}>{"Удалить дубли"}</button>
                    <button data-action-click={"openListImport"} data-action-tooltip={"Добавить записи из файла .list или .txt"} className={"list-toolbar-button list-toolbar-button-secondary"}>{"Импорт"}</button>
                    <button data-action-click={"exportListDraft"} data-action-tooltip={"Скачать текущий черновик списка"} className={"list-toolbar-button list-toolbar-button-secondary"}>{"Экспорт"}</button>
                    <input id={"listImportFile"} type={"file"} accept={".list,.txt,text/plain"} className={"hidden"} data-action-change={"importListDraft"} />
                  </div>
                </details>
                <button type={"button"} data-action-click={"requestClearListDraft"} data-action-tooltip={"Очистить содержимое текущего черновика"} aria-label={"Очистить черновик списка"} className={"config-editor-icon-button list-editor-icon-button"}>
                  <svg aria-hidden={"true"} viewBox={"0 0 24 24"} fill={"none"} stroke={"currentColor"} strokeWidth={"1.8"} strokeLinecap={"round"} strokeLinejoin={"round"}><path d={"M3 6h18M8 6V4h8v2m-9 0 1 14h8l1-14M10 10v6m4-6v6"}></path></svg>
                </button>
                <button id={"listEditorFullscreenToggle"} type={"button"} data-action-click={"toggleListEditorFullscreen"} data-action-tooltip={"Развернуть редактор списка"} aria-label={"Развернуть редактор списка"} aria-pressed={"false"} className={"config-editor-icon-button list-editor-icon-button"}>
                  <svg aria-hidden={"true"} viewBox={"0 0 24 24"} fill={"none"} stroke={"currentColor"} strokeWidth={"1.8"} strokeLinecap={"round"} strokeLinejoin={"round"}><path d={"M8 3H5a2 2 0 0 0-2 2v3m13-5h3a2 2 0 0 1 2 2v3M3 16v3a2 2 0 0 0 2 2h3m13-5v3a2 2 0 0 1-2 2h-3"}></path></svg>
                </button>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div id={"listTextView"} className={"mobile-list-editor flex h-[610px] bg-[#060a12] font-mono text-[13.5px] leading-6 overflow-hidden"}>
              {"\n            "}
              <div id={"listLineNumbers"} className={"line-numbers-col bg-[#05080f] border-r border-slate-800/80 py-4 px-3.5 w-14 overflow-hidden select-none text-xs leading-6"}>
                {"\n              1\n            "}
              </div>
              {"\n            "}
              <textarea id={"listEditorTextarea"} data-action-input={"syncListLineNumbers"} data-action-scroll={"syncListScroll"} data-args={JSON.stringify([{"source": "self"}])} spellCheck={"false"} className={"flex-1 bg-transparent text-slate-100 py-4 px-6 outline-none resize-none leading-6 font-mono overflow-y-auto border-none"}></textarea>
              {"\n          "}
            </div>
            <div id={"listRowsView"} className={"hidden list-rows-view h-[610px] overflow-y-auto bg-[#060a12] p-3"} role={"list"} aria-label={"Записи списка"}></div>
            {"\n\n          "}
            <div className={"list-editor-footer px-5 py-2.5 bg-[#080d18] border-t border-slate-800 flex items-center justify-between text-xs font-mono text-slate-400"}>
              {"\n            "}
              <span id={"listFooterCount"}>
                {"Всего записей: —"}
              </span>
              {"\n            "}
              {"\n          "}
            </div>
            {"\n        "}
          </div>
          {"\n\n      "}
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-check"} className={"hidden view-animate space-y-4"}>
          <div className={"ui-card rounded-2xl p-5 sm:p-6"}>
            <div className={"flex flex-wrap items-center justify-between gap-2 mb-4"}>
              <h2 className={"text-lg font-bold text-white"}>Диагноз блокировки</h2>
              <span className={"text-[10px] tracking-wider font-mono text-blue-300 bg-blue-500/10 border border-blue-500/20 rounded-lg px-2.5 py-1"}>DNS · HTTPS · QUIC</span>
            </div>
            <div className={"flex flex-col sm:flex-row gap-2.5"}>
              <input id={"diagDomainInput"} type={"text"} aria-label={"Домен или публичный IPv4 для проверки"} autoCapitalize={"none"} spellCheck={false} placeholder={"example.com или 8.8.8.8"} className={"min-w-0 flex-1 bg-[#060a12] border border-slate-700 rounded-xl px-4 py-3 text-sm font-mono text-white focus:outline-none focus:border-blue-500 focus:ring-2 focus:ring-blue-500/10 transition-colors"} />
              <button data-action-click={"runSiteDiagnosis"} className={"min-w-[140px] px-5 py-3 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-sm transition-colors shadow-lg shadow-blue-600/15"}>Проверить</button>
            </div>
            <div className={"flex flex-wrap items-center gap-2 mt-3 text-[11px] font-mono"}>
              <button data-action-click={"setDiagPreset"} data-args={JSON.stringify(["example.com"])} className={"px-2.5 py-1 rounded-full bg-slate-800/50 hover:bg-slate-800 text-slate-400 hover:text-blue-300 transition-colors"}>example.com</button>
              <button data-action-click={"setDiagPreset"} data-args={JSON.stringify(["youtube.com"])} className={"px-2.5 py-1 rounded-full bg-slate-800/50 hover:bg-slate-800 text-slate-400 hover:text-blue-300 transition-colors"}>youtube.com</button>
              <button data-action-click={"setDiagPreset"} data-args={JSON.stringify(["openai.com"])} className={"px-2.5 py-1 rounded-full bg-slate-800/50 hover:bg-slate-800 text-slate-400 hover:text-blue-300 transition-colors"}>openai.com</button>
            </div>
            <p className={"diagnosis-help mt-4 text-xs leading-relaxed"}>Для домена сравниваются DNS, прямой доступ, текущий маршрут и QUIC. Для IP DNS и QUIC пропускаются. nfqws2 продолжит работать.</p>
          </div>
          <div id={"diagResultContainer"} className={"diagnosis-results"}></div>
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-trace"} className={"hidden view-animate space-y-5"}>
          <div className={"ui-card rounded-2xl p-6 shadow-xl"}>
            <div className={"pb-5 border-b border-slate-800"}>
              <span className={"px-2.5 py-0.5 rounded-md bg-blue-500/15 text-blue-300 border border-blue-500/30 text-xs font-mono font-bold"}>Диагностика nfqws2 --debug</span>
              <h2 className={"mt-2 text-lg font-bold text-white"}>Трассировка домена</h2>
              <p className={"mt-1 text-xs sm:text-sm text-slate-400"}>Сравнивает HTTP-ответ сайта с nfqws2 и без него. Запрос выполняется на роутере; основной процесс и конфигурация не должны изменяться.</p>
            </div>
            <div className={"mt-5 flex flex-col sm:flex-row gap-3"}>
              <div className={"trace-protocol-picker"}>
                <select id={"traceProtoSelect"} aria-label={"Протокол трассировки"} className={"trace-protocol-select"}>
                  <option value={"https"}>HTTPS</option>
                  <option value={"http"}>HTTP</option>
                </select>
                <svg className={"trace-protocol-chevron"} aria-hidden={"true"} viewBox={"0 0 20 20"} fill={"none"}><path d={"m5 7.5 5 5 5-5"} stroke={"currentColor"} strokeWidth={"1.8"} strokeLinecap={"round"} strokeLinejoin={"round"}></path></svg>
              </div>
              <input id={"traceDomainInput"} type={"text"} placeholder={"Введите домен, например example.com"} className={"flex-1 bg-[#060a12] border border-slate-700 rounded-xl px-4 py-2.5 text-sm font-mono text-white focus:outline-none focus:border-blue-500"} />
              <button data-action-click={"runSiteTrace"} className={"px-6 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-bold text-sm transition-all shadow-lg shadow-blue-600/20"}>Запустить трассировку</button>
              <button id={"traceStopButton"} data-action-click={"stopSiteTrace"} className={"hidden px-5 py-2.5 rounded-xl bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/30 text-rose-300 font-bold text-sm transition-all"}>Остановить</button>
            </div>
            <p id={"traceOperationStatus"} className={"hidden mt-3 text-xs text-blue-300"} role={"status"} aria-live={"polite"}></p>
              <p className={"trace-help mt-3 text-xs"}>Запускается отдельная проверка с копией конфига и подробным журналом. QUIC проверяется в разделе «Проверка сайта».</p>
          </div>
          <div id={"traceResultContainer"} className={"grid grid-cols-1 lg:grid-cols-2 gap-5"}></div>
        </section>
        <section id={"tabContent-autotune"} className={"hidden view-animate space-y-3"}>
          {"\n        \n        "}
          <div className={"ui-card autotune-settings-card rounded-xl p-3 sm:p-4 shadow-lg"}>
            {"\n          "}
            <div className={"flex flex-col lg:flex-row lg:items-center justify-between gap-2 pb-3 border-b border-slate-800"}>
              {"\n            "}
              <div>
                {"\n              "}
                <div className={"flex flex-wrap items-center gap-2"}>
                  {"\n                "}
                  <span className={"px-2 py-0.5 rounded-md bg-amber-500/15 text-amber-300 border border-amber-500/30 text-[11px] font-mono font-bold"}>
                    {"Автоподбор стратегий"}
                  </span>
                  {"\n                "}
                  <h2 className={"text-base font-bold text-white"}>
                    {"Авто-подбор стратегии nfqws2"}
                  </h2>
                  {"\n              "}
                </div>
                {"\n              "}
                <p className={"text-xs text-slate-300 mt-1"}>
                    {"Проверяет реальные запросы отдельным экземпляром nfqws2 и временными правилами только для тестовых соединений. Основной конфиг автоматически не меняется."}
                </p>
                {"\n            "}
              </div>
              {"\n\n            "}
              <div className={"flex flex-wrap items-center gap-3"}>
                {"\n              "}
                <button id={"startScanBtn"} data-action-click={"startStrategyPick"} className={"whitespace-nowrap px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-500 text-white font-bold text-sm flex items-center gap-2 shadow-md shadow-blue-600/20 transition-all active:scale-95"}>
                  {"\n                "}
                  <svg className={"w-4 h-4"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
                    <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2.2"} d={"M13 10V3L4 14h7v7l9-11h-7z"}></path>
                  </svg>
                  {"\n                "}
                  <span id={"scanBtnText"}>Подобрать стратегию</span>
                  {"\n              "}
                </button>
                <button id={"strategyPickCancel"} data-action-click={"cancelStrategyPick"} className={"hidden px-3 py-2 rounded-lg border border-rose-500/30 bg-rose-500/10 text-rose-200 text-sm font-semibold"}>Остановить</button>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n\n          "}
            {"\n          "}
            <div className={"grid grid-cols-1 gap-1 mt-2 strategy-settings-grid"}>
              {"\n            "}
              <div className={"strategy-target-mode-switch"} role={"group"} aria-label={"Область автоподбора"}>
                <button type={"button"} data-action-click={"setStrategyTargetMode"} data-arg={"single"} className={"active"}>Один сайт</button>
                <button type={"button"} data-action-click={"setStrategyTargetMode"} data-arg={"multi"}>От 2 до 15 сайтов</button>
              </div>
              <div className={"strategy-target-transport-row"}>
                <label htmlFor={"scanCustomHost"} className={"text-xs font-mono text-slate-400 strategy-target-single-input"}>
                  {"Домен или публичный IPv4"}
                  <input id={"scanCustomHost"} type={"text"} placeholder={"example.com или 8.8.8.8"} autoCapitalize={"none"} spellCheck={false} className={"mt-1.5 w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-1.5 text-xs sm:text-sm font-mono text-blue-300 focus:outline-none focus:border-blue-500"} />
                </label>
                <label htmlFor={"strategyProtocolSelect"} className={"text-xs font-mono text-slate-400"}>
                  {"Транспорт"}
                  <select id={"strategyProtocolSelect"} defaultValue={"https"} className={"mt-1.5 w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-1.5 text-xs sm:text-sm font-mono text-white focus:outline-none focus:border-blue-500"}>
                    <option value={"https"}>HTTPS · TCP 443</option><option value={"http"}>HTTP · TCP 80</option>
                  </select>
                </label>
                <p className={"strategy-target-transport-help"}>{"QUIC Initial проверяется в разделе «Проверка сайта»; перебор QUIC-стратегий пока не подключён."}</p>
              </div>

              <div id={"strategyMultiTargetPanel"} className={"hidden strategy-multi-target-panel"}>
                <div className={"strategy-multi-list-toolbar"}>
                  <label htmlFor={"strategyMultiListSelect"}>Список сайтов</label>
                  <select id={"strategyMultiListSelect"} defaultValue={"user.list"}>
                    <option value={"user.list"}>Пользовательские домены</option>
                  </select>
                  <button id={"pickRandomMultiSitesButton"} type={"button"} data-action-click={"pickRandomMultiSites"}>↻ Другие случайные</button>
                </div>
                <label htmlFor={"strategyMultiHostList"}>Домены для общего профиля</label>
                <textarea id={"strategyMultiHostList"} rows={5} spellCheck={false} autoCapitalize={"none"} placeholder={"youtube.com\ndiscord.com\nexample.org"}></textarea>
                <p>Укажите от 2 до 15 уникальных доменов. Сначала проверим их напрямую, затем будем искать один общий профиль.</p>
              </div>

              <div className={"strategy-setting-row"}>
                <label htmlFor={"strategyRepeatsSelect"} className={"block text-xs font-mono text-slate-400"}>Повторов</label>
                <select id={"strategyRepeatsSelect"} defaultValue={"3"} className={"mt-1.5 w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-xs sm:text-sm font-mono text-white focus:outline-none focus:border-blue-500"}>
                  <option value={"2"}>2 повтора</option><option value={"3"}>3 повтора</option><option value={"4"}>4 повтора</option><option value={"5"}>5 повторов</option>
                </select>
                <div className={"strategy-option-grid"}>
                  <label htmlFor={"strategyStopAfterFirst"} className={"flex cursor-pointer items-start gap-2 text-xs font-medium text-slate-300"}>
                    <input id={"strategyStopAfterFirst"} type={"checkbox"} className={"mt-0.5 accent-blue-500"} />
                    <span>Остановиться на первой подтверждённой стратегии</span>
                  </label>
                  <label htmlFor={"strategyRefineLighter"} className={"flex cursor-pointer items-start gap-2 text-xs font-medium text-slate-200"}>
                    <input id={"strategyRefineLighter"} type={"checkbox"} className={"mt-0.5 accent-blue-500"} />
                    <span>Искать более лёгкий вариант после подтверждения
                      <span className={"mt-1 block text-[11px] font-normal leading-4 text-slate-400"}>Проверит варианты fake-шага отдельно, до 5 минут.</span>
                    </span>
                  </label>
                  <label htmlFor={"strategyIncludeOtherHistory"} className={"flex cursor-pointer items-start gap-2 text-xs font-medium text-slate-200"}>
                    <input id={"strategyIncludeOtherHistory"} type={"checkbox"} defaultChecked={true} className={"mt-0.5 accent-blue-500"} />
                    <span>Стратегии из истории других сайтов</span>
                  </label>
                  <label htmlFor={"strategyIncludeCommunity"} className={"flex cursor-pointer items-start gap-2 text-xs font-medium text-slate-200"}>
                    <input id={"strategyIncludeCommunity"} type={"checkbox"} className={"mt-0.5 accent-teal-500"} />
                    <span>Добавить проверенные стратегии сообщества</span>
                  </label>
                  <p className={"strategy-option-help"}>Подходящие варианты будут загружены из каталога и проверены отдельными запросами.</p>
                </div>
              </div>
              {"\n\n            "}
              {"\n\n            "}
              <div className={"strategy-setting-row strategy-mode-tile strategy-mode-specific"}>
                {"\n              "}
                <label className={"block text-xs font-mono text-slate-400"}>
                  {"Режим тестирования"}
                </label>
                {"\n              "}
                <select id={"strategyModeSelect"} data-action-change={"toggleStrategyCustomInput"} data-value-source={"value"} defaultValue={"quick"} className={"mt-1.5 w-full bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-xs sm:text-sm font-mono text-white focus:outline-none focus:border-blue-500"}>
                  <option value={"quick"}>Быстрый · текущие + базовые</option><option value={"extended"}>Расширенный · каталог blockcheck2</option><option value={"full"}>Полный · каталог blockcheck2 (~3 000)</option><option value={"custom"}>Своя стратегия</option>
                </select>
                <p id={"strategyModeHelp"} aria-live={"polite"} className={"strategy-mode-help mt-2 text-xs font-medium leading-5 text-slate-300"}>Проверяются текущие стратегии и небольшой базовый набор.</p>
                <div id={"strategyCustomInputPanel"} className={"hidden mt-2 border-t border-slate-800 pt-2"}>
                  <details id={"strategyCustomDetailsDisclosure"} open className={"group"}>
                    <summary className={"flex cursor-pointer list-none items-center gap-2 text-xs font-mono text-slate-400 marker:hidden focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"}>
                      <span aria-hidden={"true"} className={"text-blue-400 transition-transform group-open:rotate-90"}>›</span>
                      <span>Свои параметры · для режима «Своя стратегия»</span>
                    </summary>
                    <div className={"pt-2"}>
                      <textarea id={"strategyCustomSteps"} rows={3} spellCheck={false} aria-label={"Параметры своей стратегии"} placeholder={"одна стратегия на строку, например:\n--lua-desync=multisplit:pos=1,midsld\n--lua-desync=fakedsplit:pos=1"} className={"w-full resize-y bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-xs font-mono leading-5 text-white placeholder:text-slate-500 focus:outline-none focus:border-blue-500"}></textarea>
                      <p className={"mt-1 text-xs leading-5 text-slate-400"}>Каждая строка — отдельный вариант. Несколько --lua-desync в одной строке — шаги одного варианта; \ в конце строки продолжает его на следующей.</p>
                      <p className={"mt-1 text-xs leading-5 text-slate-400"}>Разрешены только параметры --lua-desync.</p>
                      <button type={"button"} data-action-click={"copyStrategyParams"} data-value-source={"self"} className={"strategy-copy-button mt-2 rounded-md border border-slate-700 px-2.5 py-1 text-xs font-semibold text-slate-300 transition-colors hover:border-blue-500 hover:text-blue-200"}>Копировать</button>
                    </div>
                  </details>
                </div>
                {"\n            "}
              </div>
              <div className={"strategy-progress-card"}>
                {"\n              "}
                <div className={"flex items-center justify-between text-xs font-mono"}>
                  {"\n                "}
                  <span className={"text-slate-400"}>
                    {"Статус автоподбора:"}
                  </span>
                  {"\n                "}
                  <span id={"scanProgressLabel"} className={"text-blue-400 font-bold"}>
                    {"Готов к проверке"}
                  </span>
                  {"\n              "}
                </div>
                {"\n              "}
                <div className={"w-full bg-slate-800 h-2.5 rounded-full overflow-hidden mt-2"}>
                  {"\n                "}
                  <div id={"scanProgressBar"} className={"bg-blue-500 h-full rounded-full transition-all duration-300"} style={{ width: "0%" }}></div>
                  {"\n              "}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n        "}
          </div>
          {"\n\n        "}
          {"\n        "}
          <div className={"grid grid-cols-1 2xl:grid-cols-12 gap-3"}>
            {"\n          "}
            <div className={"2xl:col-span-8 space-y-3"} id={"autotuneStrategiesList"}></div>
            <section id={"autotunePreviewPanel"} className={"hidden 2xl:col-span-8 ui-card rounded-2xl p-4 space-y-3"} aria-live={"polite"}>
              <div>
                <h3 className={"text-sm font-bold text-white"}>Предпросмотр применения</h3>
                <p id={"autotunePreviewSummary"} className={"mt-1 text-sm font-medium text-slate-300"}></p>
              </div>
              <div id={"autotunePreviewDiff"} className={"max-h-96 overflow-auto rounded-xl bg-[#060a12] p-3 font-mono text-xs"}></div>
              <p className={"text-xs font-medium text-slate-300"}>Профиль ограничен проверенным доменом. Проверьте diff перед применением; будет создана резервная копия, затем nfqws2 перезапустится.</p>
              <button id={"applyStrategyPickButton"} data-action-click={"applyStrategyPick"} disabled className={"w-fit rounded-xl bg-blue-600 px-4 py-2.5 text-sm font-bold text-white transition hover:bg-blue-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300 disabled:cursor-wait disabled:opacity-60"}>
                {"Применить стратегию и перезапустить"}
              </button>
            </section>
            {"\n\n          "}
            <div className={"2xl:col-span-4 ui-card rounded-xl p-4 flex flex-col min-h-[180px] 2xl:h-[360px]"}>
              {"\n            "}
              <div className={"pb-3 border-b border-slate-800 flex items-center justify-between"}>
                {"\n              "}
                <span className={"text-sm font-bold text-white font-mono"}>
                  {"История последних подборов"}
                </span>
                <div className={"flex items-center gap-2"}>
                  <span className={"text-xs font-mono text-amber-400"}>до 12 задач</span>
                  <button id={"clearStrategyPickHistoryButton"} type={"button"} data-action-click={"requestClearStrategyPickHistory"} disabled title={"Очистить завершённые подборы"} aria-label={"Очистить историю подборов"} className={"rounded-md border border-slate-700 px-2 py-1 text-[11px] font-semibold text-slate-300 hover:border-rose-500 hover:text-rose-200 disabled:cursor-not-allowed disabled:opacity-40"}>Очистить</button>
                </div>
                {"\n            "}
              </div>
              {"\n            "}
              <div id={"autotuneConsoleBox"} className={"flex-1 overflow-y-auto font-mono text-xs space-y-2 mt-3 pr-1 text-slate-300 leading-relaxed"}>
                {"\n              "}
                <div className={"text-slate-500"}>
                  {"История появится после завершённых проверок. Выберите запись, чтобы открыть её результаты."}
                </div>
                {"\n            "}
              </div>
              {"\n          "}
            </div>
            {"\n        "}
          </div>
          {"\n\n      "}
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}
        <section id={"tabContent-diagnostic16k"} className={"hidden view-animate space-y-3"}>
          <div className={"ui-card rounded-2xl p-3 sm:p-3.5 space-y-2"}>
            <div>
              <h2 className={"text-base font-bold text-white"}>Обрыв на 16 КБ</h2>
              <p className={"mt-0.5 text-xs leading-relaxed text-slate-300"}>Сравниваем передачу напрямую и через nfqws2. Результаты и пояснения открываются по запросу.</p>
            </div>
            <div className={"flex flex-col sm:flex-row sm:items-end gap-2"}>
              <label htmlFor={"freezeScanHostsInput"} className={"min-w-0 flex-1 space-y-1 text-xs font-semibold text-slate-200"}>
                <span>Свои сайты <span className={"font-medium text-slate-400"}>(необязательно)</span></span>
                <input id={"freezeScanHostsInput"} type={"text"} placeholder={"example.com, video.example"} autoCapitalize={"none"} spellCheck={false} className={"w-full bg-[#060a12] border border-slate-700 rounded-lg px-3 py-1.5 text-sm font-mono text-white placeholder:text-slate-500"} />
              </label>
              <button data-action-click={"runFreezeScan"} className={"px-3.5 py-1.5 rounded-lg bg-blue-600 text-sm text-white font-semibold"}>Запустить пробу</button>
              <button id={"freezeScanCancel"} data-action-click={"cancelFreezeScan"} className={"hidden px-3 py-1.5 rounded-lg border border-rose-500/30 text-sm text-rose-200"}>Остановить</button>
            </div>
            <label className={"flex items-start gap-2 text-xs text-slate-300"}><input id={"freezeScanUploadProbe"} type={"checkbox"} className={"mt-0.5 accent-blue-500"} /><span><strong>Проверить короткие POST-ответы (до 64 КБ)</strong><small className={"mt-0.5 block text-slate-400"}>Только для введённых доменов: сервер получит тестовое тело. Отказ POST сам по себе не означает обрыв передачи.</small></span></label>
            <p id={"freezeScanStatus"} className={"hidden text-sm font-semibold text-blue-200"} role={"status"} aria-live={"polite"}></p>
            <div id={"freezeScanProgress"} className={"hidden rounded-xl border border-blue-400/25 bg-blue-500/5 p-4"} role={"status"} aria-live={"polite"}>
              <div className={"flex items-center justify-between gap-3"}>
                <strong id={"freezeScanProgressTitle"} className={"text-sm font-bold text-blue-100"}></strong>
                <span id={"freezeScanProgressPercent"} className={"shrink-0 text-xs font-bold tabular-nums text-blue-200"}></span>
              </div>
              <div className={"mt-2 h-1.5 overflow-hidden rounded-full bg-slate-800"}>
                <div id={"freezeScanProgressBar"} className={"h-full rounded-full bg-blue-400 transition-[width] duration-300"}></div>
              </div>
              <p id={"freezeScanProgressDetail"} className={"mt-2 text-xs leading-relaxed text-slate-300"}></p>
              <ol id={"freezeScanProgressSteps"} className={"mt-3 grid gap-2 sm:grid-cols-3"}></ol>
            </div>
            <p id={"freezeScanSummary"} className={"hidden rounded-xl border px-3 py-2 text-sm font-bold leading-snug"} role={"status"} aria-live={"polite"}></p>
            <div id={"freezeScanResults"} className={"hidden overflow-x-auto rounded-xl border border-slate-800"}></div>
            <section id={"freezeApplyPreviewPanel"} className={"hidden rounded-2xl border border-blue-400/30 bg-slate-900/70 p-4 space-y-3"} aria-live={"polite"}>
              <div>
                <h3 className={"text-sm font-bold text-white"}>Применение к проверенному сайту</h3>
                <p id={"freezeApplyPreviewSummary"} className={"mt-1 text-sm font-medium text-slate-200"}></p>
              </div>
              <div id={"freezeApplyPreviewDiff"} className={"max-h-72 overflow-auto rounded-xl bg-[#060a12] p-3 font-mono text-xs"}></div>
              <p className={"text-xs font-medium text-slate-300"}>Профиль ограничен этим доменом. Перед применением проверьте diff; будет создана резервная копия, затем nfqws2 перезапустится.</p>
              <button id={"applyFreezeStrategyButton"} data-action-click={"applyStrategyPick"} disabled className={"w-fit rounded-xl bg-blue-600 px-4 py-2.5 text-sm font-bold text-white transition hover:bg-blue-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-300 disabled:cursor-wait disabled:opacity-60"}>Применить и перезапустить</button>
            </section>
            <section id={"freezeJoinPreviewPanel"} className={"hidden rounded-2xl border border-emerald-400/30 bg-slate-900/70 p-4 space-y-3"} aria-live={"polite"}>
              <div><h3 className={"text-sm font-bold text-white"}>Добавление ASN-сети в профиль</h3><p id={"freezeJoinPreviewSummary"} className={"mt-1 text-sm font-medium text-slate-200"}></p></div>
              <div id={"freezeJoinPreviewDiff"} className={"max-h-72 overflow-auto rounded-xl bg-[#060a12] p-3 font-mono text-xs"}></div>
              <p className={"text-xs font-medium text-slate-300"}>Будет создан новый статический IP-список на основе RIPEstat и добавлена ссылка в выбранный профиль. Перед записью конфигурация проверяется и сохраняется резервная копия.</p>
              <div className={"flex flex-wrap gap-2"}><button id={"applyFreezeJoinButton"} data-action-click={"applyFreezeJoin"} disabled className={"w-fit rounded-xl bg-emerald-700 px-4 py-2.5 text-sm font-bold text-white transition hover:bg-emerald-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-emerald-300 disabled:cursor-wait disabled:opacity-60"}>Добавить и перезапустить</button><button data-action-click={"closeFreezeJoinPreview"} className={"rounded-xl border border-slate-700 px-4 py-2.5 text-sm font-semibold text-slate-200 hover:border-slate-500"}>Закрыть</button></div>
            </section>
            <p className={"text-xs leading-relaxed text-slate-400"}>Проверки идут по очереди. Возможный сетевой обрыв не доказывает блокировку DPI; служба nfqws2 и домашний трафик не меняются.</p>
          </div>
        </section>
        <section id={"tabContent-configCheck"} className={"hidden view-animate space-y-4"}>
          <div className={"ui-card rounded-2xl p-4 sm:p-5 space-y-4"}>
            <div>
              <h2 className={"text-lg font-bold text-white"}>Проверка сторонней конфигурации</h2>
              <p className={"mt-1 max-w-none text-xs leading-5 text-slate-400"}>Сравните текущий конфиг и до 10 вариантов на одних сайтах. Перед тестом nfqws2 --dry-run проверяет TCP, UDP, QUIC, IPSET и пользовательские профили. Каждый вариант отдельно проверяется по HTTPS/TCP выбранное число раз подряд; после первой ошибки повторы прекращаются. Текущий конфиг и обычный трафик не меняются. Пути Lua, блобов и списков берутся из установки, автосписок изолирован, debug-запись отключена. Проверка идёт последовательно и может занять время.</p>
            </div>
            <div className={"flex flex-wrap items-center gap-2"}>
              <button data-action-click={"openForeignConfigText"} className={"min-w-28 rounded-lg border border-slate-700 px-3 py-2 text-sm font-semibold text-slate-200 hover:border-blue-400"}>Текстом</button>
              <button data-action-click={"openForeignConfigUrl"} className={"min-w-28 rounded-lg border border-slate-700 px-3 py-2 text-sm font-semibold text-slate-200 hover:border-blue-400"}>По ссылке</button>
              <button data-action-click={"openForeignConfigFile"} className={"min-w-28 rounded-lg border border-slate-700 px-3 py-2 text-sm font-semibold text-slate-200 hover:border-blue-400"}>Файл .conf/.txt</button>
              <input id={"foreignConfigFileInput"} type={"file"} accept={".conf,.txt,text/plain"} className={"hidden"} />
            </div>
            <div id={"foreignConfigTextForm"} className={"hidden space-y-2 rounded-xl border border-slate-800 bg-slate-950/40 p-3"}>
              <input id={"foreignConfigTextName"} type={"text"} placeholder={"Название конфига (необязательно)"} className={"w-full rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2 text-sm text-white placeholder:text-slate-500"} />
              <textarea id={"foreignConfigTextContent"} rows={7} placeholder={"Вставьте содержимое nfqws2.conf целиком"} className={"w-full resize-y rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2 font-mono text-xs leading-relaxed text-white placeholder:text-slate-500"}></textarea>
              <div className={"flex justify-end gap-2"}><button data-action-click={"closeForeignConfigForms"} className={"rounded-lg px-3 py-2 text-sm text-slate-400"}>Закрыть</button><button data-action-click={"addForeignConfigText"} className={"rounded-lg bg-blue-600 px-3 py-2 text-sm font-bold text-white"}>Добавить конфиг</button></div>
            </div>
            <div id={"foreignConfigUrlForm"} className={"hidden space-y-2 rounded-xl border border-slate-800 bg-slate-950/40 p-3"}>
              <input id={"foreignConfigUrlName"} type={"text"} placeholder={"Название конфига (необязательно)"} className={"w-full rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2 text-sm text-white placeholder:text-slate-500"} />
              <input id={"foreignConfigUrl"} type={"url"} placeholder={"https://github.com/.../blob/main/nfqws2.conf или raw URL"} className={"w-full rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2 font-mono text-xs text-white placeholder:text-slate-500"} />
              <div className={"flex justify-end gap-2"}><button data-action-click={"closeForeignConfigForms"} className={"rounded-lg px-3 py-2 text-sm text-slate-400"}>Закрыть</button><button data-action-click={"addForeignConfigUrl"} className={"rounded-lg bg-blue-600 px-3 py-2 text-sm font-bold text-white"}>Добавить ссылку</button></div>
            </div>
            <div id={"foreignConfigList"} className={"space-y-2"}><p className={"text-sm text-slate-500"}>Добавьте конфиг текстом, ссылкой или файлом .conf/.txt.</p></div>
            <label htmlFor={"foreignConfigHosts"} className={"block max-w-3xl space-y-1.5 text-sm font-semibold text-slate-200"}>
              <span>Сайты для сравнения <span className={"font-normal text-slate-500"}>(1–15, домен или публичный IPv4)</span></span>
              <input id={"foreignConfigHosts"} type={"text"} defaultValue={"youtube.com, discord.com, googleusercontent.com"} className={"w-full rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2 font-mono text-xs text-white"} />
            </label>
            <div className={"flex flex-wrap items-center gap-3"}>
              <label htmlFor={"foreignConfigRepeats"} className={"flex items-center gap-2 text-sm text-slate-300"}>Повторов<select id={"foreignConfigRepeats"} defaultValue={"3"} className={"rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"}><option value={"1"}>1</option><option value={"2"}>2</option><option value={"3"}>3</option><option value={"4"}>4</option><option value={"5"}>5</option></select></label>
              <button id={"runForeignConfigCheck"} data-action-click={"runForeignConfigCheck"} className={"rounded-lg bg-blue-600 px-4 py-2.5 text-sm font-bold text-white disabled:opacity-50"}>▷ Проверить конфиги</button>
              <button id={"cancelForeignConfigCheck"} data-action-click={"cancelForeignConfigCheck"} className={"hidden rounded-lg border border-rose-500/30 px-4 py-2.5 text-sm font-semibold text-rose-200"}>Остановить</button>
              <p id={"foreignConfigStatus"} className={"hidden text-sm text-blue-200"} role={"status"} aria-live={"polite"}></p>
            </div>
            <div id={"foreignConfigProgress"} className={"hidden h-1.5 overflow-hidden rounded-full bg-slate-800"}><div id={"foreignConfigProgressBar"} className={"h-full bg-blue-500 transition-[width] duration-300"}></div></div>
            <div id={"foreignConfigResults"} className={"grid gap-3 lg:grid-cols-2"}></div>
            <p className={"text-[11px] leading-relaxed text-slate-500"}>Принимаются статические конфиги до 64 КиБ. Известные пути Lua/blob-файлов подставляются из текущей установки; незнакомые Lua-файлы и shell-команды не запускаются. Ссылка должна использовать HTTPS без редиректов. Проверки идут последовательно; при свободной памяти меньше 20 МиБ тест не запускается.</p>
          </div>
        </section>
        <section id={"tabContent-monitor"} className={"hidden view-animate space-y-5"}>
          <div className={"flex gap-2 border-b border-slate-800 pb-3"}><button data-action-click={"showMonitorSection"} data-args={JSON.stringify(["sites"])} className={"monitor-subtab active rounded-lg px-4 py-2 text-sm font-semibold"}>Мониторинг</button><button data-action-click={"showMonitorSection"} data-args={JSON.stringify(["telegram"])} className={"monitor-subtab rounded-lg px-4 py-2 text-sm font-semibold"}>Уведомления Telegram</button></div>
          <section id={"monitorSection-sites"} className={"ui-card rounded-2xl p-4 sm:p-5 space-y-3"}>
            <div className={"flex flex-wrap items-center justify-between gap-2"}><div><h2 className={"text-base font-bold text-white"}>Сайты</h2><p className={"mt-0.5 text-xs text-slate-400"}>Проверяются с роутера через основной nfqws2 · история до 96 циклов</p></div><div className={"monitor-settings-row flex flex-wrap items-center gap-1.5"}><label className={"flex items-center gap-1.5 text-xs text-slate-200"}><input id={"monitoringEnabled"} type={"checkbox"} defaultChecked className={"accent-blue-500"} /> Автопроверка</label><select id={"monitoringInterval"} data-action-change={"toggleMonitoringCustom"} data-args={JSON.stringify([{source:"self"}])} className={"rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-1.5 text-xs text-white"}><option value={"30"}>30 мин</option><option value={"60"}>1 час</option><option value={"120"}>2 часа</option><option value={"custom"}>Свой интервал</option></select><span id={"monitoringCustom"} className={"hidden items-center gap-1 text-xs text-slate-300"}><input id={"monitoringHours"} type={"number"} min={"0"} max={"24"} defaultValue={"0"} aria-label={"Часы интервала"} className={"w-14 rounded border border-slate-700 bg-slate-950 px-2 py-1"} /> ч <input id={"monitoringMinutes"} type={"number"} min={"1"} max={"59"} defaultValue={"30"} aria-label={"Минуты интервала"} className={"w-14 rounded border border-slate-700 bg-slate-950 px-2 py-1"} /> мин</span><button data-action-click={"saveMonitoringSettings"} className={"list-toolbar-button list-toolbar-button-secondary !min-h-8 !px-2.5 !py-1 text-xs"}>Сохранить</button><button data-action-click={"runMonitoring"} title={"Проверить все сайты сейчас"} className={"list-toolbar-button list-toolbar-button-primary !min-h-8 !px-2.5 !py-1 text-xs disabled:opacity-50"}>⟳ Проверить все</button></div></div>
            <p id={"monitoringStatus"} className={"text-xs text-slate-400"} role={"status"} aria-live={"polite"}>Планировщик проверяет сайты автоматически.</p>
            <div className={"flex gap-2"}><input id={"monitorHostInput"} name={"monitor-site-host"} type={"text"} autoComplete={"off"} autoCapitalize={"none"} spellCheck={false} inputMode={"url"} placeholder={"добавить домен, например rutracker.org"} aria-label={"Домен для мониторинга"} className={"min-w-0 flex-1 rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2 font-mono text-sm text-white placeholder:text-slate-500"} /><button data-action-click={"addMonitoringSite"} className={"list-toolbar-button list-toolbar-button-primary px-3 py-2 text-sm"}>＋ Добавить</button></div>
            <div id={"monitorSites"} className={"monitor-sites-grid grid gap-2 md:grid-cols-2"}></div>
          </section>
          <section id={"monitorSection-telegram"} className={"hidden ui-card rounded-2xl p-5 sm:p-6 space-y-4"}>
            <div><h2 className={"text-lg font-bold text-white"}>Уведомления в Telegram</h2><p className={"mt-1 text-sm leading-relaxed text-slate-400"}>Сообщение приходит, когда сайт из мониторинга перестаёт или снова начинает открываться. Создайте бота через @BotFather, отправьте ему сообщение, затем найдите ID чата.</p></div>
            <label className={"grid gap-2 text-sm text-slate-300 md:grid-cols-[11rem_minmax(0,35rem)] md:items-center"}>Токен бота<input id={"telegramToken"} type={"password"} autoComplete={"new-password"} placeholder={"123456:ABC…"} className={"rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2.5 text-white"} /></label>
            <label className={"grid gap-2 text-sm text-slate-300 md:grid-cols-[11rem_minmax(0,35rem)] md:items-center"}>Отправлять<select id={"telegramVia"} data-action-change={"toggleTelegramTransportFields"} data-value-source={"value"} className={"rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2.5 text-white"}><option value={"direct"}>как обычно</option><option value={"interface"}>через сетевой интерфейс</option><option value={"proxy"}>через прокси</option></select></label>
            <div id={"telegramInterfaceField"} className={"hidden grid gap-2 md:grid-cols-[11rem_minmax(0,35rem)] md:items-center"}><label htmlFor={"telegramInterface"} className={"text-sm text-slate-300"}>Сетевой интерфейс</label><input id={"telegramInterface"} placeholder={"например eth3"} className={"rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2.5 text-white"} /></div>
            <div id={"telegramProxyField"} className={"hidden grid gap-2 md:grid-cols-[11rem_minmax(0,35rem)] md:items-center"}><label htmlFor={"telegramProxy"} className={"text-sm text-slate-300"}>HTTP / SOCKS5 proxy</label><input id={"telegramProxy"} placeholder={"socks5h://127.0.0.1:1080"} className={"rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2.5 text-white"} /></div>
            <div className={"grid gap-2 md:grid-cols-[11rem_minmax(0,35rem)] md:items-center"}><label htmlFor={"telegramChatId"} className={"text-sm text-slate-300"}>ID чата</label><div className={"flex gap-2"}><input id={"telegramChatId"} placeholder={"123456789"} className={"min-w-0 flex-1 rounded-lg border border-slate-700 bg-[#060a12] px-3 py-2.5 font-mono text-white"} /><button data-action-click={"findTelegramChatId"} className={"rounded-lg border border-slate-700 px-3 py-2 text-sm text-white hover:bg-slate-800"}>⌕ Найти</button></div></div>
            <p id={"telegramStatus"} className={"text-xs text-slate-400"} role={"status"} aria-live={"polite"}></p><div className={"flex gap-2"}><button data-action-click={"saveTelegramSettings"} className={"rounded-lg bg-blue-600 px-4 py-2.5 text-sm font-semibold text-white hover:bg-blue-500"}>Сохранить</button><button data-action-click={"sendTestTelegram"} className={"rounded-lg border border-slate-700 px-4 py-2.5 text-sm font-semibold text-white hover:bg-slate-800"}>Отправить проверочное</button></div>
            <div id={"telegramNotifications"} className={"space-y-3"} aria-live={"polite"}></div>
          </section>
        </section>
        <section id={"tabContent-appearance"} className={"hidden view-animate space-y-5"}>
          <div><h1 className={"text-xl font-bold text-white"}>Настройки и оформление UI</h1><p className={"mt-1 text-sm text-slate-400"}>Управляйте внешним видом панели и доступом к ней.</p></div>
          <div className={"flex flex-wrap gap-2 border-b border-slate-800 pb-3"} role={"tablist"} aria-label={"Настройки UI"}>
            <button type={"button"} data-action-click={"showSettingsSection"} data-args={JSON.stringify(["appearance"])} aria-pressed={"true"} className={"settings-subtab settings-subtab-active rounded-lg border border-slate-700 px-3 py-2 text-sm font-semibold text-slate-100"}>Оформление UI</button>
            <button type={"button"} data-action-click={"showSettingsSection"} data-args={JSON.stringify(["security"])} aria-pressed={"false"} className={"settings-subtab rounded-lg border border-slate-800 px-3 py-2 text-sm font-semibold text-slate-400"}>Настройки UI</button>
          </div>
          <div id={"settingsAppearancePanel"} className={"settings-section-panel space-y-5"}>
          <section className={"ui-card max-w-4xl rounded-2xl p-5 sm:p-6 space-y-5"}>
            <div><h2 className={"font-bold text-white"}>Цветовая тема</h2><p className={"mt-1 text-xs text-slate-400"}>Выберите подходящее оформление панели.</p></div>
            <div className={"grid gap-3 sm:grid-cols-3"}>
              <label className={"appearance-choice"}><input type={"radio"} name={"appearanceTheme"} value={"dark"} data-action-change={"setAppearanceTheme"} data-value-source={"value"} /><span className={"appearance-preview appearance-preview-dark"}></span><span><strong>Стандартная</strong><small>Текущая тёмная тема</small></span></label>
              <label className={"appearance-choice"}><input type={"radio"} name={"appearanceTheme"} value={"light"} data-action-change={"setAppearanceTheme"} data-value-source={"value"} /><span className={"appearance-preview appearance-preview-light"}></span><span><strong>Светлая</strong><small>Светлый фон и контрастный текст</small></span></label>
              <label className={"appearance-choice"}><input type={"radio"} name={"appearanceTheme"} value={"neon"} data-action-change={"setAppearanceTheme"} data-value-source={"value"} /><span className={"appearance-preview appearance-preview-neon"}></span><span><strong>Фиолетово-неоновая</strong><small>Тёмная тема с фиолетовым акцентом</small></span></label>
            </div>
            <div className={"border-t border-slate-800 pt-4"}><h2 className={"font-bold text-white"}>Акцентная палитра</h2><p className={"mt-1 text-xs text-slate-400"}>Меняет цвет выделения, активного меню и основных кнопок.</p><div className={"mt-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-3"}>
              {[["blue", "Синий (стандартная)", "#3b82f6"], ["cyan", "Океан", "#06b6d4"], ["emerald", "Изумруд", "#10b981"], ["amber", "Янтарь", "#f59e0b"], ["rose", "Рубин", "#f43f5e"], ["violet", "Аметист", "#a855f7"]].map(([value, label, color]) => <label key={value} className={"appearance-accent-choice"}><input type={"radio"} name={"appearanceAccent"} value={value} data-action-change={"setAppearanceAccent"} data-value-source={"value"} /><span className={"appearance-accent-swatch"} style={{ backgroundColor: color }}></span><strong>{label}</strong></label>)}
            </div></div>
            <div className={"border-t border-slate-800 pt-4"}><h2 className={"font-bold text-white"}>Статус-бар</h2><p className={"mt-1 text-xs text-slate-400"}>Пять вариантов размещения состояния службы, управления и системных метрик.</p><div className={"mt-3 grid gap-2 sm:grid-cols-2"}>
              {[['standard', 'Классический (стандартный)', 'Состояние, управление и RAM / CPU в одну строку'], ['minimal', 'Минималистичный', 'Состояние и кнопки управления'], ['compact', 'Компактный с иконками', 'Те же данные при меньших отступах'], ['stacked', 'Двухрядный', 'Метрики переходят на отдельный ряд'], ['reverse', 'Обратный порядок', 'Метрики, управление и статус справа налево']].map(([value, label, description]) => <label key={value} className={`appearance-layout-choice appearance-status-${value}`}><input type={"radio"} name={"appearanceStatusBar"} value={value} data-action-change={"setAppearanceStatusBar"} data-value-source={"value"} /><span className={"appearance-layout-preview status-layout-preview"} aria-hidden={"true"}><i></i><i></i><i></i><i></i></span><span><strong>{label}</strong><small>{description}</small></span></label>)}
            </div></div>
            <label className={"flex cursor-pointer items-start gap-3 border-t border-slate-800 pt-4 text-sm text-slate-200"}><input type={"checkbox"} data-appearance-remind-timestamps={"true"} data-action-change={"setAppearanceTimestampReminder"} data-value-source={"checked"} className={"mt-0.5 accent-blue-500"} /><span><strong>Напоминать о TCP timestamps в Windows</strong><small className={"mt-1 block text-xs text-slate-400"}>Показывать предупреждение, если стратегия использует только TCP timestamps. При необходимости включите их командой <code>netsh interface tcp set global timestamps=enabled</code>.</small></span></label>
            <label className={"flex cursor-pointer items-start gap-3 border-t border-slate-800 pt-4 text-sm text-slate-200"}><input type={"checkbox"} data-appearance-config-warnings={"true"} data-action-change={"setAppearanceConfigWarnings"} data-value-source={"checked"} className={"mt-0.5 accent-blue-500"} /><span><strong>Показывать предупреждения конфигурации</strong><small className={"mt-1 block text-xs text-slate-400"}>Проверять черновик и показывать найденные предупреждения под редактором. Ошибки синтаксиса по-прежнему проверяются перед применением.</small></span></label>
            <div className={"border-t border-slate-800 pt-4"}><h2 className={"font-bold text-white"}>Расположение меню</h2><p className={"mt-1 text-xs text-slate-400"}>Варианты расположения всей навигации. Все разделы остаются доступны, кроме группы диагностики в режиме «Компактный».</p><div className={"mt-3 grid gap-3 sm:grid-cols-2 lg:grid-cols-3"}>
              {[["standard", "Стандартный", "Слева, вертикальный список"], ["compact", "Компактный", "Скрывает группу диагностики"], ["dense", "Плотный", "Меньше отступов"], ["wide", "Широкий", "Расширенная боковая панель"], ["right", "Справа", "Навигация у правого края"], ["top", "Верхнее меню", "Разделы в горизонтальной строке"], ["tiles", "Плитки", "Разделы в виде карточек"], ["simplified", "Группы", "Шесть компактных вкладок с разделами"]].map(([value, label, description]) => <label key={value} className={`appearance-layout-choice appearance-layout-${value}`}><input type={"radio"} name={"appearanceLayout"} value={value} data-action-change={"setAppearanceLayout"} data-value-source={"value"} /><span className={"appearance-layout-preview"} aria-hidden={"true"}><i></i><i></i><i></i><i></i></span><span><strong>{label}</strong><small>{description}</small></span></label>)}
            </div></div>
          </section>
          </div>
          <section id={"settingsSecurityPanel"} className={"settings-section-panel hidden ui-card max-w-4xl rounded-2xl p-5 sm:p-6 space-y-5"}>
            <div><h2 className={"font-bold text-white"}>Защита входа в панель</h2><p className={"mt-1 text-sm text-slate-400"}>Пароль хранится на роутере в виде стойкого хэша. Без отметки сеанс действует 24 часа. Срок галочки «Запомнить меня» задаётся ниже и применяется при следующих входах.</p></div>
            <label className={"flex items-center gap-3 text-sm text-slate-200"}><input id={"panelAuthEnabled"} type={"checkbox"} className={"accent-blue-500"} /><span><strong>Требовать пароль при открытии панели</strong><small id={"panelAuthState"} className={"mt-1 block text-xs text-slate-400"}>Загрузка состояния…</small></span></label>
            <label className={"block text-sm text-slate-300"}>Новый пароль <input id={"panelAuthPassword"} type={"password"} autoComplete={"new-password"} maxLength={1024} placeholder={"Введите пароль"} className={"mt-1.5 w-full max-w-lg rounded-lg border border-slate-700 bg-slate-950 px-3 py-2.5 text-white"} /></label>
            <label id={"panelAuthCurrentWrap"} className={"hidden block text-sm text-slate-300"}>Текущий пароль <input id={"panelAuthCurrentPassword"} type={"password"} autoComplete={"current-password"} className={"mt-1.5 w-full max-w-lg rounded-lg border border-slate-700 bg-slate-950 px-3 py-2.5 text-white"} /></label>
            <label className={"block max-w-lg text-sm text-slate-300"}>Забывать вход через<select id={"panelAuthRememberDays"} className={"mt-1.5 w-full rounded-lg border border-slate-700 bg-slate-950 px-3 py-2.5 text-white"}><option value={"1"}>1 день</option><option value={"7"}>7 дней</option><option value={"30"}>30 дней</option><option value={"90"}>90 дней</option><option value={"365"}>365 дней</option><option value={"0"}>Никогда</option></select><small className={"mt-1 block text-xs text-slate-500"}>При входе на новом устройстве срок применяется, только если отметить «Запомнить меня».</small></label>
            <p className={"text-xs text-slate-500"}>Можно задать пароль даже из одного символа; пустое поле оставляет пароль без изменений. При изменении уже включённых настроек потребуется текущий пароль.</p>
            <div className={"flex flex-wrap gap-2"}><button type={"button"} data-action-click={"savePanelAuthSettings"} className={"rounded-lg bg-blue-600 px-4 py-2.5 text-sm font-semibold text-white hover:bg-blue-500"}>Сохранить настройки</button><button type={"button"} data-action-click={"logoutPanel"} className={"rounded-lg border border-slate-700 px-4 py-2.5 text-sm font-semibold text-slate-200 hover:bg-slate-800"}>Выйти из панели</button></div>
            <p id={"panelAuthFeedback"} className={"text-sm text-slate-400"} role={"status"} aria-live={"polite"}></p>
          </section>
        </section>
        <section id={"tabContent-about"} className={"hidden view-animate space-y-5"}>
          <div><h1 className={"text-xl font-bold text-white"}>О nfqws2-UI</h1><p className={"mt-1 text-sm text-slate-400"}>Версия панели, обновления и команды управления на роутере.</p></div>
          <section className={"ui-card max-w-4xl rounded-2xl p-5 sm:p-6 space-y-5"}>
            <div className={"flex flex-wrap items-center justify-between gap-3 border-b border-slate-800 pb-4"}><div><h2 className={"font-bold text-white"}>nfqws2-UI</h2><a href={"https://github.com/rangerxxyy/nfqws2-ui"} target={"_blank"} rel={"noreferrer"} className={"mt-1 inline-block text-sm text-blue-400 hover:text-blue-300"}>github.com/rangerxxyy/nfqws2-ui ↗</a></div><div className={"flex items-center gap-3"}><p className={"text-sm text-slate-300"}>Установлена: <strong id={"aboutCurrentVersion"} className={"font-mono text-white"}>—</strong><br />Последняя: <strong id={"aboutLatestVersion"} className={"font-mono text-white"}>—</strong></p><button data-action-click={"checkUiUpdate"} className={"rounded-lg border border-slate-700 px-3 py-2 text-sm font-semibold text-slate-100 hover:bg-slate-800"}>↻ Проверить обновление</button></div></div>
            <p id={"aboutUpdateStatus"} className={"text-sm text-slate-400"} role={"status"} aria-live={"polite"}>Загрузка сведений о версии…</p>
            <div><h2 className={"font-bold text-white"}>Команды в терминале роутера</h2><p className={"mt-1 text-sm text-slate-400"}>Подключитесь к роутеру по SSH, затем скопируйте нужную команду целиком. Кнопка подтвердит копирование.</p><div className={"mt-3 grid gap-2"}>
              {[["nfqws2-ui-ctl update", "Обновить панель до последней стабильной версии"], ["nfqws2-ui-ctl update --check", "Проверить релиз; команда установки появится после загрузки его версии"], ["nfqws2-ui-ctl status", "Посмотреть состояние панели и nfqws2"], ["nfqws2-ui-ctl help", "Открыть справку по командам управления"]].map(([command, description], index) => <div key={command} className={"about-command-row"}><div className={"min-w-0"}><code data-about-version-command={index === 1 ? "true" : undefined}>{command}</code><small data-about-version-command-description={index === 1 ? "true" : undefined}>{description}</small></div><button aria-label={"Копировать команду"} title={"Копировать команду"} data-action-click={"copySnippetToClipboard"} data-args={JSON.stringify([{source:"self"}])} className={"rounded-lg border border-slate-700 px-3 py-2 text-xs text-slate-200 hover:bg-slate-800"}>Копировать</button></div>)}
            </div><p id={"aboutCopyStatus"} className={"mt-2 text-xs text-slate-400"} role={"status"} aria-live={"polite"}></p><p className={"mt-3 text-xs text-slate-500"}><code>nfqws2-ui-ctl</code> — отдельная команда панели rangerxxyy; штатное управление nfqws2 не меняется.</p></div>
          </section>
        </section>
        <section id={"tabContent-community"} className={"hidden view-animate space-y-3"}>
          <div className={"flex flex-wrap items-end justify-between gap-2"}><div><h1 className={"text-lg font-bold text-white"}>Стратегии сообщества</h1><p className={"mt-0.5 text-xs text-slate-400"}>Подсказки для сети вашего провайдера из открытой базы Zemidala. Перед применением каждая стратегия проходит отдельную проверку на роутере.</p></div><a id={"communityDatabaseLink"} href={"https://github.com/zemidala/nfqws2-strategies"} target={"_blank"} rel={"noreferrer"} className={"text-xs font-medium text-blue-300 hover:text-blue-200"}>База сообщества ↗</a></div>
          <section className={"community-provider-card"} aria-label={"Провайдер и ASN"}>
            <div className={"community-provider-topline"}><div className={"min-w-0"}><p className={"community-provider-caption"}>Ваш провайдер</p><p id={"communityProviderSummary"} className={"community-provider-name"}>Провайдер не выбран</p></div><div className={"community-provider-actions"}><button type={"button"} id={"communityEditProvider"} data-action-click={"editCommunityProvider"} className={"backup-action hidden"}>Изменить</button><button type={"button"} data-action-click={"detectCommunityProvider"} className={"backup-action"}>Определить автоматически</button><button type={"button"} data-action-click={"refreshCommunityStrategies"} className={"backup-action"}>Обновить</button></div></div>
            <div id={"communityProviderEditor"} className={"community-provider-editor"}><label className={"community-field"}>Номер ASN<input id={"communityProviderAsn"} inputMode={"numeric"} placeholder={"например, 12389"} /></label><label className={"community-field community-provider-name-field"}>Название провайдера<input id={"communityProviderName"} maxLength={120} placeholder={"определится автоматически или введите вручную"} /></label><div className={"community-provider-edit-actions"}><button type={"button"} data-action-click={"saveCommunityProvider"} className={"backup-action community-provider-save"}>Сохранить</button><button type={"button"} data-action-click={"cancelCommunityProviderEdit"} className={"backup-action community-provider-save"}>Отмена</button></div></div>
            <div className={"community-provider-footer"}><p id={"communityDatabaseSummary"} role={"status"} aria-live={"polite"}>При первом открытии определим ASN и загрузим базу.</p><p id={"communityDatabaseError"} className={"hidden community-provider-error"} role={"status"} aria-live={"polite"}></p></div>
          </section>
          <label className={"community-test-host"}>Сайт для проверки, если стратегия не содержит домен<input id={"communityTestHost"} autoCapitalize={"none"} spellCheck={false} placeholder={"example.com"} /></label>
          <div id={"communityStrategiesList"} className={"grid gap-2"}></div>
          <section className={"community-how-card"}>
            <h2 className={"community-how-title"}>Что это значит и как это устроено</h2>
            <div className={"community-how-grid"}>
              <div><h3>Что это значит</h3><p>У разных интернет-провайдеров фильтрация трафика отличается. Здесь собраны стратегии, которые подтвердились у пользователей того же провайдера.</p></div>
              <div><h3>Как это устроено</h3><p>При подборе включите «Стратегии сообщества». Каждая найденная стратегия проверяется на вашем роутере и не меняет конфигурацию без предпросмотра и подтверждения.</p></div>
              <div><h3>Подтверждения и публикация</h3><p>После проверки можно подтвердить рабочую стратегию, отметить неудачную или поделиться своей. Откроется форма GitHub; IP-адрес в неё не передаётся.</p></div>
              <div><h3>Открытая база</h3><p>Список хранится в <a href={"https://github.com/zemidala/nfqws2-strategies"} target={"_blank"} rel={"noreferrer"}>репозитории сообщества</a>. Перед применением кандидаты проходят обычную проверку nfqws2-ui.</p></div>
            </div>
          </section>
        </section>
        <section id={"tabContent-profiles"} className={"hidden view-animate space-y-4"}>
          <div className={"flex flex-wrap items-center justify-between gap-3"}><div><h1 className={"text-xl font-bold text-white"}>Конструктор стратегий</h1><p className={"mt-1 text-sm text-slate-400"}>Настройка профилей nfqws2 из раздела конфигурации.</p></div><div className={"flex flex-wrap gap-2"}><button type={"button"} data-action-click={"pasteRuntimeProfile"} className={"backup-action"}>Вставить чужой</button><button type={"button"} data-action-click={"addRuntimeProfile"} className={"backup-action"}>＋ Свой профиль</button><button type={"button"} data-action-click={"saveRuntimeProfiles"} className={"rounded-lg bg-blue-600 px-4 py-2 text-sm font-semibold text-white hover:bg-blue-500"}>Применить изменения</button></div></div>
          <section className={"ui-card max-w-[90rem] rounded-xl p-3 sm:p-4"}>
            <p id={"profileStatus"} className={"mb-3 text-sm text-slate-400"} role={"status"} aria-live={"polite"}>Профили загружаются из конфигурации nfqws2…</p>
            <div className={"grid min-h-[34rem] gap-3 lg:grid-cols-[15rem_minmax(0,1fr)]"}>
              <nav id={"profileList"} className={"grid content-start gap-1 rounded-lg border border-slate-800 bg-slate-950/40 p-2"} aria-label={"Порядок профилей nfqws2"}></nav>
              <div id={"profileEditor"} className={"min-w-0 rounded-lg border border-slate-800 bg-slate-950/20 p-3 sm:p-4"}></div>
            </div>
          </section>
        </section>
        <section id={"tabContent-snapshots"} className={"hidden view-animate space-y-4"}>
          <div><h1 className={"text-lg font-bold text-white"}>Резервные копии</h1><p className={"mt-1 text-xs text-slate-400"}>Ручные снимки и автоматическая защита конфигурации.</p></div>
          <section className={"ui-card max-w-5xl rounded-xl p-4 sm:p-5"}>
            <div className={"flex flex-wrap items-center justify-between gap-2"}><div><h2 className={"text-sm font-semibold text-white"}>Снимки на роутере</h2><p id={"snapshotCount"} className={"mt-0.5 text-[11px] text-slate-400"}>—</p></div><div className={"flex flex-wrap gap-1.5"}><button type={"button"} data-action-click={"createSnapshot"} className={"rounded-lg bg-blue-600 px-2.5 py-1.5 text-xs font-semibold text-white hover:bg-blue-500"}>＋ Создать снимок</button><label className={"backup-action cursor-pointer"}>Импортировать<input id={"snapshotRestoreFile"} type={"file"} accept={".tar.gz,application/gzip"} className={"hidden"} data-action-change={"restoreSnapshotFile"} data-args={JSON.stringify([{source:"self"}])} /></label></div></div>
            <div className={"mt-2 flex flex-wrap items-center justify-between gap-2 border-y border-slate-800 py-2"}><label className={"inline-flex items-center gap-2 text-xs font-medium text-slate-200"}><input id={"snapshotAutoEnabled"} type={"checkbox"} defaultChecked={true} disabled={true} data-action-change={"toggleAutomaticSnapshots"} data-value-source={"checked"} /><span>Автоматически создавать копии</span></label><p className={"text-[11px] text-slate-500"}>Отключает копии перед изменениями и ежедневное расписание; восстановление всегда сохраняет страховочную копию.</p></div>
            <div className={"mt-3 grid gap-2 sm:grid-cols-[repeat(3,minmax(0,1fr))_auto] sm:items-end"}>
              <label className={"block text-xs font-medium text-slate-300"}>Максимум локальных копий<input id={"snapshotMaxCount"} type={"number"} min={5} max={500} step={1} className={"mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-2 text-sm text-white"} /></label>
              <label className={"block text-xs font-medium text-slate-300"}>Срок хранения, дней<input id={"snapshotDays"} type={"number"} min={1} max={3650} step={1} className={"mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-2 text-sm text-white"} /></label>
              <label className={"block text-xs font-medium text-slate-300"}>Лимит места, МиБ<input id={"snapshotMaxLocalMib"} type={"number"} min={320} max={16384} step={64} className={"mt-1 w-full rounded-lg border border-slate-700 bg-slate-950 px-2.5 py-2 text-sm text-white"} /></label>
              <button type={"button"} data-action-click={"saveSnapshotRetention"} className={"rounded-lg bg-blue-600 px-3 py-2 text-xs font-semibold text-white hover:bg-blue-500"}>Сохранить ограничения</button>
            </div>
            <p id={"snapshotStorageStatus"} className={"mt-2 text-[11px] text-slate-400"} role={"status"} aria-live={"polite"}>Лимит хранения снимков загружается…</p>
            <p id={"snapshotDiskSpaceStatus"} className={"mt-1 text-[11px] text-slate-400"} role={"status"} aria-live={"polite"}>Свободное место на разделе снимков загружается…</p>
            <p id={"backupStatus"} className={"mt-2 text-[11px] text-slate-400"} role={"status"} aria-live={"polite"}>Снимки загружаются с роутера…</p><p id={"snapshotDirectory"} className={"mt-0.5 break-all font-mono text-[10px] text-slate-500"}></p><div id={"backupList"} className={"mt-2 grid gap-1.5"}></div>
          </section>
        </section>
        <section id={"tabContent-changes"} className={"hidden view-animate space-y-4"}>
          <div className={"flex flex-wrap items-center justify-between gap-2"}>
            <div><h1 className={"text-xl font-bold text-white"}>История изменений</h1><p className={"mt-0.5 text-xs text-slate-400"}>Конфигурация и списки · в том числе изменения вне панели</p></div>
            <div className={"change-history-controls"}>
              <button id={"changeHistoryUndoLast"} type={"button"} disabled={true} data-action-click={"requestUndoLastChange"} className={"backup-action"}>↶ Отменить последнее</button>
              <button id={"changeHistoryRestoreInitial"} type={"button"} disabled={true} data-action-click={"requestRestoreInitialHistory"} className={"backup-action"}>⟲ Вернуть как при установке</button>
              <button type={"button"} data-action-click={"refreshChangeHistory"} className={"backup-action"} aria-label={"Обновить историю"} title={"Обновить"}>↻</button>
            </div>
          </div>
          <section className={"ui-card rounded-xl p-3 sm:p-4"}>
            <div className={"flex flex-wrap gap-2"}><input id={"changeHistorySearch"} type={"search"} placeholder={"Фильтр по файлу или подписи"} className={"min-w-[14rem] flex-1 rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"} data-action-input={"filterChangeHistory"} /></div>
            <p id={"changeHistoryStatus"} className={"my-2 text-xs text-slate-400"} role={"status"} aria-live={"polite"}></p>
            <div id={"changeHistoryList"} className={"grid gap-1"}></div>
          </section>
        </section>
        <section id={"tabContent-logs"} className={"hidden view-animate space-y-4"}>
          <div><h1 className={"text-xl font-bold text-white"}>Журнал</h1><p className={"mt-1 text-sm text-slate-400"}>Логи читает backend на роутере ограниченным хвостом. Автообновление можно приостановить, пока вы читаете журнал.</p></div>
          <section className={"ui-card rounded-xl p-4 sm:p-5"}><div className={"flex flex-wrap items-center gap-2"}><label className={"text-sm text-slate-300"} htmlFor={"logsSource"}>Источник</label><select id={"logsSource"} defaultValue={"nfqws2-ui"} data-action-change={"changeLogSource"} data-value-source={"value"} className={"rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-white"}><option value={"nfqws2-ui"}>nfqws2-UI</option><option value={"nfqws2"}>nfqws2</option></select><input id={"logsSearch"} type={"search"} placeholder={"Фильтр строк"} data-action-input={"filterLogs"} className={"min-w-[12rem] flex-1 rounded-lg border border-slate-700 bg-slate-950 px-3 py-2 text-sm text-white"} /><button type={"button"} data-action-click={"refreshLogs"} className={"backup-action"}>Обновить</button><button type={"button"} data-action-click={"downloadLogs"} className={"backup-action"}>Скачать</button><button type={"button"} data-action-click={"clearLogs"} className={"backup-action border-rose-700/60 text-rose-300 hover:bg-rose-950/40"}>Очистить</button><label className={"ml-auto inline-flex items-center gap-2 rounded-lg border border-slate-800 px-3 py-2 text-xs text-slate-300"}><input id={"logsAutoRefresh"} type={"checkbox"} defaultChecked={true} data-action-change={"toggleLogAutoRefresh"} data-value-source={"checked"} /><span>Автообновление</span></label></div><p id={"logsAutoRefreshStatus"} className={"mt-2 text-xs text-slate-500"} aria-live={"polite"}>Автообновление каждые 5 секунд</p><p id={"logsStatus"} className={"my-2 text-xs text-slate-400"} role={"status"} aria-live={"polite"}>Журнал загружается…</p><pre id={"logsOutput"} className={"max-h-[70vh] overflow-auto rounded-lg border border-slate-800 bg-[#060a12] p-3 font-mono text-xs leading-5 text-slate-200 whitespace-pre-wrap break-all"}></pre></section>
        </section>
        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}

        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}

        {"\n\n      "}
        {"\n      "}
        {"\n      "}
        {"\n      "}

        {"\n"}
      </main>
      {"\n\n  "}
    </div>
{"\n\n  "}
{"\n  "}
{"\n  "}
{"\n  "}
    <div id={"updateModalBackdrop"} data-app-modal-root={"true"} aria-hidden={"true"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-50 hidden items-center justify-center p-4"}>
      {"\n    "}
      <div role={"dialog"} aria-modal={"true"} aria-labelledby={"updateModalTitle"} tabIndex={-1} className={"bg-[#0b101b] border border-slate-800 rounded-2xl max-w-2xl w-full overflow-hidden shadow-2xl view-animate"}>
        {"\n      \n      "}
        <div className={"px-6 pt-5 pb-4 flex items-center justify-between"}>
          {"\n        "}
          <div className={"flex items-center gap-3"}>
            {"\n          "}
            <svg className={"w-6 h-6 text-blue-500"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2.2"} d={"M4 16v2a2 2 0 002 2h12a2 2 0 002-2v-2M7 10l5 5m0 0l5-5m-5 5V3"}></path>
              {"\n          "}
            </svg>
            {"\n          "}
            <h3 id={"updateModalTitle"} className={"text-lg sm:text-xl font-bold text-white tracking-tight"}>
              {"Обновление nfqws2-UI"}
            </h3>
            {"\n        "}
          </div>
          {"\n\n        "}
          <button data-action-click={"closeUpdateModal"} className={"text-slate-400 hover:text-white p-1 rounded-lg transition-colors"}>
            {"\n          "}
            <svg className={"w-5 h-5"} fill={"none"} stroke={"currentColor"} viewBox={"0 0 24 24"}>
              {"\n            "}
              <path strokeLinecap={"round"} strokeLinejoin={"round"} strokeWidth={"2"} d={"M6 18L18 6M6 6l12 12"}></path>
              {"\n          "}
            </svg>
            {"\n        "}
          </button>
          {"\n      "}
        </div>
        {"\n\n      "}
        <div className={"px-6 pb-3 flex flex-wrap items-center justify-between gap-2"}>
          {"\n        "}
          <span id={"updatePickerHint"} className={"text-sm text-slate-400"}>
            {"Выберите версию панели"}
          </span>
          {"\n        "}
          {"\n\n        "}
          <div className={"flex items-center gap-2"}>
            {"\n          "}
            <span id={"updateRepoSlug"} className={"px-3 py-1 rounded-full bg-[#111827] border border-slate-700/80 text-xs font-mono font-bold text-slate-200"}>
              {"\n            rangerxxyy/nfqws2-ui\n          "}
            </span>
            {"\n          "}
            <span className={"px-2.5 py-1 rounded-full bg-[#064e3b] text-[#34d399] text-xs font-bold"}>
              {"\n            GitHub\n          "}
            </span>
            {"\n        "}
          </div>
          {"\n      "}
        </div>
        {"\n\n      "}
        <div id={"updateVersionsList"} className={"px-6 py-2 max-h-[410px] overflow-y-auto space-y-2.5"}></div>
        {"\n\n      "}
        <div id={"updateInstallProgressBox"} className={"hidden px-6 pt-3"}>
          {"\n        "}
          <div className={"flex items-center justify-between text-xs font-mono text-blue-300 mb-1.5"}>
            {"\n          "}
            <span id={"updateInstallStepText"}>
              {"Скачивание пакета с GitHub Releases..."}
            </span>
            {"\n          "}
            <span id={"updateInstallPercent"}>
              {"0%"}
            </span>
            {"\n        "}
          </div>
          {"\n        "}
          <div className={"w-full bg-slate-900 h-2 rounded-full overflow-hidden"}>
            {"\n          "}
            <div id={"updateInstallBar"} className={"bg-blue-500 h-full transition-all duration-300"} style={{ width: "0%" }}></div>
            {"\n        "}
          </div>
          {"\n      "}
        </div>
        {"\n\n      "}
        <div className={"p-6 pt-4"}>
          {"\n        "}
          <button id={"confirmUpdateInstallBtn"} data-action-click={"installSelectedRelease"} className={"w-full py-3 rounded-xl bg-[#1d4ed8] hover:bg-[#2563eb] text-white font-bold text-sm transition-all shadow-lg shadow-blue-600/25 active:scale-[0.99]"}>
            {"\n          Установить\n        "}
          </button>
          {"\n      "}
        </div>
        {"\n\n    "}
      </div>
      {"\n  "}
    </div>
{"\n\n  "}
{"\n  "}
    <div id={"diffModalBackdrop"} data-app-modal-root={"true"} aria-hidden={"true"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-50 hidden items-center justify-center p-4"}>
      {"\n    "}
      <div role={"dialog"} aria-modal={"true"} aria-labelledby={"diffModalTitle"} tabIndex={-1} className={"bg-[#0b101b] border border-slate-800 rounded-2xl max-w-3xl w-full overflow-hidden shadow-2xl view-animate"}>
        {"\n      "}
        <div className={"px-6 py-4 border-b border-slate-800 flex items-center justify-between"}>
          {"\n        "}
          <h3 id={"diffModalTitle"} className={"text-base font-bold text-white"}>
            {"Сравнение с сохранённым `nfqws.conf`"}
          </h3>
          <button type={"button"} data-action-click={"closeDiffModal"} aria-label={"Закрыть сравнение"} className={"backup-action"}>Закрыть</button>
          {"\n        "}
          {"\n      "}
        </div>
        {"\n      "}
        <div id={"diffModalBody"} className={"p-6 font-mono text-xs max-h-[460px] overflow-y-auto space-y-1.5 bg-[#060a12]"}></div>
        {"\n    "}
      </div>
      {"\n  "}
    </div>
{"\n\n  "}
{"\n  "}
    <div id={"toastWrapper"} className={"fixed bottom-24 right-5 z-[40] space-y-2.5 pointer-events-none"}></div>
    <div id={"updateNoticeWrapper"} className={"fixed bottom-5 right-5 z-[60] grid gap-2.5 pointer-events-none"}></div>
    <div id={"configClearModal"} data-app-modal-root={"true"} aria-hidden={"true"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-[70] hidden items-center justify-center p-4"}>
      <section className={"w-full max-w-md rounded-2xl border border-slate-700 bg-[#0b101b] p-5 shadow-2xl view-animate"} role={"dialog"} aria-modal={"true"} aria-labelledby={"configClearTitle"}>
        <h3 id={"configClearTitle"} className={"text-base font-bold text-white"}>Очистить конфигурацию?</h3>
        <p className={"mt-2 text-sm text-slate-300"}>Весь текст в редакторе будет удалён. Изменения затронут только черновик и не применятся к службе, пока вы отдельно не нажмёте «Применить конфигурацию».</p>
        <div className={"mt-5 flex justify-end gap-2"}>
          <button type={"button"} data-action-click={"cancelClearConfig"} className={"rounded-lg border border-slate-700 px-3 py-2 text-sm text-slate-300 hover:bg-slate-800"}>Отмена</button>
          <button type={"button"} data-action-click={"confirmClearConfig"} className={"rounded-lg bg-rose-600 px-3 py-2 text-sm font-semibold text-white hover:bg-rose-500"}>Очистить</button>
        </div>
      </section>
    </div>
    <div id={"listClearModal"} data-app-modal-root={"true"} aria-hidden={"true"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-[70] hidden items-center justify-center p-4"}>
      <section className={"w-full max-w-md rounded-2xl border border-slate-700 bg-[#0b101b] p-5 shadow-2xl view-animate"} role={"dialog"} aria-modal={"true"} aria-labelledby={"listClearTitle"}>
        <h3 id={"listClearTitle"} className={"text-base font-bold text-white"}>Очистить черновик списка?</h3>
        <p className={"mt-2 text-sm text-slate-300"}>Содержимое текущего файла будет удалено только из черновика. Чтобы записать изменения, отдельно нажмите «Применить список».</p>
        <div className={"mt-5 flex justify-end gap-2"}>
          <button type={"button"} data-action-click={"cancelClearListDraft"} className={"rounded-lg border border-slate-700 px-3 py-2 text-sm text-slate-300 hover:bg-slate-800"}>Отмена</button>
          <button type={"button"} data-action-click={"confirmClearListDraft"} className={"rounded-lg bg-rose-600 px-3 py-2 text-sm font-semibold text-white hover:bg-rose-500"}>Очистить черновик</button>
        </div>
      </section>
    </div>
    <div id={"strategyHistoryClearModal"} data-app-modal-root={"true"} aria-hidden={"true"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-[70] hidden items-center justify-center p-4"}>
      <section className={"w-full max-w-md rounded-2xl border border-slate-700 bg-[#0b101b] p-5 shadow-2xl view-animate"} role={"dialog"} aria-modal={"true"} aria-labelledby={"strategyHistoryClearTitle"}>
        <h3 id={"strategyHistoryClearTitle"} className={"text-base font-bold text-white"}>Очистить историю подборов?</h3>
        <p className={"mt-2 text-sm text-slate-300"}>Удалятся сохранённые результаты завершённых подборов. Текущая проверка продолжится и останется в истории.</p>
        <div className={"mt-5 flex justify-end gap-2"}>
          <button type={"button"} data-action-click={"cancelClearStrategyPickHistory"} className={"rounded-lg border border-slate-700 px-3 py-2 text-sm text-slate-300 hover:bg-slate-800"}>Отмена</button>
          <button type={"button"} data-action-click={"confirmClearStrategyPickHistory"} className={"rounded-lg bg-rose-600 px-3 py-2 text-sm font-semibold text-white hover:bg-rose-500"}>Очистить историю</button>
        </div>
      </section>
    </div>
    <div id={"customListDeleteModal"} data-app-modal-root={"true"} aria-hidden={"true"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-[70] hidden items-center justify-center p-4"}>
      <section className={"w-full max-w-md rounded-2xl border border-slate-700 bg-[#0b101b] p-5 shadow-2xl view-animate"} role={"dialog"} aria-modal={"true"} aria-labelledby={"customListDeleteTitle"}>
        <h3 id={"customListDeleteTitle"} className={"text-base font-bold text-white"}>Удалить пользовательский список?</h3>
        <p className={"mt-2 text-sm text-slate-300"}>Файл <code id={"customListDeleteName"} className={"text-sky-200"}></code> будет удалён из <code>/lists</code>. Несохранённый текст этого файла в редакторе тоже будет потерян.</p>
        <div className={"mt-5 flex justify-end gap-2"}>
          <button type={"button"} data-action-click={"cancelDeleteCustomList"} title={"Закрыть подтверждение и оставить список"} className={"rounded-lg border border-slate-700 px-3 py-2 text-sm text-slate-300 hover:bg-slate-800"}>Отмена</button>
          <button id={"confirmCustomListDelete"} type={"button"} data-action-click={"confirmDeleteCustomList"} title={"Удалить только этот пользовательский файл списка"} className={"rounded-lg bg-rose-600 px-3 py-2 text-sm font-semibold text-white hover:bg-rose-500"}>Удалить</button>
        </div>
      </section>
    </div>
    <div id={"operationConfirmModal"} data-app-modal-root={"true"} aria-hidden={"true"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-[80] hidden items-center justify-center p-4"}>
      <section className={"w-full max-w-md rounded-2xl border border-slate-700 bg-[#0b101b] p-5 shadow-2xl view-animate"} role={"alertdialog"} aria-modal={"true"} aria-labelledby={"operationConfirmTitle"} aria-describedby={"operationConfirmMessage"}>
        <h3 id={"operationConfirmTitle"} className={"text-base font-bold text-white"}>Подтвердите действие</h3>
        <p id={"operationConfirmMessage"} className={"mt-2 text-sm leading-6 text-slate-300 whitespace-pre-wrap"}></p>
        <div className={"mt-5 flex justify-end gap-2"}><button type={"button"} data-action-click={"cancelOperationConfirmation"} className={"rounded-lg border border-slate-700 px-3 py-2 text-sm text-slate-300 hover:bg-slate-800"}>Отмена</button><button type={"button"} data-action-click={"confirmOperationConfirmation"} className={"rounded-lg bg-rose-600 px-3 py-2 text-sm font-semibold text-white hover:bg-rose-500"}>Продолжить</button></div>
      </section>
    </div>
    <div id={"communityShareModal"} data-app-modal-root={"true"} aria-hidden={"true"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-[82] hidden items-center justify-center p-4"}>
      <section className={"w-full max-w-xl rounded-xl border border-slate-700 bg-[#0b101b] p-4 shadow-2xl"} role={"dialog"} aria-modal={"true"} aria-labelledby={"communityShareTitle"}>
        <h3 id={"communityShareTitle"} className={"text-base font-bold text-white"}>Поделиться проверенной стратегией</h3>
        <p className={"mt-1 text-xs leading-5 text-slate-400"}>Откроется заполненная публичная форма GitHub. Проверьте данные и отправьте её самостоятельно. IP-адрес роутера не передаётся.</p>
        <div className={"mt-3 grid gap-2 sm:grid-cols-[8rem_minmax(0,1fr)]"}><label className={"text-[11px] text-slate-400"}>ASN провайдера<input id={"communityShareAsn"} inputMode={"numeric"} className={"mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100"} /></label><label className={"text-[11px] text-slate-400"}>Провайдер<input id={"communityShareProvider"} maxLength={120} className={"mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100"} /></label></div>
        <label className={"mt-2 block text-[11px] text-slate-400"}>Сайт, на котором стратегия проверена<input id={"communityShareHost"} className={"mt-1 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100"} /></label>
        <pre id={"communitySharePreview"} className={"mt-3 max-h-40 overflow-auto rounded-lg border border-slate-800 bg-slate-950 p-2.5 font-mono text-[11px] leading-4 text-slate-300 whitespace-pre-wrap break-all"}></pre>
        <p id={"communityShareStatus"} className={"mt-1 text-[11px] text-amber-300"} role={"status"} aria-live={"polite"}></p>
        <div className={"mt-3 flex justify-end gap-2"}><button type={"button"} data-action-click={"closeCommunityShare"} className={"backup-action"}>Отмена</button><button type={"button"} data-action-click={"submitCommunityShare"} className={"rounded-lg bg-blue-600 px-3 py-2 text-xs font-semibold text-white hover:bg-blue-500"}>Открыть форму GitHub</button></div>
      </section>
    </div>
    <div id={"profileImportDialog"} data-app-modal-root={"true"} aria-hidden={"true"} className={"fixed inset-0 bg-black/75 backdrop-blur-sm z-[75] hidden items-center justify-center p-4"}>
      <section className={"w-full max-w-2xl rounded-2xl border border-slate-700 bg-[#0b101b] p-5 shadow-2xl view-animate"} role={"dialog"} aria-modal={"true"} aria-labelledby={"profileImportTitle"}>
        <h3 id={"profileImportTitle"} className={"text-base font-bold text-white"}>Вставить профиль nfqws2</h3>
        <p className={"mt-2 text-sm text-slate-400"}>Вставьте аргументы профиля, включая параметры фильтров и шаги desync. Разделители --new будут импортированы отдельными профилями.</p>
        <textarea id={"profileImportInput"} rows={10} spellCheck={false} className={"mt-4 w-full rounded-lg border border-slate-700 bg-slate-950 p-3 font-mono text-xs leading-5 text-slate-100 focus:border-blue-500 focus:outline-none"}></textarea>
        <div className={"mt-4 flex justify-end gap-2"}><button type={"button"} data-action-click={"cancelProfileImport"} className={"backup-action"}>Отмена</button><button type={"button"} data-action-click={"acceptProfileImport"} className={"rounded-lg bg-blue-600 px-3 py-2 text-sm font-semibold text-white hover:bg-blue-500"}>Добавить</button></div>
      </section>
    </div>
{"\n\n  "}
{"\n  "}
{"\n  "}
{"\n  "}
{"\n"}
    </>
  );
}
