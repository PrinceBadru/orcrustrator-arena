import React, { useEffect, useRef } from 'react';
import { Network, Activity } from 'lucide-react';

interface ServiceNode {
  id: string;
  name: string;
  role: string;
  port: number;
  x: number;
  y: number;
  health: 'healthy' | 'degraded' | 'offline';
  qps: number;
  latencyMs: number;
}

interface Edge {
  from: string;
  to: string;
  active: boolean;
}

export const ServiceMap: React.FC<{ chaosTarget?: string | null }> = ({ chaosTarget }) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let animationFrameId: number;

    const nodes: ServiceNode[] = [
      // Client
      { id: 'client', name: 'Arena Client', role: 'Traffic Generator', port: 3000, x: 80, y: 220, health: 'healthy', qps: 120, latencyMs: 2 },

      // Gateways
      { id: 'gw-naive', name: 'Naive Gateway', role: 'Sequential', port: 8001, x: 260, y: 110, health: 'healthy', qps: 40, latencyMs: 82 },
      { id: 'gw-handrolled', name: 'Hand-Rolled', role: 'Tokio::join!', port: 8002, x: 260, y: 220, health: 'healthy', qps: 40, latencyMs: 24 },
      { id: 'gw-orcrustrator', name: 'Orcrustrator', role: 'DAG + MemoryCache', port: 8003, x: 260, y: 330, health: 'healthy', qps: 40, latencyMs: 0.1 },

      // Microservices
      { id: 'svc-registry', name: 'Registry', role: 'Patients & Encounters', port: 8081, x: 520, y: 70, health: chaosTarget === 'registry' ? 'degraded' : 'healthy', qps: 45, latencyMs: 12 },
      { id: 'svc-lab', name: 'Lab Service', role: 'Hematology & Tests', port: 8082, x: 520, y: 145, health: chaosTarget === 'lab' ? 'degraded' : 'healthy', qps: 38, latencyMs: 14 },
      { id: 'svc-pharmacy', name: 'Pharmacy', role: 'Stock & Prescriptions', port: 8083, x: 520, y: 220, health: chaosTarget === 'pharmacy' ? 'degraded' : 'healthy', qps: 42, latencyMs: 15 },
      { id: 'svc-terminology', name: 'Terminology', role: 'LOINC / RxNorm', port: 8084, x: 520, y: 295, health: chaosTarget === 'terminology' ? 'degraded' : 'healthy', qps: 95, latencyMs: 9 },
      { id: 'svc-billing', name: 'Billing Service', role: 'Invoices & Accounts', port: 8085, x: 520, y: 370, health: chaosTarget === 'billing' ? 'degraded' : 'healthy', qps: 40, latencyMs: 18 },
      { id: 'svc-notifications', name: 'Notifications', role: 'SMS & Alerts', port: 8086, x: 520, y: 445, health: chaosTarget === 'notifications' ? 'degraded' : 'healthy', qps: 20, latencyMs: 11 },
    ];

    const edges: Edge[] = [
      { from: 'client', to: 'gw-naive', active: true },
      { from: 'client', to: 'gw-handrolled', active: true },
      { from: 'client', to: 'gw-orcrustrator', active: true },

      { from: 'gw-naive', to: 'svc-registry', active: true },
      { from: 'gw-naive', to: 'svc-lab', active: true },
      { from: 'gw-naive', to: 'svc-pharmacy', active: true },
      { from: 'gw-naive', to: 'svc-terminology', active: true },
      { from: 'gw-naive', to: 'svc-billing', active: true },

      { from: 'gw-handrolled', to: 'svc-registry', active: true },
      { from: 'gw-handrolled', to: 'svc-lab', active: true },
      { from: 'gw-handrolled', to: 'svc-pharmacy', active: true },
      { from: 'gw-handrolled', to: 'svc-terminology', active: true },
      { from: 'gw-handrolled', to: 'svc-billing', active: true },

      { from: 'gw-orcrustrator', to: 'svc-registry', active: true },
      { from: 'gw-orcrustrator', to: 'svc-lab', active: true },
      { from: 'gw-orcrustrator', to: 'svc-pharmacy', active: true },
      { from: 'gw-orcrustrator', to: 'svc-terminology', active: true },
      { from: 'gw-orcrustrator', to: 'svc-billing', active: true },
      { from: 'gw-orcrustrator', to: 'svc-notifications', active: true },
    ];

    // Particle flow animation
    const particles: { edgeIdx: number; progress: number; speed: number }[] = [];
    for (let i = 0; i < 40; i++) {
      particles.push({
        edgeIdx: Math.floor(Math.random() * edges.length),
        progress: Math.random(),
        speed: 0.006 + Math.random() * 0.008,
      });
    }

    let pulseTime = 0;

    const render = () => {
      pulseTime += 0.05;
      ctx.clearRect(0, 0, canvas.width, canvas.height);

      // Draw Edges
      edges.forEach((edge, idx) => {
        const fromNode = nodes.find((n) => n.id === edge.from);
        const toNode = nodes.find((n) => n.id === edge.to);
        if (!fromNode || !toNode) return;

        ctx.beginPath();
        ctx.moveTo(fromNode.x, fromNode.y);
        ctx.bezierCurveTo(
          fromNode.x + 80,
          fromNode.y,
          toNode.x - 80,
          toNode.y,
          toNode.x,
          toNode.y
        );

        ctx.strokeStyle = '#1e2433';
        ctx.lineWidth = 1.5;
        ctx.stroke();
      });

      // Update & Draw Flowing Particles
      particles.forEach((p) => {
        p.progress += p.speed;
        if (p.progress > 1) {
          p.progress = 0;
          p.edgeIdx = Math.floor(Math.random() * edges.length);
        }

        const edge = edges[p.edgeIdx];
        const fromNode = nodes.find((n) => n.id === edge.from);
        const toNode = nodes.find((n) => n.id === edge.to);
        if (!fromNode || !toNode) return;

        const t = p.progress;
        // Cubic bezier interpolation
        const cx1 = fromNode.x + 80;
        const cy1 = fromNode.y;
        const cx2 = toNode.x - 80;
        const cy2 = toNode.y;

        const px =
          Math.pow(1 - t, 3) * fromNode.x +
          3 * Math.pow(1 - t, 2) * t * cx1 +
          3 * (1 - t) * Math.pow(t, 2) * cx2 +
          Math.pow(t, 3) * toNode.x;
        const py =
          Math.pow(1 - t, 3) * fromNode.y +
          3 * Math.pow(1 - t, 2) * t * cy1 +
          3 * (1 - t) * Math.pow(t, 2) * cy2 +
          Math.pow(t, 3) * toNode.y;

        ctx.beginPath();
        ctx.arc(px, py, 2.5, 0, Math.PI * 2);
        ctx.fillStyle = edge.from === 'gw-orcrustrator' ? '#10b981' : '#f97316';
        ctx.shadowColor = ctx.fillStyle;
        ctx.shadowBlur = 6;
        ctx.fill();
        ctx.shadowBlur = 0;
      });

      // Draw Nodes
      nodes.forEach((node) => {
        const isDegraded = node.health === 'degraded';
        const isOrc = node.id.includes('orcrustrator');

        // Outer Glow / Pulse
        if (isDegraded) {
          const pulseRadius = 24 + Math.sin(pulseTime * 2) * 6;
          ctx.beginPath();
          ctx.arc(node.x, node.y, pulseRadius, 0, Math.PI * 2);
          ctx.fillStyle = 'rgba(239, 68, 68, 0.15)';
          ctx.fill();
        }

        // Node Circle
        ctx.beginPath();
        ctx.arc(node.x, node.y, 18, 0, Math.PI * 2);
        ctx.fillStyle = isDegraded ? '#ef4444' : isOrc ? '#10b981' : '#1e2433';
        ctx.strokeStyle = isDegraded ? '#f87171' : isOrc ? '#34d399' : '#334155';
        ctx.lineWidth = 2;
        ctx.fill();
        ctx.stroke();

        // Node Title
        ctx.fillStyle = '#f8fafc';
        ctx.font = 'bold 11px Inter, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(node.name, node.x, node.y + 32);

        // Subtitle / Port
        ctx.fillStyle = '#64748b';
        ctx.font = '9px JetBrains Mono, monospace';
        ctx.fillText(`:${node.port} • ${node.latencyMs}ms`, node.x, node.y + 45);
      });

      animationFrameId = requestAnimationFrame(render);
    };

    render();

    return () => cancelAnimationFrame(animationFrameId);
  }, [chaosTarget]);

  return (
    <div className="bg-[#111520] border border-[#1e2433] rounded-xl p-5 shadow-2xl">
      <div className="flex items-center justify-between pb-4 border-b border-[#1e2433]">
        <div className="flex items-center gap-2">
          <Network className="w-5 h-5 text-emerald-400" />
          <h2 className="text-base font-bold text-white">Live Service Mesh & Particle Flow Map</h2>
        </div>
        <div className="flex items-center gap-4 text-xs font-mono">
          <span className="flex items-center gap-1.5 text-emerald-400">
            <span className="w-2 h-2 rounded-full bg-emerald-400 animate-ping" /> Mesh Healthy
          </span>
          <span className="text-slate-400">60 FPS Canvas</span>
        </div>
      </div>

      <div className="relative mt-4 flex justify-center bg-[#07090e] rounded-lg border border-[#1e2433] overflow-hidden">
        <canvas ref={canvasRef} width={680} height={510} className="w-full max-w-[680px] h-auto" />
      </div>
    </div>
  );
};
