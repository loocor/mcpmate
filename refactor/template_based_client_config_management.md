# 基于模板的 MCP 客户端配置管理方案

## 🎯 设计目标

- **准确性**: 精确支持各种 MCP 客户端的配置格式
- **灵活性**: 能够适应客户端配置格式的变化  
- **全面性**: 尽可能支持更多 MCP 客户端
- **透明性**: 配置文件可读，用户可理解和修改
- **可扩展**: 支持线上更新配置，新增客户端支持对用户透明

## 📋 现状分析

### 发现的问题
1. **配置格式在演进**: Zed 的实际配置格式已与数据库中的规则不符
   - 数据库期望: `command.path` 结构
   - 实际配置: 扁平的 `command` + `args` 结构，还新增了 `source`、`enabled` 字段

2. **数据库模式的局限性**: 
   - 难以快速跟上客户端配置格式变化
   - 新增客户端支持需要修改代码和数据库
   - 配置规则不够透明，难以调试

3. **MCP 配置结构高度一致**:
   - Claude Desktop: `{"mcpServers": {"server_name": {"command": "...", "args": [...], "env": {}}}}`
   - Cursor: 相同结构，多了 `"type": "stdio"` 字段
   - **核心差异在单个 server 节点 `{}` 的字段映射**

## 🏗️ 解决方案：基于文件的精简模板系统

### 核心理念

1. **配置即文件**: 所有客户端配置规则存储为 JSON 文件
2. **聚焦差异**: 只定义客户端特有的差异化配置
3. **标准抽取**: MCP 规范标准部分统一管理，避免重复
4. **线上可更新**: 支持从线上同步最新的客户端配置
5. **业务隔离**: 配置格式变化不影响核心业务逻辑

### 📁 文件结构

```
~/.mcpmate/
├── clients/                          # 客户端配置定义
│   ├── official/                     # 官方维护
│   │   ├── claude_desktop.json
│   │   ├── cursor.json
│   │   ├── zed.json                  # 可以线上更新
│   │   ├── windsurf.json
│   │   └── vscode.json
│   ├── community/                    # 社区贡献
│   │   ├── neovim.json
│   │   └── emacs.json
│   └── user/                         # 用户自定义
│       └── my_custom_editor.json
├── mcp_standards.json               # MCP 规范标准配置
├── registry.json                    # 客户端注册表
└── cache/                           # 运行时缓存
    ├── detected_clients.json
    └── last_update.json
```

## 🔧 配置文件格式

### MCP 规范标准配置 (`mcp_standards.json`)

```json
{
  "version": "1.0.0",
  "mcp_spec_version": "2024.11.05",
  "transports": {
    "stdio": {
      "required_fields": ["command"],
      "optional_fields": ["args", "env", "cwd"],
      "standard_template": {
        "command": "{{command}}",
        "args": "{{args}}",
        "env": "{{env}}"
      }
    },
    "sse": {
      "required_fields": ["url"],
      "optional_fields": ["headers", "timeout"],
      "standard_template": {
        "url": "{{url}}",
        "headers": "{{headers}}"
      }
    },
    "streamable_http": {
      "required_fields": ["url"],
      "optional_fields": ["headers", "timeout"],
      "standard_template": {
        "type": "streamableHttp",
        "url": "{{url}}",
        "headers": "{{headers}}"
      }
    }
  },
  "common_patterns": {
    "server_container_keys": ["mcpServers", "context_servers", "servers", "mcp"],
    "array_indicators": ["servers", "mcpServers"]
  }
}
```

### 客户端配置文件示例

#### Zed 配置 (`zed.json`)
```json
{
  "identifier": "zed",
  "display_name": "Zed",
  "version": "2025.01.15",
  
  "detection": {
    "macos": [
      {
        "method": "file_path", 
        "value": "/Applications/Zed.app", 
        "config_path": "~/.config/zed/settings.json"
      }
    ],
    "linux": [
      {
        "method": "file_path", 
        "value": "~/.local/bin/zed", 
        "config_path": "~/.config/zed/settings.json"
      }
    ]
  },
  
  "config_mapping": {
    "container_key": "context_servers",
    "container_type": "object_map",
    "merge_strategy": "deep_merge",
    
    "server_node_template": {
      "source": "custom",           // Zed 特有字段
      "enabled": true,              // Zed 特有字段  
      "command": "{{command}}",     // 标准字段映射
      "args": "{{args}}",           // 标准字段映射
      "env": "{{env}}"              // 标准字段映射
    }
  }
}
```

#### Cursor 配置 (`cursor.json`)
```json
{
  "identifier": "cursor",
  "display_name": "Cursor", 
  "version": "2025.01.15",
  
  "detection": {
    "macos": [
      {
        "method": "bundle_id", 
        "value": "com.todesktop.230313mzl4w4u92", 
        "config_path": "~/.cursor/mcp.json"
      }
    ]
  },
  
  "config_mapping": {
    "container_key": "mcpServers",
    "container_type": "object_map",
    "merge_strategy": "replace",
    
    "server_node_template": {
      "type": "stdio",              // Cursor 要求显式 type 字段
      "command": "{{command}}",
      "args": "{{args}}",
      "env": "{{env}}"
    }
  }
}
```

