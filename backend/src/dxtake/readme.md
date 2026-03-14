# MCPB Compatibility (DXT → MCPB)

## Overview

This module note has been superseded by the consolidated feature document. The DXT naming has evolved into MCPB (MCP Bundles).

Please see:

- docs/features/server_sources_and_artifacts.md — unified model for sources and artifacts (MCPB + local + package)
- docs/roadmap/mcpb_support.md — plan and milestones

Historical context and prior analysis remain below for reference.

## Value Summary

### What DXT Adds Directly
DXT adds **packaged distribution** alongside manual copy-paste of JSON, without breaking existing MCPMate design. We only need a parallel module similar to the web MCP Server config JSON parser.

### DXT as Reference
1. **Database design**: Improves server registry and tables such as `server_args`, `server_config`, `server_env`, `server_meta`, `server_tools`.
2. **Variable abstraction**: Standard reference for handling generic variables.
3. **GUI patterns**: Design guidance for adding GUI for MCP Server configuration.

### DXT as Extension
1. **Validation of improvement space**: Confirms room for improvement in client (desktop) JSON-based MCP Server config distribution.
2. **Security exposure**: Highlights security shortcomings of traditional mcp.json-style config.
3. **Enterprise reference**: DXT’s enterprise story informs how MCPMate can add value in similar scenarios.
4. **Future direction**: Concrete options for future feature completeness.

### DXT Spec Summary

Desktop Extensions (`.dxt`) are Anthropic’s MCP server extension format, similar to Chrome or VS Code extensions:

- **One-click install**: `.dxt` bundles the MCP server and all dependencies.
- **Standard config**: `manifest.json` defines server metadata and user config.
- **Multi-runtime**: Node.js, Python, and Binary server types.
- **Security**: PKCS#7 signatures for integrity.
- **User-friendly**: GUI config; no manual JSON editing.

## Application Value

### For users
- **Zero-config**: From complex JSON to drag-and-drop install.
- **Rich ecosystem**: Access MCP servers from DXT extension stores.
- **Unified management**: DXT extensions and traditional MCP servers in one UI.
- **Security**: Signature verification and permission management.

### For developers
- **Full toolchain**: Develop, test, package, and distribute in one flow.
- **Standard distribution**: Single packaging and distribution format.
- **Simpler deployment**: No manual dependency or config handling for users.
- **Broader audience**: Lower barrier, larger user base.

### For MCPMate
- **Technical agility**: Early DXT support and fast iteration.
- **Differentiation**: Best-in-class DXT management experience.
- **Developer ecosystem**: Stronger position in MCP developer tooling.
- **Cross-application**: One solution for multiple desktop AI apps’ MCP server management.

## Core Features

### 1. DXT Import and Management

#### Drag-and-drop import
- Drag `.dxt` files into MCPMate.
- Parse and validate `manifest.json`.
- Show install preview with extracted metadata.

#### Extension management
- Install, enable, disable, uninstall DXT extensions.
- Version management and update checks.
- Same UI as existing server management.

#### Compatibility checks
- Platform (macOS, Windows, Linux).
- Runtime dependencies (Python, Node.js versions).
- Compatibility warnings and suggestions.

### 2. Dynamic Config UI

#### user_config-driven UI
Generate UI from DXT `manifest.json` `user_config`:

```json
{
  "user_config": {
    "api_key": {
      "type": "string",
      "title": "API Key",
      "description": "Your API key for authentication",
      "sensitive": true,
      "required": true
    },
    "allowed_directories": {
      "type": "directory",
      "title": "Allowed Directories",
      "description": "Directories the server can access",
      "multiple": true,
      "required": true
    }
  }
}
```

#### Supported config types
- **string**: Text input; optional masking for sensitive values.
- **number**: Numeric input; min/max validation.
- **boolean**: Toggle.
- **directory**: Directory picker; optional multiple.
- **file**: File picker; optional multiple.

#### Validation and storage
- Live validation and error messages.
- Encrypted storage for sensitive fields.
- Config changes take effect immediately.

### 3. Sensitive Information Management

#### Standardized config system
Align with DXT config patterns and refactor MCPMate sensitive-data handling:

- **Unified template**: Standard field definitions.
- **Secure storage**: Encrypted storage and access control.
- **Variable substitution**: Single substitution mechanism.
- **Inheritance**: Global and per-extension config.

