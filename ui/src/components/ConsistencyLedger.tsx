import React, { useState } from 'react';
import { ShieldCheck, ShieldAlert, AlertCircle, RefreshCw, FileText, CheckCircle2 } from 'lucide-react';

interface LedgerRecord {
  id: string;
  type: 'Encounter' | 'Reservation' | 'Invoice' | 'Notification';
  service: string;
  gateway: 'Naive' | 'Hand-Rolled' | 'Orcrustrator';
  status: 'ACTIVE' | 'CANCELLED' | 'VOIDED' | 'ORPHANED';
  timestamp: string;
}

export const ConsistencyLedger: React.FC<{ hasOrphanedRecords?: boolean }> = ({ hasOrphanedRecords = false }) => {
  const [isAuditing, setIsAuditing] = useState(false);

  const records: LedgerRecord[] = hasOrphanedRecords
    ? [
        { id: 'enc_98a72b1', type: 'Encounter', service: 'Registry :8081', gateway: 'Naive', status: 'ORPHANED', timestamp: 'Just now' },
        { id: 'res_11f42c9', type: 'Reservation', service: 'Pharmacy :8083', gateway: 'Naive', status: 'ORPHANED', timestamp: 'Just now' },
        { id: 'enc_33c90e2', type: 'Encounter', service: 'Registry :8081', gateway: 'Orcrustrator', status: 'CANCELLED', timestamp: 'Just now' },
        { id: 'res_88e10d4', type: 'Reservation', service: 'Pharmacy :8083', gateway: 'Orcrustrator', status: 'CANCELLED', timestamp: 'Just now' },
      ]
    : [
        { id: 'enc_55b21a8', type: 'Encounter', service: 'Registry :8081', gateway: 'Orcrustrator', status: 'ACTIVE', timestamp: '1m ago' },
        { id: 'res_77d12f0', type: 'Reservation', service: 'Pharmacy :8083', gateway: 'Orcrustrator', status: 'ACTIVE', timestamp: '1m ago' },
        { id: 'inv_99a41c3', type: 'Invoice', service: 'Billing :8085', gateway: 'Orcrustrator', status: 'ACTIVE', timestamp: '1m ago' },
        { id: 'not_22e90f1', type: 'Notification', service: 'Notifications :8086', gateway: 'Orcrustrator', status: 'ACTIVE', timestamp: '1m ago' },
      ];

  const handleAudit = () => {
    setIsAuditing(true);
    setTimeout(() => setIsAuditing(false), 500);
  };

  const violationsCount = records.filter((r) => r.status === 'ORPHANED').length;

  return (
    <div className="bg-[#111520] border border-[#1e2433] rounded-xl p-5 shadow-2xl">
      <div className="flex items-center justify-between pb-4 border-b border-[#1e2433]">
        <div className="flex items-center gap-2">
          {violationsCount > 0 ? (
            <ShieldAlert className="w-5 h-5 text-red-500 animate-bounce" />
          ) : (
            <ShieldCheck className="w-5 h-5 text-emerald-400" />
          )}
          <h2 className="text-base font-bold text-white">Consistency Ledger & Auditor Violations</h2>
        </div>

        <div className="flex items-center gap-3">
          <span
            className={`text-xs font-mono font-bold px-2 py-0.5 rounded border ${
              violationsCount > 0
                ? 'bg-red-500/20 text-red-400 border-red-500/30'
                : 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30'
            }`}
          >
            {violationsCount > 0 ? `${violationsCount} Invariant Violations` : '0 Violations (Clean)'}
          </span>
          <button
            onClick={handleAudit}
            disabled={isAuditing}
            className="flex items-center gap-1.5 bg-[#1e2433] hover:bg-[#283247] text-slate-300 text-xs px-2.5 py-1 rounded-lg border border-slate-700 transition"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isAuditing ? 'animate-spin' : ''}`} />
            Run Audit
          </button>
        </div>
      </div>

      {/* Rules Banner */}
      <div className="mt-4 p-3 rounded-lg bg-[#0a0d14] border border-[#1e2433] text-[11px] font-mono text-slate-400 flex flex-col md:flex-row md:items-center justify-between gap-2">
        <div>
          <span className="text-slate-200 font-semibold">Active Invariant Rules:</span> [1] No invoice without encounter • [2] No reserved stock without invoice • [3] Zero ghost records on abort
        </div>
      </div>

      {/* Record Grid */}
      <div className="mt-4 grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
        {records.map((rec, idx) => {
          const isOrphaned = rec.status === 'ORPHANED';
          const isCancelled = rec.status === 'CANCELLED' || rec.status === 'VOIDED';

          return (
            <div
              key={idx}
              className={`p-3.5 rounded-lg border transition ${
                isOrphaned
                  ? 'bg-red-950/20 border-red-500/60 shadow-lg shadow-red-950/50 animate-pulse'
                  : isCancelled
                  ? 'bg-amber-950/15 border-amber-500/40'
                  : 'bg-[#0a0d14] border-[#1e2433]'
              }`}
            >
              <div className="flex items-center justify-between mb-2">
                <span className="text-xs font-mono font-bold text-slate-200">{rec.type}</span>
                <span
                  className={`text-[10px] font-mono px-1.5 py-0.5 rounded font-bold ${
                    isOrphaned
                      ? 'bg-red-500 text-white'
                      : isCancelled
                      ? 'bg-amber-500/20 text-amber-300'
                      : 'bg-emerald-500/20 text-emerald-400'
                  }`}
                >
                  {rec.status}
                </span>
              </div>
              <div className="text-xs font-mono text-slate-300 truncate">{rec.id}</div>
              <div className="text-[10px] text-slate-500 font-mono mt-1 flex justify-between">
                <span>{rec.service}</span>
                <span>{rec.gateway}</span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
