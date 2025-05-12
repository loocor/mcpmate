import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import { formatDistance } from 'date-fns';

/**
 * Combines multiple class names with Tailwind support
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Formats a number of bytes to a human-readable string
 */
export function formatBytes(bytes: number, decimals = 2) {
  if (bytes === 0) return '0 Bytes';
  
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
  
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

/**
 * Formats an uptime in seconds to a human-readable string
 */
export function formatUptime(seconds: number) {
  if (seconds < 60) return `${seconds}s`;
  
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
  
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ${minutes % 60}m`;
  
  const days = Math.floor(hours / 24);
  return `${days}d ${hours % 24}h`;
}

/**
 * Formats a timestamp to a relative time
 */
export function formatRelativeTime(timestamp: string) {
  try {
    const date = new Date(timestamp);
    return formatDistance(date, new Date(), { addSuffix: true });
  } catch (error) {
    return 'Invalid date';
  }
}

/**
 * Get a color variant based on status
 */
export function getStatusVariant(status: string): "success" | "warning" | "destructive" | "default" {
  switch (status.toLowerCase()) {
    case 'connected':
    case 'running':
    case 'healthy':
      return 'success';
    case 'disconnected':
    case 'initializing':
      return 'warning';
    case 'error':
    case 'unhealthy':
    case 'stopped':
      return 'destructive';
    default:
      return 'default';
  }
}

/**
 * Truncate a string if it's too long
 */
export function truncate(str: string, length: number) {
  if (str.length <= length) return str;
  return str.slice(0, length) + '...';
}