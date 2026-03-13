import { CheckCircle, Circle } from 'lucide-react';
import Section from '../ui/Section';

interface RoadmapItemProps {
  title: string;
  description: string;
  completed?: boolean;
  date: string;
}

const RoadmapItem = ({ title, description, completed = false, date }: RoadmapItemProps) => {
  return (
    <div className="flex">
      <div className="flex flex-col items-center mr-4">
        <div className="flex-shrink-0">
          {completed ? (
            <CheckCircle className="h-8 w-8 text-green-500" />
          ) : (
            <Circle className="h-8 w-8 text-blue-500" />
          )}
        </div>
        <div className="w-px h-full bg-slate-200 dark:bg-slate-700 my-2"></div>
      </div>
      <div className="pb-8">
        <div className="text-sm font-medium text-slate-500 dark:text-slate-400 mb-1">
          {date}
        </div>
        <h3 className="text-xl font-semibold mb-2">{title}</h3>
        <p className="text-slate-600 dark:text-slate-400">
          {description}
        </p>
      </div>
    </div>
  );
};

const Roadmap = () => {
  return (
    <Section
      id="roadmap"
      title="Roadmap"
      subtitle="Our journey to build the ultimate MCP management center."
      centered
    >
      <div className="max-w-3xl mx-auto mt-12">
        <RoadmapItem
          title="Core Proxy Implementation"
          description="High-performance MCP proxy server with multi-server connection pooling, real-time monitoring, and a RESTful API for management."
          completed={true}
          date="Q1 2025 - Completed"
        />

        <RoadmapItem
          title="Bridge Component"
          description="Lightweight bridge component to connect stdio-based MCP clients to SSE-based MCPMate proxy servers."
          completed={true}
          date="Q2 2025 - Completed"
        />

        <RoadmapItem
          title="MCPMate Desktop - Alpha"
          description="First alpha release of the cross-platform desktop application with basic server management capabilities."
          date="Q3 2025 - In Progress"
        />

        <RoadmapItem
          title="Resource Management"
          description="Advanced resource management features including process monitoring, resource usage tracking, and automatic optimization."
          date="Q4 2025 - Planned"
        />

        <RoadmapItem
          title="AI-Powered Configuration"
          description="Smart configuration management with AI-assisted setup, automated optimization, and intelligent resource allocation."
          date="Q1 2026 - Planned"
        />

        <RoadmapItem
          title="Team Collaboration"
          description="Enterprise features including multi-user support, shared configurations, and team management capabilities."
          date="Q2 2026 - Planned"
        />
      </div>
    </Section>
  );
};

export default Roadmap;