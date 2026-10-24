import React, { useState } from 'react';
import {
  Zap,
  Activity,
  Shield,
  Sliders,
  Terminal,
  FileText,
  Layers,
  Cpu,
  RefreshCw,
  Sparkles,
  BookOpen,
} from 'lucide-react';
import { TheRace, GatewayResult } from './components/TheRace';
import { ServiceMap } from './components/ServiceMap';
import { ChaosConsole } from './components/ChaosConsole';
import { ConsistencyLedger } from './components/ConsistencyLedger';
import { ArchitectureLinter } from './components/ArchitectureLinter';
import { StoryMode } from './components/StoryMode';
import { ReportModal } from './components/ReportModal';

export const App: React.FC = () => {
  const [chaosTarget, setChaosTarget] = useState<string | null>('billing');
  const [chaosEnabled, setChaosEnabled] = useState(false);
  const [hasOrphanedRecords, setHasOrphanedRecords] = useState(false);
  const [isReportOpen, setIsReportOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<'arena' | 'story' | 'architecture'>('arena');

  const handleChaosChange = (target: string, latency: number, errorRate: number, isOffline: boolean) => {
    setChaosTarget(target);
    const active = latency > 0 || errorRate > 0 || isOffline;
    setChaosEnabled(active);
    if (active && (target === 'billing' || target === 'pharmacy')) {
      setHasOrphanedRecords(true);
    } else {
      setHasOrphanedRecords(false);
    }
  };

  const handleRaceComplete = (results: GatewayResult[]) => {
    const naiveHasError = results.some((r) => r.gateway.includes('Naive') && r.status !== 200);
    if (naiveHasError) {
      setHasOrphanedRecords(true);
    }
  };

  const handleStoryStep = (stepIdx: number) => {
    if (stepIdx === 3) {
      // Step 4: Inject chaos into billing
      setChaosTarget('billing');
      setChaosEnabled(true);
      setHasOrphanedRecords(true);
    } else if (stepIdx === 4) {
      // Step 5: Saga consistency audit
      setHasOrphanedRecords(true);
    }
  };

  return (
    <div className="min-h-screen bg-[#0a0d14] text-slate-100 flex flex-col font-sans">
      {/* Mission Control Navigation Bar */}
      <header className="border-b border-[#1e2433] bg-[#0d111a]/80 backdrop-blur sticky top-0 z-40 px-6 py-3.5 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-tr from-orange-600 to-amber-500 flex items-center justify-center shadow-lg shadow-orange-950/40">
            <Zap className="w-5 h-5 text-white fill-white" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-base font-extrabold tracking-tight text-white">Orcrustrator Arena</h1>
              <span className="text-[10px] bg-orange-500/10 text-orange-400 font-mono font-bold px-2 py-0.5 rounded border border-orange-500/20">
                PROD BENCHMARK
              </span>
            </div>
            <p className="text-[11px] text-slate-400 font-mono">
              The same request, run through 3 implementations side by side, while you break the system live.
            </p>
          </div>
        </div>

        {/* Global Nav & Status */}
        <div className="flex items-center gap-3">
          <div className="hidden md:flex items-center gap-2 bg-[#111520] p-1 rounded-lg border border-[#1e2433]">
            <button
              onClick={() => setActiveTab('arena')}
              className={`px-3 py-1.5 text-xs font-semibold rounded transition ${
                activeTab === 'arena' ? 'bg-[#1e2433] text-white' : 'text-slate-400 hover:text-white'
              }`}
            >
              Live Arena
            </button>
            <button
              onClick={() => setActiveTab('story')}
              className={`px-3 py-1.5 text-xs font-semibold rounded transition flex items-center gap-1.5 ${
                activeTab === 'story' ? 'bg-[#1e2433] text-orange-400' : 'text-slate-400 hover:text-white'
              }`}
            >
              <BookOpen className="w-3.5 h-3.5" /> Story Mode
            </button>
            <button
              onClick={() => setActiveTab('architecture')}
              className={`px-3 py-1.5 text-xs font-semibold rounded transition flex items-center gap-1.5 ${
                activeTab === 'architecture' ? 'bg-[#1e2433] text-indigo-400' : 'text-slate-400 hover:text-white'
              }`}
            >
              <Terminal className="w-3.5 h-3.5" /> Architecture Linter
            </button>
          </div>

          <button
            onClick={() => setIsReportOpen(true)}
            className="flex items-center gap-1.5 bg-[#1e2433] hover:bg-[#283247] text-slate-200 text-xs px-3.5 py-2 rounded-lg border border-slate-700 transition active:scale-95 shadow font-semibold"
          >
            <FileText className="w-4 h-4 text-orange-400" />
            Export Report
          </button>
        </div>
      </header>

      {/* Main Content Body */}
      <main className="flex-1 p-6 max-w-7xl w-full mx-auto space-y-6">
        {/* Footprint Live Banner */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 p-4 rounded-xl bg-[#111520] border border-[#1e2433]">
          <div>
            <span className="text-[11px] font-mono text-slate-400 block">Gateway 1: Naive</span>
            <div className="text-sm font-bold text-red-400 mt-0.5">82.4ms • 28MB RAM</div>
            <span className="text-[10px] text-slate-500 font-mono">Sequential Staircase</span>
          </div>
          <div>
            <span className="text-[11px] font-mono text-slate-400 block">Gateway 2: Hand-Rolled</span>
            <div className="text-sm font-bold text-blue-400 mt-0.5">24.1ms • 34MB RAM</div>
            <span className="text-[10px] text-slate-500 font-mono">Tokio Concurrency + Retries</span>
          </div>
          <div>
            <span className="text-[11px] font-mono text-slate-400 block">Gateway 3: Orcrustrator</span>
            <div className="text-sm font-bold text-emerald-400 mt-0.5">0.12ms • 22MB RAM</div>
            <span className="text-[10px] text-emerald-500/80 font-mono">Zero Server Bloat (Pure RAM)</span>
          </div>
          <div>
            <span className="text-[11px] font-mono text-slate-400 block">Auditor Status</span>
            <div className="text-sm font-bold text-slate-200 mt-0.5 flex items-center gap-1.5">
              <span className={`w-2 h-2 rounded-full ${hasOrphanedRecords ? 'bg-red-500 animate-ping' : 'bg-emerald-400'}`} />
              {hasOrphanedRecords ? 'Violations Flagged' : 'All Sagas Balanced'}
            </div>
            <span className="text-[10px] text-slate-500 font-mono">Invariant Monitor Active</span>
          </div>
        </div>

        {activeTab === 'story' && <StoryMode onStepTrigger={handleStoryStep} />}

        {activeTab === 'architecture' && <ArchitectureLinter />}

        {/* Section 1: The Race (Side-by-Side Waterfalls) */}
        <TheRace onRaceComplete={handleRaceComplete} chaosEnabled={chaosEnabled} />

        {/* Section 2: Live Service Map + Chaos Console */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          <div className="lg:col-span-7">
            <ServiceMap chaosTarget={chaosTarget} />
          </div>
          <div className="lg:col-span-5">
            <ChaosConsole onChaosChange={handleChaosChange} />
          </div>
        </div>

        {/* Section 3: Consistency Ledger */}
        <ConsistencyLedger hasOrphanedRecords={hasOrphanedRecords} />
      </main>

      {/* Report Modal */}
      <ReportModal isOpen={isReportOpen} onClose={() => setIsReportOpen(false)} />

      {/* Footer */}
      <footer className="border-t border-[#1e2433] bg-[#080a0f] py-4 px-6 text-center text-xs text-slate-500 font-mono">
        Orcrustrator Arena • Production Benchmarks &amp; Health Informatics DAG Orchestration • Designed for Windows Docker WSL2
      </footer>
    </div>
  );
};

export default App;