#### Variable substitution
DXT-style variables:
- `${__dirname}`: Extension install directory.
- `${HOME}`, `${DESKTOP}`, `${DOCUMENTS}`: System paths.
- `${user_config.key}`: User config values.

### 4. Developer Toolchain

#### Export to DXT
Export existing MCP servers as DXT using current runtime management and official DXT CLI.

**Strategy**:
- **Reuse**: MCPMate bun runtime management.
- **Official tool**: `@anthropic-ai/dxt` CLI for packing.
- **Focus**: manifest.json generation and config mapping.

**Flow**:
1. Prepare working directory; copy server files.
2. Generate DXT manifest from MCPMate config.
3. Handle dependencies (Python/Node.js).
4. Map API keys etc. to user_config.
5. Run `bun x @anthropic-ai/dxt pack` for final package.

```rust
pub struct DxtExporter {
    runtime_manager: Arc<RuntimeManager>, // reuse existing runtime management
}

impl DxtExporter {
    pub async fn export_to_dxt(&self, server_id: &str, output_path: &Path) -> Result<()> {
        // 1. Prepare working directory and files
        let temp_dir = self.prepare_export_directory(server_id).await?;

        // 2. Generate manifest.json (core work)
        self.generate_manifest(&temp_dir, server_id).await?;

        // 3. Call official DXT CLI
        let command = format!(
            "bun x @anthropic-ai/dxt pack {} {}",
            temp_dir.display(),
            output_path.display()
        );
        self.runtime_manager.execute_command(&command).await?;

        Ok(())
    }
}
```

#### Testing and validation
- Pre-export integrity checks.
- `dxt validate` for format.
- Compatibility checks and warnings.

## Technical Architecture

### Runtime Deployment

#### DXT requirements
Extensions must be self-contained:

- **Python**: Dependencies in `server/lib` or full `server/venv`.
- **Node.js**: Full `node_modules`.
- **Binary**: Prebuilt executable with dependencies.

**Duplicate deployment**: Multiple extensions can ship the same deps (storage cost). Rationale:
- **Self-contained**: Runs in any environment.
- **Version isolation**: No cross-extension conflicts.
- **Simple install**: No pre-installed runtime required.

#### MCPMate hybrid strategy
MCPMate supports flexible runtime strategies:

```rust
pub enum ExtensionRuntime {
    /// Standard DXT: use extension’s own runtime (full compatibility)
    SelfContained {
        runtime_path: PathBuf,
    },
    /// MCPMate-optimized: shared runtime (save space)
    Shared {
        runtime_type: RuntimeType,
        version_requirement: String,
    },
    /// Hybrid: prefer shared, fallback to self-contained
    Hybrid {
        preferred: Box<ExtensionRuntime>,
        fallback: Box<ExtensionRuntime>,
    },
}
```

**User choice**:
- **Default**: Follow DXT; self-contained runtime.
- **Optimized**: Advanced users can use MCPMate shared runtime.
- **Auto**: Detect whether extension is compatible with shared runtime.

### Backend Implementation

#### DXT parser
```rust
pub struct DxtManager {
    db: Arc<Database>,
    install_dir: PathBuf,
    runtime_manager: Arc<RuntimeManager>,
}

impl DxtManager {
    /// Import DXT file
    pub async fn import_dxt_file(&self, file_path: &Path) -> Result<Extension> {
        // 1. Unpack .dxt
        // 2. Parse manifest.json
        // 3. Validate format and compatibility
        // 4. Choose runtime strategy
        // 5. Install to target dir
        // 6. Register in DB
    }

    /// Configure extension
    pub async fn configure_extension(&self, ext_id: &str, config: UserConfig) -> Result<()> {
        // 1. Validate config
        // 2. Encrypt and store sensitive values
        // 3. Build runtime config
        // 4. Select runtime strategy
        // 5. Start extension service
    }
}
```

