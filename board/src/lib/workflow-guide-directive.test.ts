import { describe, expect, test } from "bun:test";

import { parseWorkflowGuide, renderWorkflowSkill, splitWorkflowGuideDocument } from "./workflow-guide-directive";

describe("parseWorkflowGuide", () => {
  test("derives headings and readable references without exposing IDs", () => {
    const guide = parseWorkflowGuide(`# Release investigation\n\n:::workflow-step {key="collect-evidence" title="Collect evidence"}\nCollect evidence with {{capability:search-release-logs}}.\n\nRead [Release policy](references/release-policy.md).\n:::`);

    expect(guide.errors).toEqual([]);
    expect(guide.headings).toEqual([{ level: 1, text: "Release investigation", offset: 0 }]);
    expect(guide.steps).toEqual([
      {
        key: "collect-evidence",
        title: "Collect evidence",
        startLine: 3,
        endLine: 7,
        body: "Collect evidence with {{capability:search-release-logs}}.\n\nRead [Release policy](references/release-policy.md).",
        references: [
          { kind: "capability", value: "search-release-logs" },
          { kind: "package_file", value: "references/release-policy.md" },
        ],
      },
    ]);
  });

  test("strips anchors from package file references", () => {
    const guide = parseWorkflowGuide(
      ':::workflow-step {key="read-policy" title="Read policy"}\nRead [Release policy](references/release-policy.md#retention).\n:::',
    );

    expect(guide.steps[0].references).toEqual([
      { kind: "package_file", value: "references/release-policy.md" },
    ]);
  });

  test("rejects pseudo references inside fenced code blocks", () => {
    const guide = parseWorkflowGuide("```markdown\n:::workflow-step {key=\"example\" title=\"Example\"}\n{{capability:example}}\n[Example](references/example.md)\n:::\n```");
    expect(guide.steps).toEqual([]);
    expect(guide.errors).toEqual([
      "line 2: Workflow Guide directives and references are not allowed in fenced code",
      "line 3: Workflow Guide directives and references are not allowed in fenced code",
      "line 4: Workflow Guide directives and references are not allowed in fenced code",
      "line 5: Workflow Guide directives and references are not allowed in fenced code",
    ]);
  });

  test("rejects pseudo references inside tilde fenced code blocks", () => {
    const guide = parseWorkflowGuide("~~~markdown\n{{capability:example}}\n~~~");
    expect(guide.errors).toEqual([
      "line 2: Workflow Guide directives and references are not allowed in fenced code",
    ]);
  });

  test("preserves fenced code inside a workflow step body", () => {
    const guide = parseWorkflowGuide(":::workflow-step {key=\"inspect-response\" title=\"Inspect response\"}\n```json\n{\"status\": \"ok\"}\n```\n:::");

    expect(guide.errors).toEqual([]);
    expect(guide.steps[0].body).toBe("```json\n{\"status\": \"ok\"}\n```");
  });

  test("reports duplicate and unclosed keys", () => {
    const guide = parseWorkflowGuide(":::workflow-step {key=\"same\" title=\"First\"}\n:::\n:::workflow-step {key=\"same\" title=\"Second\"}\n");
    expect(guide.errors).toEqual(["line 3: duplicate workflow step key 'same'", "line 3: workflow step 'same' is not closed"]);
  });

  test("reports local syntax violations before save", () => {
    const guide = parseWorkflowGuide(":::workflow-step {title=\"Missing key\"}\n:::\nProfile 123e4567-e89b-12d3-a456-426614174000\nskill://private");

    expect(guide.errors).toEqual([
      "line 1: invalid workflow step directive; expected key and title attributes",
      "line 2: workflow step end directive has no matching start",
      "line 3: opaque identifiers are not allowed in a Workflow Guide",
      "line 4: skill:// references are not allowed in a Workflow Guide",
    ]);
  });

  test("projects a readable Skill without directive syntax or opaque identifiers", () => {
    const skill = renderWorkflowSkill(
      `# Release investigation\n\n:::workflow-step {key="collect-evidence" title="Collect evidence"}\nUse {{capability:search-release-logs}}.\n:::\n\nThen use {{capability:search-release-logs}} to verify the conclusion.`,
      { "search-release-logs": "Search release logs" },
    );

    expect(skill.errors).toEqual([]);
    expect(skill.markdown).toContain("## Collect evidence");
    expect(skill.markdown).toContain("**Capability: Search release logs**");
    expect(skill.markdown).toContain("Then use **Capability: Search release logs** to verify the conclusion.");
    expect(skill.markdown).not.toContain(":::workflow-step");
    expect(skill.markdown).not.toContain("{{capability:");
    expect(skill.markdown).not.toMatch(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i);
  });

  test("splits narrative and workflow step cells without changing their source ranges", () => {
    const markdown = "# Release investigation\n\nBefore starting.\n\n:::workflow-step {key=\"collect-evidence\" title=\"Collect evidence\"}\nRead the logs.\n:::\n\nClose with findings.\n";

    const cells = splitWorkflowGuideDocument(markdown);

    expect(cells.map((cell) => cell.kind)).toEqual(["markdown", "workflow_step", "markdown"]);
    expect(cells.map((cell) => cell.source).join("")).toBe(markdown);
    expect(cells[1].step).toMatchObject({ key: "collect-evidence", title: "Collect evidence" });
  });

  test("creates a dedicated notebook cell for each outline heading", () => {
    const markdown = "# First section\n\nIntro.\n\n## Second section\n\nDetails.\n";
    const guide = parseWorkflowGuide(markdown);
    const cells = splitWorkflowGuideDocument(markdown);

    expect(cells.map((cell) => cell.startOffset)).toContain(guide.headings[0].offset);
    expect(cells.map((cell) => cell.startOffset)).toContain(guide.headings[1].offset);
  });

  test("recognizes a standalone external Markdown reference as a notebook unit", () => {
    const cells = splitWorkflowGuideDocument("[Release policy](references/release-policy.md)\n");

    expect(cells).toHaveLength(1);
    expect(cells[0]).toMatchObject({
      kind: "external_reference",
      externalReference: { title: "Release policy", relativePath: "references/release-policy.md" },
    });
  });

  test("keeps a root-relative Markdown link as ordinary Markdown", () => {
    const markdown = "[Local note](local-note.md)\n";
    const cells = splitWorkflowGuideDocument(markdown);

    expect(cells).toHaveLength(1);
    expect(cells[0]).toMatchObject({ kind: "markdown", source: markdown });
  });

  test("resolves a sibling external Markdown reference from its source document", () => {
    const cells = splitWorkflowGuideDocument(
      "[REST API](rest-api.md#authentication)\n",
      "references/cli-and-mcp.md",
    );

    expect(cells).toHaveLength(1);
    expect(cells[0]).toMatchObject({
      kind: "external_reference",
      externalReference: { title: "REST API", relativePath: "references/rest-api.md" },
    });
  });

  test("keeps inline sibling Markdown links as ordinary document content", () => {
    const markdown = "See [REST API](rest-api.md#authentication) for details.\n";
    const cells = splitWorkflowGuideDocument(markdown, "references/cli-and-mcp.md");

    expect(cells).toHaveLength(1);
    expect(cells[0]).toMatchObject({ kind: "markdown", source: markdown });
  });
});
