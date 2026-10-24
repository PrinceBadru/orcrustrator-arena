import React, { useState } from 'react';
import { Terminal, CheckCircle2, AlertOctagon, RotateCw, GitCommit } from 'lucide-react';

export const ArchitectureLinter: React.FC = () => {
  const [hasCycle, setHasCycle] = useState(false);
  const [isChecking, setIsChecking] = useState(false);
  const [checkResult, setCheckResult] = useState<'passed' | 'failed' | null>('passed');

  const runCheck = (cycleState = hasCycle) => {
    setIsChecking(true);
    setTimeout(() => {
      setIsChecking(false);
      setCheckResult(cycleState ? 'failed' : 'passed');
    }, 450);
  };

  const toggleCycle = () => {
    const next = !hasCycle;
    setHasCycle(next);
    runCheck(next);
  };

  return (
    <div className="bg-[#111520] border border-[#1e2433] rounded-xl p-5 shadow-2xl">
      <div className="flex items-center justify-between pb-4 border-b border-[#1e2433]">
        <div className="flex items-center gap-2">
          <Terminal className="w-5 h-5 text-indigo-400" />
          <h2 className="text-base font-bold text-white">Architecture Linter (`orcrustrator compile --check`)</h2>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={toggleCycle}
            className={`text-xs px-2.5 py-1 rounded-lg border font-mono transition ${
              hasCycle
                ? 'bg-red-500/20 text-red-400 border-red-500/40 font-bold'
                : 'bg-[#1e2433] text-slate-300 border-slate-700 hover:text-white'
            }`}
          >
            {hasCycle ? 'Cycle Injected (A -> B -> A)' : 'Inject Circular Relation'}
          </button>

          <button
            onClick={() => runCheck()}
            disabled={isChecking}
            className="flex items-center gap-1.5 bg-indigo-600 hover:bg-indigo-500 text-white text-xs px-3 py-1 rounded-lg font-medium transition"
          >
            <RotateCw className={`w-3.5 h-3.5 ${isChecking ? 'animate-spin' : ''}`} />
            Run Check
          </button>
        </div>
      </div>

      {/* Terminal View */}
      <div className="mt-4 p-4 rounded-lg bg-[#06080d] border border-[#1e2433] font-mono text-xs">
        <div className="flex items-center justify-between text-slate-500 pb-2 mb-2 border-b border-slate-900">
          <span className="flex items-center gap-2">
            <span className="w-2.5 h-2.5 rounded-full bg-red-500/80 inline-block" />
            <span className="w-2.5 h-2.5 rounded-full bg-amber-500/80 inline-block" />
            <span className="w-2.5 h-2.5 rounded-full bg-emerald-500/80 inline-block" />
            <span className="text-[11px] text-slate-400 ml-2">CI Gate: Kahn Topological DAG Validator</span>
          </span>
          <span>v0.2.1</span>
        </div>

        <div className="text-slate-300">
          <span className="text-emerald-400">$</span> orcrustrator compile --check
        </div>
        <div className="text-slate-500 mt-1">Scanning Rust AST for relation! declarations...</div>
        <div className="text-slate-500">Discovered 4 relational links across 6 service boundaries.</div>

        {checkResult === 'passed' && (
          <div className="mt-3 p-2.5 rounded bg-emerald-950/20 border border-emerald-500/40 text-emerald-400 flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 shrink-0" />
            <span>Success: DAG verification passed. Zero cycles detected. definitions.toml is up to date.</span>
          </div>
        )}

        {checkResult === 'failed' && (
          <div className="mt-3 p-2.5 rounded bg-red-950/25 border border-red-500/50 text-red-300 flex items-start gap-2 animate-pulse">
            <AlertOctagon className="w-4 h-4 text-red-400 shrink-0 mt-0.5" />
            <div>
              <div className="font-bold text-red-400">Build Failure: Cyclic Relationship Graph Detected</div>
              <div className="text-[11px] text-red-300/90 mt-1">
                Kahn topological sort failed: Cycle found between &apos;patient_registry&apos; and &apos;billing_service&apos;.
                Endpoints cannot depend on each other symmetrically. Compilation aborted with exit code 1.
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
