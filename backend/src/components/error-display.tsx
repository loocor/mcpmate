import React from 'react';
import { Alert, AlertTitle, AlertDescription } from './ui/alert';
import { Button } from './ui/button';
import { RefreshCw } from 'lucide-react';

interface ErrorDisplayProps {
  title?: string;
  error: Error | string | null;
  onRetry?: () => void;
}

export function ErrorDisplay({ 
  title = "Error", 
  error, 
  onRetry 
}: ErrorDisplayProps) {
  if (!error) return null;
  
  const errorMessage = typeof error === 'string' 
    ? error 
    : error.message || 'An unknown error occurred';
  
  return (
    <Alert variant="destructive" className="my-4">
      <AlertTitle className="flex items-center justify-between">
        {title}
        {onRetry && (
          <Button 
            variant="outline" 
            size="sm" 
            onClick={onRetry}
            className="ml-2"
          >
            <RefreshCw className="mr-2 h-4 w-4" />
            Retry
          </Button>
        )}
      </AlertTitle>
      <AlertDescription className="mt-2">
        {errorMessage}
      </AlertDescription>
    </Alert>
  );
}
