export interface WorkflowGuideCapabilityBlock {
  name: string;
  exposure: "direct" | "meta_on_demand";
  guide: string;
  startLine: number;
  endLine: number;
}

export interface WorkflowGuideParseResult {
  headings: Array<{ level: number; text: string; offset: number }>;
  capabilities: WorkflowGuideCapabilityBlock[];
  errors: string[];
}

export interface WorkflowGuideDocumentCell {
  id: string;
  kind: "markdown" | "external_reference" | "capability";
  source: string;
  startOffset: number;
  endOffset: number;
  capability?: WorkflowGuideCapabilityBlock;
  externalReference?: { title: string; relativePath: string };
}

const CAPABILITY_START = /^:::capability\s+(\{.*\})\s*$/;
const DIRECTIVE_END = /^:::\s*$/;
const UUID_REFERENCE = /\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b/i;

export function parseWorkflowGuide(markdown: string): WorkflowGuideParseResult {
  const headings: Array<{ level: number; text: string; offset: number }> = [];
  const capabilities: WorkflowGuideCapabilityBlock[] = [];
  const errors: string[] = [];
  const lines = markdown.split("\n");
  const lineOffsets: number[] = [];
  let nextOffset = 0;
  for (const line of lines) {
    lineOffsets.push(nextOffset);
    nextOffset += line.length + 1;
  }
  let fenced = false;
  let active: {
    name: string;
    exposure: "direct" | "meta_on_demand";
    startLine: number;
    lines: string[];
  } | null = null;

  lines.forEach((line, index) => {
    const lineNumber = index + 1;
    if (isFenceMarker(line)) {
      fenced = !fenced;
      if (active) active.lines.push(line);
      return;
    }
    if (fenced) {
      if (containsReservedWorkflowGuideSyntax(line)) {
        errors.push(`line ${lineNumber}: Workflow Guide directives and references are not allowed in fenced code`);
      }
      if (active) active.lines.push(line);
      return;
    }

    if (!active) {
      const heading = /^(#{1,6})\s+(.+?)\s*$/.exec(line);
      if (heading) {
        headings.push({
          level: heading[1].length,
          text: heading[2],
          offset: lineOffsets[index],
        });
      }
      const start = CAPABILITY_START.exec(line);
      if (start) {
        try {
          const header = JSON.parse(start[1]) as { name?: unknown; exposure?: unknown };
          if (typeof header.name !== "string" || !header.name.trim()) {
            errors.push(`line ${lineNumber}: Capability name must not be empty`);
          } else if (header.exposure !== "direct" && header.exposure !== "meta_on_demand") {
            errors.push(`line ${lineNumber}: Capability exposure must be direct or meta_on_demand`);
          } else {
            active = {
              name: header.name,
              exposure: header.exposure,
              startLine: lineNumber,
              lines: [],
            };
          }
        } catch (error) {
          errors.push(
            `line ${lineNumber}: invalid Capability directive: ${error instanceof Error ? error.message : String(error)}`,
          );
        }
      } else if (line.trimStart().startsWith(":::capability")) {
        errors.push(`line ${lineNumber}: invalid Capability directive; expected JSON name and exposure`);
      } else if (DIRECTIVE_END.test(line)) {
        errors.push(`line ${lineNumber}: Capability directive end has no matching start`);
      }
      return;
    }

    if (DIRECTIVE_END.test(line)) {
      capabilities.push({
        name: active.name,
        exposure: active.exposure,
        guide: active.lines.join("\n").trim(),
        startLine: active.startLine,
        endLine: lineNumber,
      });
      active = null;
      return;
    }
    active.lines.push(line);
  });

  if (active) errors.push(`line ${active.startLine}: Capability '${active.name}' directive is not closed`);
  lines.forEach((line, index) => {
    if (UUID_REFERENCE.test(line)) errors.push(`line ${index + 1}: opaque identifiers are not allowed in a Workflow Guide`);
    if (line.includes("skill://")) errors.push(`line ${index + 1}: skill:// references are not allowed in a Workflow Guide`);
  });
  return { headings, capabilities, errors };
}

export function renderWorkflowSkill(markdown: string) {
  const parsed = parseWorkflowGuide(markdown);
  if (parsed.errors.length > 0) return { markdown: "", errors: parsed.errors };

  const capabilitiesByStartLine = new Map(
    parsed.capabilities.map((capability) => [capability.startLine, capability]),
  );
  const lines = markdown.split("\n");
  const output: string[] = [];

  for (let index = 0; index < lines.length; index += 1) {
    const lineNumber = index + 1;
    const capability = capabilitiesByStartLine.get(lineNumber);
    if (capability) {
      output.push(renderCapability(capability));
      index = capability.endLine - 1;
      continue;
    }
    output.push(lines[index]);
  }

  return { markdown: output.join("\n").replace(/\n{3,}/g, "\n\n").trim(), errors: [] };
}

export function splitWorkflowGuideDocument(
  markdown: string,
  sourcePath = "SKILL.md",
): WorkflowGuideDocumentCell[] {
  const parsed = parseWorkflowGuide(markdown);
  const lineOffsets = [0];
  for (let index = 0; index < markdown.length; index += 1) {
    if (markdown[index] === "\n") lineOffsets.push(index + 1);
  }

  const cells: WorkflowGuideDocumentCell[] = [];
  let cursor = 0;
  for (const capability of parsed.capabilities) {
    const startOffset = lineOffsets[capability.startLine - 1] ?? markdown.length;
    const endOffset = lineOffsets[capability.endLine] ?? markdown.length;
    if (cursor < startOffset) {
      cells.push({
        id: `markdown-${cursor}`,
        kind: "markdown",
        source: markdown.slice(cursor, startOffset),
        startOffset: cursor,
        endOffset: startOffset,
      });
    }
    cells.push({
      id: `capability-${startOffset}`,
      kind: "capability",
      source: markdown.slice(startOffset, endOffset),
      startOffset,
      endOffset,
      capability,
    });
    cursor = endOffset;
  }
  if (cursor < markdown.length || cells.length === 0) {
    cells.push({
      id: `markdown-${cursor}`,
      kind: "markdown",
      source: markdown.slice(cursor),
      startOffset: cursor,
      endOffset: markdown.length,
    });
  }
  return cells
    .flatMap(splitMarkdownHeadings)
    .flatMap((cell) => splitExternalMarkdownReferences(cell, sourcePath));
}

function splitMarkdownHeadings(cell: WorkflowGuideDocumentCell): WorkflowGuideDocumentCell[] {
  if (cell.kind !== "markdown") return [cell];
  const lines = cell.source.match(/[^\n]*\n|[^\n]+/g) ?? [];
  const result: WorkflowGuideDocumentCell[] = [];
  let offset = cell.startOffset;
  let segmentStart = offset;
  for (const line of lines) {
    if (/^(#{1,6})\s+.+?\s*$/.test(line) && segmentStart < offset) {
      result.push({
        ...cell,
        id: `markdown-${segmentStart}`,
        source: cell.source.slice(segmentStart - cell.startOffset, offset - cell.startOffset),
        startOffset: segmentStart,
        endOffset: offset,
      });
      segmentStart = offset;
    }
    offset += line.length;
  }
  if (segmentStart < cell.endOffset || result.length === 0) {
    result.push({
      ...cell,
      id: `markdown-${segmentStart}`,
      source: cell.source.slice(segmentStart - cell.startOffset),
      startOffset: segmentStart,
      endOffset: cell.endOffset,
    });
  }
  return result;
}

function splitExternalMarkdownReferences(
  cell: WorkflowGuideDocumentCell,
  sourcePath: string,
): WorkflowGuideDocumentCell[] {
  if (cell.kind !== "markdown") return [cell];
  const result: WorkflowGuideDocumentCell[] = [];
  const lines = cell.source.match(/[^\n]*\n|[^\n]+/g) ?? [];
  let offset = cell.startOffset;
  let markdownStart = offset;
  for (const line of lines) {
    const reference = /^\s*\[([^\]\n]+)\]\(((?:references\/[^\s)#]+\.md)|(?:(?:\.\/)?[^/\s)#]+\.md))(?:#[^\s)]+)?\)\s*$/.exec(line);
    const nextOffset = offset + line.length;
    const relativePath = reference && resolveExternalMarkdownPath(sourcePath, reference[2]);
    if (!reference || !relativePath) {
      offset = nextOffset;
      continue;
    }
    if (markdownStart < offset) {
      result.push({
        ...cell,
        id: `${cell.id}-markdown-${markdownStart}`,
        source: cell.source.slice(markdownStart - cell.startOffset, offset - cell.startOffset),
        startOffset: markdownStart,
        endOffset: offset,
      });
    }
    result.push({
      ...cell,
      id: `${cell.id}-external-${offset}`,
      kind: "external_reference",
      source: line,
      startOffset: offset,
      endOffset: nextOffset,
      externalReference: {
        title: reference[1],
        relativePath,
      },
    });
    markdownStart = nextOffset;
    offset = nextOffset;
  }
  if (markdownStart < cell.endOffset) {
    result.push({
      ...cell,
      id: `${cell.id}-markdown-${markdownStart}`,
      source: cell.source.slice(markdownStart - cell.startOffset),
      startOffset: markdownStart,
      endOffset: cell.endOffset,
    });
  }
  return result.length > 0 ? result : [cell];
}

function resolveExternalMarkdownPath(sourcePath: string, target: string): string | null {
  if (target.startsWith("references/")) return target;
  const separator = sourcePath.lastIndexOf("/");
  if (separator < 0) return null;
  const parent = sourcePath.slice(0, separator);
  return `${parent}/${target.replace(/^\.\//, "")}`;
}

function containsReservedWorkflowGuideSyntax(line: string) {
  return line.trimStart().startsWith(":::capability")
    || DIRECTIVE_END.test(line)
    || /\[[^\]]+\]\((references|scripts|assets)\/[^\s)]+\)/.test(line);
}

function isFenceMarker(line: string) {
  const trimmed = line.trimStart();
  return trimmed.startsWith("```") || trimmed.startsWith("~~~");
}

export function capabilitySource(
  name: string,
  exposure: "direct" | "meta_on_demand",
  guide: string,
) {
  const header = JSON.stringify({ name, exposure });
  return `:::capability ${header}\n${guide.trim()}\n:::`;
}

function renderCapability(capability: WorkflowGuideCapabilityBlock) {
  const exposure = capability.exposure === "direct" ? "Direct" : "Meta on demand";
  const header = `**Capability: ${capability.name}**  \nExposure: ${exposure}`;
  return capability.guide ? `${header}\n\n${capability.guide}` : header;
}