#### Unified config system
Refactor MCPMate config around DXT User Configuration:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTemplate {
    pub field_type: ConfigFieldType,
    pub title: String,
    pub description: String,
    pub required: bool,
    pub sensitive: bool,
    pub default_value: Option<ConfigValue>,
    pub validation: Option<ValidationRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigFieldType {
    String { sensitive: bool },
    Number { min: Option<f64>, max: Option<f64> },
    Boolean,
    Directory { multiple: bool },
    File { multiple: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<String>),
}

/// Variable substitution engine; supports DXT variable syntax
pub struct VariableSubstitution {
    system_vars: HashMap<String, String>,
    user_configs: HashMap<String, ConfigValue>,
}

impl VariableSubstitution {
    pub fn new() -> Self {
        let mut system_vars = HashMap::new();
        system_vars.insert("HOME".to_string(), dirs::home_dir().unwrap().to_string_lossy().to_string());
        system_vars.insert("DESKTOP".to_string(), dirs::desktop_dir().unwrap().to_string_lossy().to_string());
        system_vars.insert("DOCUMENTS".to_string(), dirs::document_dir().unwrap().to_string_lossy().to_string());
        system_vars.insert("DOWNLOADS".to_string(), dirs::download_dir().unwrap().to_string_lossy().to_string());
        system_vars.insert("/".to_string(), std::path::MAIN_SEPARATOR.to_string());

        Self { system_vars, user_configs: HashMap::new() }
    }

    /// DXT variable syntax: ${HOME}, ${user_config.key}, ${__dirname}, etc.
    pub fn substitute(&self, template: &str, extension_dir: Option<&Path>) -> String {
        // full DXT variable substitution logic
    }
}
```

#### DXT exporter (simplified)
```rust
pub struct DxtExporter {
    runtime_manager: Arc<RuntimeManager>, // reuse existing runtime management
    config_manager: Arc<ConfigManager>,
}

impl DxtExporter {
    /// Export server as DXT using official CLI
    pub async fn export_to_dxt(&self, server_id: &str, output_path: &Path) -> Result<()> {
        // 1. Create temp working dir
        let temp_dir = self.create_temp_directory()?;

        // 2. Prepare server files and deps
        self.prepare_server_files(&temp_dir, server_id).await?;

        // 3. Generate manifest.json (core work)
        self.generate_manifest(&temp_dir, server_id).await?;

        // 4. Call official @anthropic-ai/dxt CLI
        self.call_dxt_cli_pack(&temp_dir, output_path).await?;

        // 5. Cleanup temp
        self.cleanup_temp_directory(&temp_dir)?;

        Ok(())
    }

    async fn call_dxt_cli_pack(&self, source_dir: &Path, output_path: &Path) -> Result<()> {
        let command = format!(
            "bun x @anthropic-ai/dxt pack {} {}",
            source_dir.display(),
            output_path.display()
        );

        // use existing runtime management
        self.runtime_manager.execute_command(&command).await
    }

    /// Generate DXT-compliant manifest.json
    async fn generate_manifest(&self, temp_dir: &Path, server_id: &str) -> Result<()> {
        let server_config = self.config_manager.get_server_config(server_id).await?;

        // Map MCPMate config to DXT manifest
        let manifest = DxtManifest {
            dxt_version: "0.1".to_string(),
            name: server_config.name.clone(),
            version: server_config.version.unwrap_or("1.0.0".to_string()),
            description: server_config.description.clone(),
            author: AuthorInfo {
                name: "MCPMate User".to_string(),
                email: None,
                url: None,
            },
            server: ServerConfig {
                server_type: self.detect_server_type(&server_config)?,
                entry_point: self.prepare_entry_point(&server_config)?,
                mcp_config: self.generate_mcp_config(&server_config)?,
            },
            user_config: self.extract_user_config(&server_config)?,
            // ... other fields
        };

        // write manifest.json
        let manifest_path = temp_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        tokio::fs::write(manifest_path, manifest_json).await?;

        Ok(())
    }
}
```

### Database Design

#### DXT extensions table
```sql
CREATE TABLE dxt_extensions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    display_name TEXT,
    version TEXT NOT NULL,
    author_name TEXT NOT NULL,
    description TEXT,
    manifest_json TEXT NOT NULL,
    install_path TEXT NOT NULL,
    installed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    enabled BOOLEAN DEFAULT TRUE,
    signature_verified BOOLEAN DEFAULT FALSE
);
```

#### DXT extension config table
```sql
CREATE TABLE dxt_extension_configs (
    id TEXT PRIMARY KEY,
    extension_id TEXT NOT NULL,
    config_key TEXT NOT NULL,
    config_value TEXT,
    is_sensitive BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (extension_id) REFERENCES dxt_extensions(id) ON DELETE CASCADE,
    UNIQUE(extension_id, config_key)
);
```

### Frontend

#### React components (Tauri Dashboard)
```tsx
// DXT extension management
import { useQuery, useMutation } from '@tanstack/react-query';

