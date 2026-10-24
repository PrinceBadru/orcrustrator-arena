import React, { useState } from 'react';
import { Skull, AlertTriangle, Sliders, RotateCcw, Power } from 'lucide-react';

interface ChaosConsoleProps {
  onChaosChange?: (target: string, latency: number, errorRate: number, isOffline: boolean) => void;
}

export const ChaosConsole: React.FC<ChaosConsoleProps> = ({ onChaosChange }) => {
  const [selectedService, setSelectedService] = useState('billing');
  const [latency, setLatency] = useState(80);
  const [jitter, setJitter] = useState(25);
  const [errorRate, setErrorRate] = useState(0.35);
  const [isOffline, setIsOffline] = useState(false);
  const [seededChaos, setSeededChaos] = useState(true);

  const services = [
    { id: 'registry', name: 'Registry', port: 8081 },
    { id: 'lab', name: 'Lab Service', port: 8082 },
    { id: 'pharmacy', name: 'Pharmacy', port: 8083 },
    { id: 'terminology', name: 'Terminology', port: 8084 },
    { id: 'billing', name: 'Billing Service', port: 8085 },
    { id: 'notifications', name: 'Notifications', port: 8086 },
  ];

  const handleApply = (newOffline = isOffline) => {
    if (onChaosChange) {
      onChaosChange(selectedService, latency, errorRate, newOffline);
    }
  };

  const handleToggleKill = () => {
    const next = !isOffline;
    setIsOffline(next);
    handleApply(next);
  };

  const handleReset = () => {
    setLatency(0);
    setJitter(0);
    setErrorRate(0);
    setIsOffline(false);
    if (onChaosChange) {
      onChaosChange(selectedService, 0, 0, false);
    }
  };

  return (
    <div className="bg-[#111520] border border-[#1e2433] rounded-xl p-5 shadow-2xl">
      <div className="flex items-center justify-between pb-4 border-b border-[#1e2433]">
        <div className="flex items-center gap-2">
          <Sliders className="w-5 h-5 text-red-500" />
          <h2 className="text-base font-bold text-white">Pull the Plug Console (Toxiproxy Chaos)</h2>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-[11px] font-mono text-slate-400">Seed:</span>
          <span className="text-[11px] font-mono text-amber-400 bg-amber-500/10 px-2 py-0.5 rounded border border-amber-500/20">
            0xDEADBEEF-42
          </span>
        </div>
      </div>

      {/* Target Service Picker */}
      <div className="mt-4">
        <label className="text-xs font-semibold text-slate-300 block mb-2">Target Microservice:</label>
        <div className="grid grid-cols-3 gap-2">
          {services.map((svc) => (
            <button
              key={svc.id}
              onClick={() => {
                setSelectedService(svc.id);
                setIsOffline(false);
              }}
              className={`p-2 rounded-lg text-left text-xs border transition ${
                selectedService === svc.id
                  ? 'bg-red-500/10 border-red-500/50 text-white shadow'
                  : 'bg-[#0a0d14] border-[#1e2433] text-slate-400 hover:text-slate-200'
              }`}
            >
              <div className="font-semibold">{svc.name}</div>
              <div className="text-[10px] text-slate-500 font-mono">:{svc.port}</div>
            </button>
          ))}
        </div>
      </div>

      {/* Sliders */}
      <div className="mt-5 space-y-4">
        <div>
          <div className="flex justify-between text-xs font-mono mb-1">
            <span className="text-slate-400">Injected Base Latency:</span>
            <span className="text-orange-400 font-bold">{latency} ms</span>
          </div>
          <input
            type="range"
            min="0"
            max="300"
            step="5"
            value={latency}
            onChange={(e) => {
              setLatency(Number(e.target.value));
              handleApply();
            }}
            className="w-full accent-orange-500 h-1.5 bg-[#1e2433] rounded-lg cursor-pointer"
          />
        </div>

        <div>
          <div className="flex justify-between text-xs font-mono mb-1">
            <span className="text-slate-400">Random Jitter:</span>
            <span className="text-orange-400 font-bold">±{jitter} ms</span>
          </div>
          <input
            type="range"
            min="0"
            max="100"
            step="5"
            value={jitter}
            onChange={(e) => {
              setJitter(Number(e.target.value));
              handleApply();
            }}
            className="w-full accent-orange-500 h-1.5 bg-[#1e2433] rounded-lg cursor-pointer"
          />
        </div>

        <div>
          <div className="flex justify-between text-xs font-mono mb-1">
            <span className="text-slate-400">Simulated Error Rate:</span>
            <span className="text-red-400 font-bold">{(errorRate * 100).toFixed(0)}%</span>
          </div>
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={errorRate}
            onChange={(e) => {
              setErrorRate(Number(e.target.value));
              handleApply();
            }}
            className="w-full accent-red-500 h-1.5 bg-[#1e2433] rounded-lg cursor-pointer"
          />
        </div>
      </div>

      {/* Action Buttons */}
      <div className="mt-6 flex items-center gap-3">
        <button
          onClick={handleToggleKill}
          className={`flex-1 flex items-center justify-center gap-2 py-2.5 rounded-lg font-bold text-xs shadow-lg transition active:scale-95 ${
            isOffline
              ? 'bg-red-600 hover:bg-red-500 text-white animate-pulse shadow-red-950/50'
              : 'bg-red-950/40 hover:bg-red-900/60 text-red-300 border border-red-500/30'
          }`}
        >
          <Power className="w-4 h-4" />
          {isOffline ? 'OFFLINE (KILL SWITCH ACTIVE)' : 'TRIGGER KILL SWITCH'}
        </button>

        <button
          onClick={handleReset}
          className="p-2.5 rounded-lg bg-[#1e2433] hover:bg-[#283247] text-slate-300 text-xs border border-slate-700 transition"
          title="Reset All Chaos Knobs"
        >
          <RotateCcw className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
};
