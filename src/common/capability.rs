// Unified capability tokens and helpers

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityToken {
    Tools,
    Prompts,
    Resources,
    ResourceTemplates,
}

impl CapabilityToken {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityToken::Tools => "tools",
            CapabilityToken::Prompts => "prompts",
            CapabilityToken::Resources => "resources",
            CapabilityToken::ResourceTemplates => "resource_templates",
        }
    }
}

impl std::fmt::Display for CapabilityToken {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