function DxtExtensionsView() {
  const { data: extensions } = useQuery({
    queryKey: ['dxt', 'extensions'],
    queryFn: dxtApi.getExtensions,
  });

  const importMutation = useMutation({
    mutationFn: dxtApi.importDxt,
    onSuccess: () => queryClient.invalidateQueries(['dxt', 'extensions']),
  });

  return (
    <div
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => handleDxtFileDrop(e, importMutation)}
    >
      {extensions?.map((ext) => (
        <DxtExtensionRow key={ext.id} extension={ext} />
      ))}
    </div>
  );
}

// Dynamic config form
function DxtConfigurationView({ extension }: { extension: DxtExtension }) {
  const [configValues, setConfigValues] = useState<Record<string, unknown>>({});

  return (
    <form>
      {extension.userConfigFields.map((field) => (
        <DxtConfigField
          key={field.key}
          field={field}
          value={configValues[field.key]}
          onChange={(value) => setConfigValues((v) => ({ ...v, [field.key]: value }))}
        />
      ))}
    </form>
  );
}
```

#### Tauri Integration
```rust
// DXT management via Tauri commands
#[tauri::command]
async fn import_dxt_file(path: String, manager: State<'_, DxtManager>) -> Result<Extension, String> {
    manager.import_dxt_file(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn configure_dxt_extension(
    extension_id: String,
    config: UserConfig,
    manager: State<'_, DxtManager>,
) -> Result<(), String> {
    manager.configure_extension(&extension_id, config).await.map_err(|e| e.to_string())
}
        guard let engine = rustEngine else { return false }
        return mcpmate_engine_configure_dxt(engine, extensionId, config)
    }

    /// Export to DXT
    public func exportToDxt(serverId: String, outputPath: String) -> Bool {
        guard let engine = rustEngine else { return false }
        return mcpmate_engine_export_dxt(engine, serverId, outputPath)
    }
}
```

## API

### DXT management

#### Extensions
```
GET    /api/dxt/extensions          # List installed extensions
POST   /api/dxt/extensions/import   # Import DXT file
DELETE /api/dxt/extensions/{id}     # Uninstall
PUT    /api/dxt/extensions/{id}/enable   # Enable
PUT    /api/dxt/extensions/{id}/disable  # Disable
```

#### Config
```
GET    /api/dxt/extensions/{id}/config        # Get config
PUT    /api/dxt/extensions/{id}/config        # Update config
GET    /api/dxt/extensions/{id}/config/schema # Get config schema
```

#### Developer
```
POST   /api/dxt/export/{server_id}  # Export server as DXT
GET    /api/dxt/export/{job_id}     # Export job status
```

## User Configuration Spec

### Field types (DXT MANIFEST.md)

#### Basic types
```json
{
  "api_key": {
    "type": "string",
    "title": "API Key",
    "description": "Your API key for authentication",
    "sensitive": true,
    "required": true
  },
  "max_file_size": {
    "type": "number",
    "title": "Maximum File Size (MB)",
    "description": "Maximum file size to process",
    "default": 10,
    "min": 1,
    "max": 100
  },
  "read_only": {
    "type": "boolean",
    "title": "Read Only Mode",
    "description": "Open database in read-only mode",
    "default": true
  }
}
```

#### File system types
```json
{
  "allowed_directories": {
    "type": "directory",
    "title": "Allowed Directories",
    "description": "Directories the server can access",
    "multiple": true,
    "required": true,
    "default": ["${HOME}/Desktop", "${HOME}/Documents"]
  },
  "database_path": {
    "type": "file",
    "title": "Database File",
    "description": "Path to your SQLite database file",
    "required": true
  }
}
```

### Variable substitution

#### System
- `${HOME}`: User home
- `${DESKTOP}`: Desktop
- `${DOCUMENTS}`: Documents
- `${DOWNLOADS}`: Downloads
- `${/}` or `${pathSeparator}`: Path separator

#### Extension
- `${__dirname}`: Extension install dir
- `${user_config.key}`: User config value

#### Array expansion
When a field has `multiple: true`, it is expanded in `args`:
```json
// User selection: ["/home/user/docs", "/home/user/projects"]
"args": ["${user_config.allowed_directories}"]
// Expands to: ["/home/user/docs", "/home/user/projects"]
```

### MCPMate config refactor
These rules drive the refactor:

1. **Unified model**: Same field definition shape.
2. **Sensitive**: Secure storage for `sensitive: true`.
3. **Validation**: `min`/`max`, `required`, etc.
4. **Defaults**: With variable substitution.
5. **Multiple**: `multiple: true` for file/directory.

## Implementation Plan

### Phase 1: Unified config refactor (top priority, ~1 month)
- [ ] Refactor config field definitions per DXT User Configuration.
- [ ] Variable substitution engine (DXT variable syntax).
- [ ] Secure storage and handling of sensitive config.
- [ ] Validation and default values.
- [ ] Unified config UI generation.

### Phase 2: DXT import (~1 month)
- [ ] DXT parse and validate.
- [ ] Dynamic UI from unified config.
- [ ] Runtime strategy (self-contained vs shared).
- [ ] Integrate with existing server management.

### Phase 3: Export to DXT (~1 month)
- [ ] manifest.json generation.
- [ ] Config extraction and mapping (MCPMate → DXT).
- [ ] Integrate @anthropic-ai/dxt CLI.
- [ ] Pre-export validation and tests.

### Phase 4: UX (~2 months)
- [ ] React/Tauri extension management UI.
- [ ] Drag-and-drop import.
- [ ] Runtime strategy UI.
- [ ] Extension status and logs.

### Phase 5: Advanced (~3 months)
- [ ] Signature verification (`dxt verify`).
- [ ] Extension updates.
- [ ] Permissions.
- [ ] Performance and security hardening.

## Compatibility and Security

### Platforms
- **macOS**: Full support; first target.
- **Windows**: Planned.
- **Linux**: Planned.

### Security
- **Signatures**: PKCS#7 verification.
- **Sandbox**: Process isolation and permissions.
- **Sensitive data**: Encrypted storage and transport.
- **Permissions**: Fine-grained filesystem and network control.

### Backward compatibility
- Existing traditional MCP server config unchanged.
- Smart parsing, drag config, etc. still supported.
- Config can mix DXT extensions and traditional servers.

## Key Decisions

### 1. Runtime deployment
**Issue**: DXT requires self-contained deps → duplicate deployment.
**Approach**: MCPMate hybrid strategy: compatibility + storage optimization.

### 2. Export to DXT
**Decision**: Use official @anthropic-ai/dxt CLI instead of reimplementing.
**Benefits**: Less code, compatibility, lower maintenance.

### 3. Config refactor
**Principle**: Follow DXT User Configuration strictly.
**Value**: Standardized config foundation for MCPMate.

## Research Notes

### DXT behavior

#### Post-install handling
From [DXT source](https://github.com/anthropics/dxt/blob/main/src/index.ts):

1. **Full unpack**: All content (node_modules, Python venv) unpacked to a dedicated install dir.
2. **Absolute paths**: `${__dirname}` replaced at install time; final mcp.json uses absolute paths.
3. **Isolation**: Each extension runs in its own directory.

#### Install directories
Not mandated by spec; app-defined. Suggested:
```
macOS:    ~/Library/Application Support/MCPMate/Extensions/
Windows:  %APPDATA%/MCPMate/Extensions/
Linux:    ~/.config/MCPMate/Extensions/
```

#### Updates
- **Store extensions**: Auto-update.
- **Private**: Manual update.
- **Check**: Via extension registry version compare.

#### Variable substitution timing
All variables (including sensitive `${user_config.api_key}`) are replaced **at install time** and stored in plaintext in client config. This differs from runtime resolution and has security implications.

### Enterprise security

#### DXT packaging vs npx cache scan
DXT’s bundled deps enable stronger security than npx cache scanning:

1. **Static analysis**: Full pre-install scan of deps.
2. **Supply chain**: Locked versions; less drift.
3. **Policy**: Dependency whitelist, vulnerability scan, license compliance.
4. **Audit**: Full install and usage trail.

#### Hybrid security
- **DXT**: Static dependency scan (pre-install).
- **Traditional MCP**: Runtime cache scan.
- **Unified**: Same policy and whitelist for both.

### Sensitive data

#### MCPMate improvements over DXT
DXT’s install-time plaintext substitution is risky. MCPMate can improve:

1. **Runtime vs install-time**:
   - DXT: install-time replacement, plaintext storage.
   - MCPMate: runtime resolution, encrypted storage.

2. **Mixed strategy**:
   - Non-sensitive: install-time (DXT-compatible).
   - Sensitive: runtime injection (MCPMate enhancement).

3. **Enterprise key management**:
   - Encrypted sensitive config.
   - Integration with enterprise KMS.
   - Fine-grained access and audit.

## Summary

DXT compatibility makes MCPMate a fast, user-friendly MCP server manager by:

1. **Early DXT support**: Technical agility.
2. **Smart runtime**: Address duplicate deployment; offer optimization.
3. **Unified config**: DXT-based refactor; better product quality.
4. **Simpler implementation**: Official tools; focus on core value.
5. **Enterprise security**: Stronger sensitive-data handling than default DXT.

Beyond DXT support, the User Configuration spec gives MCPMate a standardized config base and distinct enterprise security value.

## Enterprise Security Architecture

### Layered security
```rust
// Reference: src/dxtake/security/
pub struct EnterpriseSecurityManager {
    // Static scan for DXT extensions
    dxt_analyzer: Arc<DxtSecurityAnalyzer>,