#### Augment 配置 (`augment.json`) - 数组模式示例
```json
{
  "identifier": "augment",
  "display_name": "Augment",
  "version": "2025.01.15",
  
  "detection": {
    "macos": [
      {
        "method": "bundle_id", 
        "value": "com.augmentcode.app", 
        "config_path": "~/Library/Application Support/Augment/servers.json"
      }
    ]
  },
  
  "config_mapping": {
    "container_key": null,          // 根级别就是数组
    "container_type": "array",
    "merge_strategy": "replace",
    
    "server_node_template": {
      "name": "{{name}}",           // 数组模式需要 name 字段
      "command": "{{command}}",
      "args": "{{args}}",
      "env": "{{env}}"
    }
  }
}
```

### 客户端注册表 (`registry.json`)
```json
{
  "version": "1.0.0",
  "last_sync": "2025-01-15T10:30:00Z",
  "remote_registry": "https://registry.mcpmate.com/clients/",
  "clients": {
    "claude_desktop": {
      "local_version": "2025.01.10",
      "remote_version": "2025.01.15", 
      "auto_update": true,
      "source": "official"
    },
    "zed": {
      "local_version": "2025.01.15",
      "remote_version": "2025.01.15",
      "auto_update": true,
      "source": "official"  
    },
    "my_custom_editor": {
      "local_version": "1.0.0",
      "source": "user"
    }
  }
}
```

## 💻 核心架构

### 配置引擎核心逻辑

```rust
// src/config/client/engine.rs
pub struct ConfigEngine {
    mcp_standards: McpStandards,
    client_configs: HashMap<String, ClientConfig>,
}

impl ConfigEngine {
    // 为单个客户端生成配置
    pub fn generate_config_for_client(
        &self,
        client_id: &str, 
        servers: &[Server]
    ) -> Result<serde_json::Value> {
        let client_config = &self.client_configs[client_id];
        let mapping = &client_config.config_mapping;
        
        match mapping.container_type.as_str() {
            "object_map" => self.generate_object_map_config(servers, mapping),
            "array" => self.generate_array_config(servers, mapping),
            _ => Err(anyhow::anyhow!("Unsupported container type"))
        }
    }
    
    // 渲染单个服务器节点 - 核心逻辑
    fn render_server_node(
        &self,
        server: &Server,
        template: &serde_json::Value
    ) -> Result<serde_json::Value> {
        let context = self.create_server_context(server)?;
        
        // 简单的模板替换
        let template_str = serde_json::to_string(template)?;
        let rendered_str = self.replace_template_variables(&template_str, &context)?;
        let rendered_value: serde_json::Value = serde_json::from_str(&rendered_str)?;
        
        Ok(rendered_value)
    }
}
```

### 数据结构定义

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub identifier: String,
    pub display_name: String,
    pub version: String,
    pub detection: HashMap<String, Vec<DetectionRule>>,
    pub config_mapping: ConfigMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]  
pub struct ConfigMapping {
    pub container_key: Option<String>,      // "mcpServers" | "context_servers" | null
    pub container_type: String,             // "object_map" | "array"
    pub merge_strategy: String,             // "replace" | "deep_merge"
    pub server_node_template: serde_json::Value, // 单个服务器节点的模板
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStandards {
    pub version: String,
    pub transports: HashMap<String, TransportSpec>,
    pub common_patterns: CommonPatterns,
}
```

## 🚀 用户体验

### 完全透明的新增支持
```bash
# 用户无需任何操作，MCPMate 在后台：
# 1. 定期检查线上配置更新
# 2. 发现新客户端配置时自动下载
# 3. 下次检测时自动支持新客户端

$ mcpmate configure
✅ 检测到 Claude Desktop
✅ 检测到 Cursor  
✅ 检测到 Zed
✅ 检测到 VS Code Insiders (新增支持！)
✅ 检测到 Neovim (社区贡献！)

配置完成！5个客户端已更新MCP配置。
```

### 配置格式变化的透明处理
当 Zed 更新配置格式时，只需要更新几行 JSON：
```json
{
  "server_node_template": {
    "source": "custom",        // 如果新增这个字段
    "enabled": true,          // 如果新增这个字段  
    "command": "{{command}}", // 标准字段保持不变
    "args": "{{args}}",       // 标准字段保持不变
    "env": "{{env}}"          // 标准字段保持不变
  }
}
```

## 📊 方案优势

1. **极简配置**: 每个客户端配置文件只有几十行，只关注差异
2. **标准抽取**: MCP 规范部分统一管理，避免重复
3. **聚焦核心**: 重点解决单个 server 节点的字段映射问题
4. **易于维护**: 新增客户端或格式变化只需修改小文件
5. **规范驱动**: 基于 MCP 标准，易于跟随规范演进
6. **透明度高**: 所有配置都是可读的 JSON 文件
7. **隔离变化**: 配置格式变化只影响配置文件，不影响代码
8. **线上可更新**: 支持自动同步最新的客户端支持
9. **社区友好**: 支持社区贡献新客户端配置
10. **用户透明**: 新增支持和格式更新对用户完全透明

## 🎯 实施步骤

1. **第一阶段**: 实现基于文件的配置管理核心架构
2. **第二阶段**: 从现有数据库配置迁移到 JSON 文件系统
3. **第三阶段**: 实现线上同步机制
4. **第四阶段**: 建立配置文件的持续更新机制

## 📝 后续细节完善

- [ ] 完善 streamable_http 传输方式的处理
- [ ] 设计配置文件的版本兼容性机制
- [ ] 实现智能配置合并算法
- [ ] 添加配置文件验证和错误处理
- [ ] 设计线上配置文件的分发和更新机制

---

**设计日期**: 2025-01-15  
**设计者**: 老陈 & 超超  
**状态**: 方案确定，待实施