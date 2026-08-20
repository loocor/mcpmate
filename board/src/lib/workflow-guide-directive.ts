export interface WorkflowGuideReference {
  kind: "capability" | "package_file";
  value: string;
}

export interface WorkflowGuideStepBlock {
  key: string;
  title: string;
  body: string;
  references: WorkflowGuideReference[];
  startLine: number;
  endLine: number;
}

export interface WorkflowGuideParseResult {
  headings: Array<{ level: number; text: string; offset: number }>;
  steps: WorkflowGuideStepBlock[];
  errors: string[];
}

export interface WorkflowGuideDocumentCell {
  id: string;
  kind: "markdown" | "external_reference" | "workflow_step";
  source: string;
  startOffset: number;
  endOffset: number;
  step?: WorkflowGuideStepBlock;
  externalReference?: { title: string; relativePath: string };
}

const STEP_START = /^:::workflow-step\s+\{key="([a-z0-9][a-z0-9-]{0,62})"\s+title="([^"\n]{1,120})"\}\s*$/;
const STEP_END = /^:::\s*$/;
const CAPABILITY_REFERENCE = /\{\{capability:([a-z0-9][a-z0-9-]{0,62})\}\}/g;
const PACKAGE_REFERENCE = /\[[^\]]+\]\((references|scripts|assets)\/([^\s)#]+)(?:#[^\s)]+)?\)/g;
const UUID_REFERENCE = /\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b/i;

export function parseWorkflowGuide(markdown: string): WorkflowGuideParseResult {
  const headings: Array<{ level: number; text: string; offset: number }> = [];
  const steps: WorkflowGuideStepBlock[] = [];
  const errors: string[] = [];
  const keys = new Set<string>();
  const lines = markdown.split("\n");
  const lineOffsets: number[] = [];
  let nextOffset = 0;
  for (const line of lines) {
    lineOffsets.push(nextOffset);
    nextOffset += line.length + 1;
  }
  let fenced = false;
  let active: { key: string; title: string; startLine: number; lines: string[] } | null = null;

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
      const start = STEP_START.exec(line);
      if (start) {
        if (keys.has(start[1])) errors.push(`line ${lineNumber}: duplicate workflow step key '${start[1]}'`);
        keys.add(start[1]);
        active = { key: start[1], title: start[2], startLine: lineNumber, lines: [] };
      } else if (line.trimStart().startsWith(":::workflow-step")) {
        errors.push(`line ${lineNumber}: invalid workflow step directive; expected key and title attributes`);
      } else if (STEP_END.test(line)) {
        errors.push(`line ${lineNumber}: workflow step end directive has no matching start`);
      }
      return;
    }

    if (STEP_END.test(line)) {
      const body = active.lines.join("\n").trim();
      steps.push({
        key: active.key,
        title: active.title,
        body,
        startLine: active.startLine,
        endLine: lineNumber,
        references: collectReferences(body),
      });
      active = null;
      return;
    }
    active.lines.push(line);
  });

  if (active) errors.push(`line ${active.startLine}: workflow step '${active.key}' is not closed`);
  lines.forEach((line, index) => {
    if (UUID_REFERENCE.test(line)) errors.push(`line ${index + 1}: opaque identifiers are not allowed in a Workflow Guide`);
    if (line.includes("skill://")) errors.push(`line ${index + 1}: skill:// references are not allowed in a Workflow Guide`);
  });
  return { headings, steps, errors };
}

export function renderWorkflowSkill(markdown: string, capabilityNames: Record<string, string>) {
  const parsed = parseWorkflowGuide(markdown);
  if (parsed.errors.length > 0) return { markdown: "", errors: parsed.errors };

  const stepsByStartLine = new Map(parsed.steps.map((step) => [step.startLine, step]));
  const lines = markdown.split("\n");
  const output: string[] = [];

  for (let index = 0; index < lines.length; index += 1) {
    const lineNumber = index + 1;
    const step = stepsByStartLine.get(lineNumber);
    if (step) {
      output.push(`## ${step.title}`);
      output.push(replaceCapabilityReferences(step.body, capabilityNames));
      index = step.endLine - 1;
      continue;
    }
    output.push(replaceCapabilityReferences(lines[index], capabilityNames));
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
  for (const step of parsed.steps) {
    const startOffset = lineOffsets[step.startLine - 1] ?? markdown.length;
    const endOffset = lineOffsets[step.endLine] ?? markdown.length;
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
      id: `workflow-step-${step.key}-${startOffset}`,
      kind: "workflow_step",
      source: markdown.slice(startOffset, endOffset),
      startOffset,
      endOffset,
      step,
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

function collectReferences(body: string): WorkflowGuideReference[] {
  const references: WorkflowGuideReference[] = [];
  for (const match of body.matchAll(CAPABILITY_REFERENCE)) references.push({ kind: "capability", value: match[1] });
  for (const match of body.matchAll(PACKAGE_REFERENCE)) references.push({ kind: "package_file", value: `${match[1]}/${match[2]}` });
  return references;
}

function containsReservedWorkflowGuideSyntax(line: string) {
  return line.trimStart().startsWith(":::workflow-step")
    || STEP_END.test(line)
    || /\{\{capability:[a-z0-9][a-z0-9-]{0,62}\}\}/.test(line)
    || /\[[^\]]+\]\((references|scripts|assets)\/[^\s)]+\)/.test(line);
}

function isFenceMarker(line: string) {
  const trimmed = line.trimStart();
  return trimmed.startsWith("```") || trimmed.startsWith("~~~");
}

function replaceCapabilityReferences(body: string, capabilityNames: Record<string, string>) {
  return body.replace(CAPABILITY_REFERENCE, (_reference, key: string) => {
    const name = capabilityNames[key];
    return name ? `**Capability: ${name}**` : `**Capability: ${key}**`;
  });
}