    // Runtime scan for traditional MCP servers
    runtime_scanner: Arc<RuntimeSecurityScanner>,

    // Unified policy engine
    policy_engine: Arc<SecurityPolicyEngine>,

    // Approval workflow
    approval_workflow: Arc<ApprovalWorkflow>,
}
```

### Enterprise policy
```rust
// Reference: src/dxtake/enterprise/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseSecurityPolicy {
    // Dependency whitelist
    pub dependency_whitelist: DependencyWhitelistPolicy,

    // Vulnerability tolerance
    pub vulnerability_tolerance: VulnerabilityTolerance,

    // License policy
    pub license_policy: LicensePolicy,

    // Approval requirements
    pub approval_requirements: ApprovalRequirements,

    // Monitoring and audit
    pub monitoring_config: MonitoringConfig,
}
```

### Business value
- **Enterprise demand**: Strict compliance in finance, healthcare, government.
- **Differentiation**: First enterprise-grade MCP security tool.
- **Coverage**: Unified approach for DXT and traditional MCP.

## Implementation Guide

### Module layout
```
src/dxtake/
├── mod.rs                    # Module entry
├── manager.rs               # DXT extension manager
├── parser.rs                # DXT file parsing
├── installer.rs             # Extension installer
├── exporter.rs              # Export to DXT
├── config/
│   ├── mod.rs
│   ├── template.rs          # Config template system
│   ├── variables.rs         # Variable substitution
│   └── validation.rs       # Config validation
├── security/
│   ├── mod.rs
│   ├── analyzer.rs          # Security analyzer
│   ├── scanner.rs           # Dependency scanner
│   └── policy.rs            # Security policy
├── enterprise/
│   ├── mod.rs
│   ├── approval.rs          # Approval workflow
│   ├── audit.rs             # Audit log
│   └── integration.rs       # Enterprise integration
└── runtime/
    ├── mod.rs
    ├── strategy.rs          # Runtime strategy
    └── isolation.rs         # Process isolation
