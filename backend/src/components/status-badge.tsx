import { Badge } from './ui/badge';
import { getStatusVariant } from '../lib/utils';
import { InstanceSummary } from '../lib/types';

interface StatusBadgeProps {
  status?: string;
  instances?: InstanceSummary[];
  showLabel?: boolean;
  className?: string;
  blinkOnError?: boolean;
}

export function StatusBadge({
  status = 'unknown',
  instances = [],
  showLabel = true,
  className = '',
  blinkOnError = true
}: StatusBadgeProps) {
  // 如果提供了实例数组，根据实例状态确定整体状态
  let statusStr = status?.toString() || 'unknown';
  let shouldBlink = false;

  if (instances && instances.length > 0) {
    // 检查是否有任何实例处于运行状态
    const hasRunningInstance = instances.some(
      instance => instance.status === 'running' || instance.status === 'connected'
    );

    // 检查是否有任何实例处于错误状态
    const hasErrorInstance = instances.some(
      instance => instance.status === 'error' || instance.status === 'unhealthy'
    );

    if (hasRunningInstance) {
      statusStr = 'running';
    } else if (hasErrorInstance) {
      statusStr = 'error';
      shouldBlink = blinkOnError;
    } else {
      statusStr = 'disconnected';
    }
  } else if (statusStr === 'error' && blinkOnError) {
    shouldBlink = true;
  }

  const variant = getStatusVariant(statusStr);

  // 根据状态确定显示的文本
  let displayText = statusStr;
  if (statusStr === 'running' || statusStr === 'connected') {
    displayText = 'Normal';
  } else if (statusStr === 'error' || statusStr === 'unhealthy') {
    displayText = 'Error';
  } else if (statusStr === 'disconnected' || statusStr === 'stopped') {
    displayText = 'Disconnected';
  } else if (statusStr === 'initializing') {
    displayText = 'Initializing';
  } else {
    displayText = 'Unknown';
  }

  return (
    <Badge
      variant={variant}
      className={`${className} ${shouldBlink ? 'animate-pulse' : ''}`}
    >
      <span className="flex items-center">
        <span className={`mr-1 h-2 w-2 rounded-full ${variant === 'success' ? 'bg-emerald-400' :
          variant === 'warning' ? 'bg-amber-400' :
            variant === 'destructive' ? 'bg-red-400' : 'bg-slate-400'
          }`} />
        {showLabel && displayText}
      </span>
    </Badge>
  );
}