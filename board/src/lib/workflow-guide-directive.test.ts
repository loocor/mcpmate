import { describe, expect, test } from "bun:test";

import {
  capabilitySource,
  parseWorkflowGuide,
  renderWorkflowSkill,
  splitWorkflowGuideDocument,
} from "./workflow-guide-directive";

describe("Workflow Guide directives", () => {
  test("parses occurrence-level canonical name, exposure, and guide", () => {
    const guide = parseWorkflowGuide(
      '# Release investigation\n\n:::capability {"name":"server__lookup","exposure":"direct"}\nUse PNG output for this occurrence.\n:::\n',
    );

    expect(guide.errors).toEqual([]);
    expect(guide.headings).toEqual([
      { level: 1, text: "Release investigation", offset: 0 },
    ]);
    expect(guide.capabilities).toEqual([
      {
        name: "server__lookup",
        exposure: "direct",
        guide: "Use PNG output for this occurrence.",
        startLine: 3,
        endLine: 5,
      },
    ]);
  });

  test("allows repeated names with independent exposure and guide", () => {
    const guide = parseWorkflowGuide(
      `${capabilitySource("server__lookup", "meta_on_demand", "Inspect details first.")}\n${capabilitySource("server__lookup", "direct", "Call directly here.")}`,
    );

    expect(guide.errors).toEqual([]);
    expect(guide.capabilities.map(({ exposure, guide }) => ({ exposure, guide }))).toEqual([
      { exposure: "meta_on_demand", guide: "Inspect details first." },
      { exposure: "direct", guide: "Call directly here." },
    ]);
  });

  test("rejects reserved syntax inside fenced code", () => {
    const guide = parseWorkflowGuide(
      '```markdown\n:::capability {"name":"server__lookup","exposure":"direct"}\n[Example](references/example.md)\n:::\n```',
    );
    expect(guide.capabilities).toEqual([]);
    expect(guide.errors).toEqual([
      "line 2: Workflow Guide directives and references are not allowed in fenced code",
      "line 3: Workflow Guide directives and references are not allowed in fenced code",
      "line 4: Workflow Guide directives and references are not allowed in fenced code",
    ]);
  });

  test("keeps shorter fence markers inside a longer fence", () => {
    const guide = parseWorkflowGuide(
      '````markdown\n```\n:::capability {"name":"server__lookup","exposure":"direct"}\n:::\n```\n````',
    );

    expect(guide.capabilities).toEqual([]);
    expect(guide.errors).toContain(
      "line 3: Workflow Guide directives and references are not allowed in fenced code",
    );
    expect(guide.errors).toContain(
      "line 4: Workflow Guide directives and references are not allowed in fenced code",
    );
  });

  test("reports malformed directives and opaque identifiers", () => {
    const guide = parseWorkflowGuide(
      ':::capability {"name":"server__lookup"}\n123e4567-e89b-12d3-a456-426614174000\nskill://private',
    );

    expect(guide.errors).toHaveLength(3);
    expect(guide.errors[0]).toContain("Capability exposure");
    expect(guide.errors[1]).toContain("opaque identifiers");
    expect(guide.errors[2]).toContain("skill://");
  });

  test("projects readable neutral Markdown", () => {
    const skill = renderWorkflowSkill(
      `# Release investigation\n\n${capabilitySource("server__lookup", "direct", "Use PNG output.")}`,
    );

    expect(skill.errors).toEqual([]);
    expect(skill.markdown).toContain("**Capability: server__lookup**");
    expect(skill.markdown).toContain("Exposure: Direct");
    expect(skill.markdown).toContain("Use PNG output.");
    expect(skill.markdown).not.toContain(":::capability");
  });

  test("splits narrative and Capability cells without changing source ranges", () => {
    const markdown = `# Release investigation\n\nBefore starting.\n\n${capabilitySource("server__lookup", "direct", "Read the logs.")}\n\nClose with findings.\n`;
    const cells = splitWorkflowGuideDocument(markdown);

    expect(cells.map((cell) => cell.kind)).toEqual([
      "markdown",
      "capability",
      "markdown",
    ]);
    expect(cells.map((cell) => cell.source).join("")).toBe(markdown);
    expect(cells[1].capability).toMatchObject({
      name: "server__lookup",
      exposure: "direct",
      guide: "Read the logs.",
    });
  });

  test("creates a dedicated notebook cell for each outline heading", () => {
    const markdown = "# First section\n\nIntro.\n\n## Second section\n\nDetails.\n";
    const guide = parseWorkflowGuide(markdown);
    const cells = splitWorkflowGuideDocument(markdown);
    expect(cells.map((cell) => cell.startOffset)).toContain(guide.headings[0].offset);
    expect(cells.map((cell) => cell.startOffset)).toContain(guide.headings[1].offset);
  });

  test("recognizes standalone external Markdown references", () => {
    const cells = splitWorkflowGuideDocument(
      "[Release policy](references/release-policy.md)\n",
    );
    expect(cells).toHaveLength(1);
    expect(cells[0]).toMatchObject({
      kind: "external_reference",
      externalReference: {
        title: "Release policy",
        relativePath: "references/release-policy.md",
      },
    });
  });

  test("resolves sibling Markdown references only when standalone", () => {
    const standalone = splitWorkflowGuideDocument(
      "[REST API](rest-api.md#authentication)\n",
      "references/cli-and-mcp.md",
    );
    expect(standalone[0]).toMatchObject({
      kind: "external_reference",
      externalReference: { relativePath: "references/rest-api.md" },
    });

    const inline = "See [REST API](rest-api.md#authentication) for details.\n";
    expect(
      splitWorkflowGuideDocument(inline, "references/cli-and-mcp.md")[0],
    ).toMatchObject({ kind: "markdown", source: inline });
  });
});