```

### DB extension
```sql
-- Reference: src/config/database.rs
-- DXT extensions
CREATE TABLE dxt_extensions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    install_path TEXT NOT NULL,
    manifest_json TEXT NOT NULL,
    security_report TEXT,
    enterprise_approved BOOLEAN DEFAULT FALSE,
    installed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Enterprise security policies
CREATE TABLE enterprise_security_policies (
    id TEXT PRIMARY KEY,
    policy_name TEXT NOT NULL,
    policy_config TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### API layout
```
# Reference: src/api/routes/
/api/dxt/
├── extensions/              # Extension management
├── security/                # Security scanning
├── enterprise/              # Enterprise features
└── export/                  # Export
```

## References

### Official
- [DXT spec](https://github.com/anthropics/dxt)
- [DXT manifest](https://github.com/anthropics/dxt/blob/main/MANIFEST.md)
- [DXT CLI](https://github.com/anthropics/dxt/blob/main/CLI.md)
- [Claude Desktop implementation](https://github.com/anthropics/dxt/blob/main/src/index.ts)

### Enterprise
- [Claude Desktop enterprise policy](https://support.anthropic.com/en/articles/10949351-getting-started-with-model-context-protocol-mcp-on-claude-for-desktop)
- [DXT security best practices](https://www.anthropic.com/engineering/desktop-extensions)

### MCPMate modules
- Config: `src/config/`
- Runtime: `src/runtime/`
- Audit: `src/audit/`
- Protocol: `src/core/protocol/`
