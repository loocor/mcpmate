import React, { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import { MCPServerConfig } from '../lib/types';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { Textarea } from './ui/textarea';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from './ui/dialog';
import { AlertCircle, Loader2 } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from './ui/alert';

// 表单验证模式
const serverFormSchema = z.object({
  name: z.string().min(1, '服务器名称不能为空').max(50, '服务器名称不能超过50个字符'),
  kind: z.enum(['stdio', 'sse', 'streamable_http'], {
    required_error: '请选择服务器类型',
  }),
  command: z.string().optional(),
  command_path: z.string().optional(),
  args: z.string().optional(),
  env: z.string().optional(),
  max_instances: z.coerce.number().int().positive().optional(),
});

type ServerFormValues = z.infer<typeof serverFormSchema>;

interface ServerFormProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (data: Partial<MCPServerConfig>) => Promise<void>;
  initialData?: Partial<MCPServerConfig>;
  title?: string;
  submitLabel?: string;
}

export function ServerForm({
  isOpen,
  onClose,
  onSubmit,
  initialData,
  title = '添加服务器',
  submitLabel = '保存',
}: ServerFormProps) {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 将初始数据转换为表单值
  const defaultValues: Partial<ServerFormValues> = {
    name: initialData?.name || '',
    kind: initialData?.kind || 'stdio',
    command: initialData?.command || '',
    command_path: initialData?.command_path || '',
    args: initialData?.args ? initialData.args.join(' ') : '',
    env: initialData?.env ? Object.entries(initialData.env).map(([key, value]) => `${key}=${value}`).join('\n') : '',
    max_instances: initialData?.max_instances || 1,
  };

  const {
    register,
    handleSubmit,
    formState: { errors },
    reset,
    setValue,
    watch,
  } = useForm<ServerFormValues>({
    resolver: zodResolver(serverFormSchema),
    defaultValues,
  });

  // 监听服务器类型变化
  const serverType = watch('kind');

  // 处理表单提交
  const handleFormSubmit = async (data: ServerFormValues) => {
    setIsSubmitting(true);
    setError(null);

    try {
      // 转换表单数据为服务器配置
      const serverConfig: Partial<MCPServerConfig> = {
        name: data.name,
        kind: data.kind,
        command: data.command || undefined,
        command_path: data.command_path || undefined,
        args: data.args ? data.args.split(' ').filter(Boolean) : undefined,
        env: data.env
          ? data.env.split('\n').reduce((acc, line) => {
              const [key, ...valueParts] = line.split('=');
              if (key && valueParts.length > 0) {
                acc[key.trim()] = valueParts.join('=').trim();
              }
              return acc;
            }, {} as Record<string, string>)
          : undefined,
        max_instances: data.max_instances || undefined,
      };

      await onSubmit(serverConfig);
      reset();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : '保存服务器配置时出错');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>
            配置服务器连接信息。不同类型的服务器需要不同的配置参数。
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit(handleFormSubmit)} className="space-y-4 py-4">
          {error && (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertTitle>错误</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="name">服务器名称</Label>
              <Input
                id="name"
                {...register('name')}
                placeholder="例如: my-server"
              />
              {errors.name && (
                <p className="text-xs text-red-500">{errors.name.message}</p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="kind">服务器类型</Label>
              <Select
                defaultValue={defaultValues.kind}
                onValueChange={(value) => setValue('kind', value as any)}
              >
                <SelectTrigger>
                  <SelectValue placeholder="选择服务器类型" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="stdio">标准输入输出 (stdio)</SelectItem>
                  <SelectItem value="sse">服务器发送事件 (SSE)</SelectItem>
                  <SelectItem value="streamable_http">HTTP流 (Streamable HTTP)</SelectItem>
                </SelectContent>
              </Select>
              {errors.kind && (
                <p className="text-xs text-red-500">{errors.kind.message}</p>
              )}
            </div>
          </div>

          {serverType === 'stdio' && (
            <>
              <div className="space-y-2">
                <Label htmlFor="command">命令</Label>
                <Input
                  id="command"
                  {...register('command')}
                  placeholder="例如: python -m my_script"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="command_path">命令路径</Label>
                <Input
                  id="command_path"
                  {...register('command_path')}
                  placeholder="例如: /usr/local/bin"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="args">参数 (空格分隔)</Label>
                <Input
                  id="args"
                  {...register('args')}
                  placeholder="例如: --debug --port 8080"
                />
              </div>
            </>
          )}

          {(serverType === 'sse' || serverType === 'streamable_http') && (
            <div className="space-y-2">
              <Label htmlFor="command">URL</Label>
              <Input
                id="command"
                {...register('command')}
                placeholder="例如: http://localhost:8080"
              />
            </div>
          )}

          <div className="space-y-2">
            <Label htmlFor="env">环境变量 (每行一个，格式为 KEY=VALUE)</Label>
            <Textarea
              id="env"
              {...register('env')}
              placeholder="例如:&#10;PORT=8080&#10;DEBUG=true"
              rows={4}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="max_instances">最大实例数</Label>
            <Input
              id="max_instances"
              type="number"
              min="1"
              {...register('max_instances')}
            />
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose} disabled={isSubmitting}>
              取消
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              {submitLabel}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
