import React, { useState, useEffect } from 'react';
import { BookOpen, ChevronRight, ChevronLeft, Play, Pause, Sparkles, Check } from 'lucide-react';

interface StoryStep {
  title: string;
  badge: string;
  narration: string;
  technicalInsight: string;
  recommendedAction: string;
}

export const StoryMode: React.FC<{ onStepTrigger?: (stepIdx: number) => void }> = ({ onStepTrigger }) => {
  const [currentStep, setCurrentStep] = useState(0);
  const [isPlaying, setIsPlaying] = useState(false);

  const steps: StoryStep[] = [
    {
      title: '1. The Serial Staircase Trap',
      badge: 'Cold Read Aggregation',
      narration:
        'Traditional API gateways handle composite endpoints by executing queries sequentially. Notice how a single patient summary requires 7 round trips: registry, labs, pharmacy, 3 separate terminology calls, and billing.',
      technicalInsight:
        'Latency compounds additively: L_total = sum(L_i). Even with 10ms microservices, total latency exceeds 80ms.',
      recommendedAction: 'Observe the Naive gateway waterfall lane.',
    },
    {
      title: '2. Fearless Tokio Concurrency',
      badge: 'DAG Parallelism',
      narration:
        'Hand-rolled and Orcrustrator gateways use Tokio task DAGs to fetch independent resources simultaneously. The root patient record and independent joins execute concurrently.',
      technicalInsight:
        'Latency is bounded by the slowest single endpoint: L_total = max(L_root, L_join) + max(L_children).',
      recommendedAction: 'Notice the parallel block in the Hand-Rolled and Orcrustrator lanes.',
    },
    {
      title: '3. Read-Through Microsecond Caching',
      badge: 'Zero-Allocation RAM',
      narration:
        'On the second request, Orcrustrator’s pluggable MemoryCache serves the entire stitched JSON AST directly from RAM in microseconds with zero allocations.',
      technicalInsight:
        'Bypasses all upstream network calls, reducing upstream service pressure to 0 req/sec while caching is warm.',
      recommendedAction: 'Click "Warm Cache Hit" to see 0.12ms sub-millisecond retrieval.',
    },
    {
      title: '4. Injecting Chaos: Breaking Billing Live',
      badge: 'Fault Tolerance',
      narration:
        'Now let’s pull the plug on the Billing Service using Toxiproxy. We inject 150ms of network latency and a 50% simulated failure rate.',
      technicalInsight:
        'In a distributed environment, transient timeouts occur constantly. How the system degrades determines production survival.',
      recommendedAction: 'Check the Pull the Plug Console with Billing marked as Degraded.',
    },
    {
      title: '5. The Saga & Consistency Ledger',
      badge: 'Distributed Transactions',
      narration:
        'When a write fails mid-flight, Naive architectures leave orphaned encounter and pharmacy records behind. Orcrustrator immediately triggers compensating rollbacks in reverse order.',
      technicalInsight:
        'The Consistency Auditor verifies that zero ghost invoices or unfulfilled reservations remain in the system.',
      recommendedAction: 'Inspect the Consistency Ledger to see clean state vs red orphaned records.',
    },
  ];

  useEffect(() => {
    let timer: NodeJS.Timeout;
    if (isPlaying) {
      timer = setInterval(() => {
        setCurrentStep((prev) => {
          const next = (prev + 1) % steps.length;
          if (onStepTrigger) onStepTrigger(next);
          return next;
        });
      }, 7000);
    }
    return () => clearInterval(timer);
  }, [isPlaying, steps.length, onStepTrigger]);

  const goToStep = (idx: number) => {
    setCurrentStep(idx);
    if (onStepTrigger) onStepTrigger(idx);
  };

  const step = steps[currentStep];

  return (
    <div className="bg-[#111520] border border-orange-500/30 rounded-xl p-5 shadow-2xl relative overflow-hidden">
      <div className="absolute top-0 right-0 w-48 h-48 bg-orange-500/5 rounded-full blur-3xl pointer-events-none" />

      {/* Header */}
      <div className="flex items-center justify-between pb-4 border-b border-[#1e2433]">
        <div className="flex items-center gap-2">
          <BookOpen className="w-5 h-5 text-orange-400" />
          <h2 className="text-base font-bold text-white">Story Mode (5-Minute Guided Presentation)</h2>
          <span className="text-[10px] bg-orange-500/20 text-orange-400 font-mono px-2 py-0.5 rounded border border-orange-500/30">
            Step {currentStep + 1} of {steps.length}
          </span>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsPlaying(!isPlaying)}
            className="flex items-center gap-1.5 bg-[#1e2433] hover:bg-[#283247] text-slate-200 text-xs px-2.5 py-1 rounded-lg border border-slate-700 transition"
          >
            {isPlaying ? <Pause className="w-3.5 h-3.5" /> : <Play className="w-3.5 h-3.5" />}
            {isPlaying ? 'Pause Auto-Play' : 'Auto-Play Story'}
          </button>
        </div>
      </div>

      {/* Narrative Card */}
      <div className="mt-4 p-4 rounded-lg bg-[#0a0d14] border border-[#1e2433]">
        <div className="flex items-center justify-between mb-2">
          <h3 className="text-sm font-bold text-white flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-orange-400" />
            {step.title}
          </h3>
          <span className="text-[11px] font-mono text-emerald-400 bg-emerald-500/10 px-2 py-0.5 rounded border border-emerald-500/20">
            {step.badge}
          </span>
        </div>

        <p className="text-xs text-slate-300 leading-relaxed mt-2">{step.narration}</p>

        <div className="mt-3 p-2.5 rounded bg-[#111520] border border-[#1e2433] text-[11px] font-mono text-slate-400">
          <strong className="text-orange-400">Technical Note:</strong> {step.technicalInsight}
        </div>
      </div>

      {/* Navigation Controls */}
      <div className="mt-4 flex items-center justify-between">
        <div className="flex gap-1.5">
          {steps.map((_, i) => (
            <button
              key={i}
              onClick={() => goToStep(i)}
              className={`w-2.5 h-2.5 rounded-full transition ${
                currentStep === i ? 'bg-orange-500 scale-125' : 'bg-[#1e2433] hover:bg-slate-600'
              }`}
            />
          ))}
        </div>

        <div className="flex items-center gap-2">
          <button
            disabled={currentStep === 0}
            onClick={() => goToStep(currentStep - 1)}
            className="flex items-center gap-1 text-xs text-slate-400 hover:text-white disabled:opacity-30 px-2 py-1 rounded transition"
          >
            <ChevronLeft className="w-4 h-4" /> Prev
          </button>
          <button
            disabled={currentStep === steps.length - 1}
            onClick={() => goToStep(currentStep + 1)}
            className="flex items-center gap-1 text-xs bg-orange-600 hover:bg-orange-500 text-white font-medium px-3 py-1.5 rounded-lg shadow transition disabled:opacity-30"
          >
            Next <ChevronRight className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
};
