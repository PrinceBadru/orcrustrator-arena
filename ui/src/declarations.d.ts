declare module 'lucide-react' {
  import * as React from 'react';
  export interface IconProps extends React.SVGProps<SVGSVGElement> {
    size?: number | string;
    strokeWidth?: number | string;
    color?: string;
  }
  export type Icon = React.FC<IconProps>;
  export const Zap: Icon;
  export const Activity: Icon;
  export const Shield: Icon;
  export const Sliders: Icon;
  export const Terminal: Icon;
  export const FileText: Icon;
  export const Layers: Icon;
  export const Cpu: Icon;
  export const RefreshCw: Icon;
  export const Sparkles: Icon;
  export const BookOpen: Icon;
  export const Play: Icon;
  export const Pause: Icon;
  export const Flame: Icon;
  export const CheckCircle2: Icon;
  export const XCircle: Icon;
  export const Clock: Icon;
  export const Network: Icon;
  export const Skull: Icon;
  export const AlertTriangle: Icon;
  export const RotateCcw: Icon;
  export const Power: Icon;
  export const ShieldCheck: Icon;
  export const ShieldAlert: Icon;
  export const AlertCircle: Icon;
  export const ChevronRight: Icon;
  export const ChevronLeft: Icon;
  export const Check: Icon;
  export const AlertOctagon: Icon;
  export const RotateCw: Icon;
  export const GitCommit: Icon;
  export const Download: Icon;
  export const X: Icon;
}
