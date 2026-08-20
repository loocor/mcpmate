import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  BookOpenText,
  Check,
  ChevronRight,
  Eye,
  FilePlus2,
  FileText,
  Pencil,
  Plus,
  Save,
  Upload,
  Wrench,
} from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type SyntheticEvent,
} from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";

import { ApiRequestError, configSuitsApi } from "../lib/api";
import {
  parseWorkflowGuide,
  splitWorkflowGuideDocument,
  type WorkflowGuideDocumentCell,
} from "../lib/workflow-guide-directive";
import type {
  WorkflowGuideCapabilitySaveRequest,
  WorkflowGuideCapability,
  WorkflowGuideExternalDocument,
  WorkflowGuidePackageCategory,
  WorkflowGuidePackageFile,
  WorkflowGuideReclamationConfirmation,
} from "../lib/types";
import type { WorkflowCapabilityOption } from "../lib/profile-workflow-specification";
import { cn } from "../lib/utils";
import { notifyError, notifySuccess } from "../lib/notify";
import { Button } from "./ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "./ui/alert-dialog";
import { Input } from "./ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import { Segment } from "./ui/segment";
import { Textarea } from "./ui/textarea";
import { ResizableSplitPane } from "./resizable-split-pane";

interface ProfileWorkflowGuideProps {
  profileId: string;
  capabilities: WorkflowCapabilityOption[];
  capabilitiesLoading?: boolean;
}

export function ProfileWorkflowGuide({
  profileId,
  capabilities,
  capabilitiesLoading = false,
}: ProfileWorkflowGuideProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const selectionRef = useRef({ start: 0, end: 0 });
  const loadedGuideRevisionRef = useRef<number | null>(null);
  const guideQuery = useQuery({
    queryKey: ["workflowGuide", profileId],
    queryFn: () => configSuitsApi.getWorkflowGuide(profileId),
  });
  const [markdown, setMarkdown] = useState("");
  const [packageFiles, setPackageFiles] = useState<WorkflowGuidePackageFile[]>(
    [],
  );
  const [activeDocumentPath, setActiveDocumentPath] = useState("SKILL.md");
  const [externalDocuments, setExternalDocuments] = useState<
    Record<string, WorkflowGuideExternalDocument>
  >({});
  const [capabilityBindings, setCapabilityBindings] = useState<
    WorkflowGuideCapabilitySaveRequest[]
  >([]);
  const [editorMode, setEditorMode] = useState<"notebook" | "preview">(
    "notebook",
  );
  const [editingCellId, setEditingCellId] = useState<string | null>(null);
  const [pendingLocation, setPendingLocation] = useState<{
    path: string;
    offset: number;
  } | null>(null);
  const [pendingReclamation, setPendingReclamation] = useState<{
    target: "root" | "external";
    packageFiles: WorkflowGuidePackageFile[];
    capabilities: WorkflowGuideCapability[];
  } | null>(null);
  const editorOffsetRef = useRef(0);
  useEffect(() => {
    if (!guideQuery.data) return;
    setPackageFiles(guideQuery.data.package_files);
    if (loadedGuideRevisionRef.current !== guideQuery.data.guide_revision) {
      const normalizedMarkdown = stripLeadingSkillFrontMatter(
        guideQuery.data.markdown,
      ).body;
      setMarkdown(normalizedMarkdown);
      selectionRef.current = {
        start: normalizedMarkdown.length,
        end: normalizedMarkdown.length,
      };
      setCapabilityBindings(guideQuery.data.capabilities);
      setEditingCellId(null);
      loadedGuideRevisionRef.current = guideQuery.data.guide_revision;
    }
  }, [guideQuery.data]);

  const loadedExternalDocuments = useMemo(
    () =>
      Object.fromEntries(
        (guideQuery.data?.documents ?? []).map((document) =>
          [document.relative_path, document] as const,
        ),
      ),
    [guideQuery.data?.documents],
  );
  const resolvedExternalDocuments = useMemo(
    () => ({ ...loadedExternalDocuments, ...externalDocuments }),
    [loadedExternalDocuments, externalDocuments],
  );
  const reachableExternalDocuments = useMemo(
    () =>
      Object.keys(loadedExternalDocuments).map(
        (path) => resolvedExternalDocuments[path] ?? loadedExternalDocuments[path],
      ),
    [loadedExternalDocuments, resolvedExternalDocuments],
  );
  const activeExternalDocument =
    activeDocumentPath === "SKILL.md"
      ? null
      : resolvedExternalDocuments[activeDocumentPath];
  const activeMarkdown = activeExternalDocument?.markdown ?? markdown;
  const notebookMarkdown = useMemo(
    () => stripLeadingSkillFrontMatter(activeMarkdown).body,
    [activeMarkdown],
  );
  const updateActiveMarkdown = (updater: (current: string) => string) => {
    if (activeExternalDocument) {
      setExternalDocuments((current) => ({
        ...current,
        [activeDocumentPath]: {
          ...current[activeDocumentPath],
          markdown: updater(current[activeDocumentPath].markdown),
        },
      }));
      return;
    }
    setMarkdown(updater);
  };
  const guide = useMemo(
    () => parseWorkflowGuide(notebookMarkdown),
    [notebookMarkdown],
  );
  const mainGuideTitle = useMemo(
    () => parseWorkflowGuide(markdown).headings[0]?.text ?? "SKILL.md",
    [markdown],
  );
  const documentCells = useMemo(
    () => splitWorkflowGuideDocument(notebookMarkdown, activeDocumentPath),
    [activeDocumentPath, notebookMarkdown],
  );
  useEffect(() => {
    if (!pendingLocation || pendingLocation.path !== activeDocumentPath) return;
    const cell = documentCells.find(
      (candidate) =>
        candidate.startOffset <= pendingLocation.offset &&
        candidate.endOffset >= pendingLocation.offset,
    );
    if (!cell) return;
    document
      .getElementById(`guide-cell-${cell.id}`)
      ?.scrollIntoView({ behavior: "smooth", block: "center" });
    setPendingLocation(null);
  }, [activeDocumentPath, documentCells, pendingLocation]);
  const capabilityNames = useMemo(
    () =>
      Object.fromEntries(
        capabilityBindings.map((binding) => [
          binding.alias,
          binding.display_name,
        ]),
      ),
    [capabilityBindings],
  );
  const capabilitiesByRef = useMemo(
    () =>
      new Map(
        capabilities.map((capability) => [capability.ref_id, capability]),
      ),
    [capabilities],
  );
  const documentSources = useMemo(
    () => [
      { path: "SKILL.md", title: "SKILL.md", markdown },
      ...reachableExternalDocuments.map((document) => ({
        path: document.relative_path,
        title: document.title,
        markdown: document.markdown,
      })),
    ],
    [markdown, reachableExternalDocuments],
  );
  const capabilityOccurrences = useMemo(
    () =>
      collectOccurrences(
        documentSources,
        /\{\{capability:([a-z0-9][a-z0-9-]{0,62})\}\}/g,
      ),
    [documentSources],
  );
  const materialOccurrences = useMemo(
    () => collectMaterialOccurrences(documentSources),
    [documentSources],
  );
  const captureReclamation = (
    error: unknown,
    target: "root" | "external",
  ) => {
    if (
      !(error instanceof ApiRequestError) ||
      error.code !== "workflow_guide_reclamation_required"
    ) {
      return false;
    }
    setPendingReclamation({
      target,
      packageFiles: error.details?.packageFiles ?? [],
      capabilities: error.details?.capabilities ?? [],
    });
    return true;
  };
  const saveMutation = useMutation({
    mutationFn: (
      reclamationConfirmation?: WorkflowGuideReclamationConfirmation,
    ) =>
      configSuitsApi.saveWorkflowGuide({
        profile_id: profileId,
        expected_guide_revision: guideQuery.data!.guide_revision,
        markdown,
        capabilities: capabilityBindings,
        reclamation_confirmation: reclamationConfirmation,
      }),
    onSuccess: async (saved) => {
      await queryClient.invalidateQueries({
        queryKey: ["workflowGuide", profileId],
      });
      await queryClient.invalidateQueries({
        queryKey: ["workflowSpecification", profileId],
      });
      notifySuccess(
        t("profiles:detail.workflow.guide.saved", {
          defaultValue: "Workflow Guide saved",
        }),
      );
      setMarkdown(saved.guide.markdown);
      setPendingReclamation(null);
    },
    onError: (error) => {
      if (captureReclamation(error, "root")) return;
      notifyError(
        error instanceof Error
          ? error.message
          : t("profiles:detail.workflow.guide.saveFailed", {
              defaultValue: "Failed to save Workflow Guide",
            }),
      );
    },
  });
  const previewMutation = useMutation({
    mutationFn: () =>
      configSuitsApi.previewWorkflowGuide({
        profile_id: profileId,
        relative_path: activeExternalDocument?.relative_path,
        markdown: activeMarkdown,
        capabilities: capabilityBindings,
      }),
    onError: (error) =>
      notifyError(
        error instanceof Error
          ? error.message
          : t("profiles:detail.workflow.guide.previewFailed", {
              defaultValue: "Failed to render Skill Preview",
            }),
      ),
  });
  const externalDocumentMutation = useMutation({
    mutationFn: (file: WorkflowGuidePackageFile) =>
      configSuitsApi.getWorkflowGuideExternalDocument(
        profileId,
        file.package_file_id,
      ),
    onSuccess: (document) => {
      setExternalDocuments((current) => ({
        ...current,
        [document.relative_path]: document,
      }));
      setActiveDocumentPath(document.relative_path);
      setEditorMode("notebook");
      setEditingCellId(null);
    },
    onError: (error) =>
      notifyError(
        error instanceof Error
          ? error.message
          : t("profiles:detail.workflow.guide.documentLoadFailed", {
              defaultValue: "Failed to load external Markdown document",
            }),
      ),
  });
  const packageFileMutation = useMutation({
    mutationFn: (draft: {
      title: string;
      category: WorkflowGuidePackageCategory;
      file: File;
      knownPackageFileIds: string[];
    }) => {
      const formData = new FormData();
      formData.append("profile_id", profileId);
      formData.append(
        "expected_guide_revision",
        String(guideQuery.data!.guide_revision),
      );
      formData.append("title", draft.title.trim() || draft.file.name);
      formData.append("category", draft.category);
      formData.append("file", draft.file);
      return configSuitsApi
        .uploadWorkflowGuidePackageFile(formData)
        .then(async (saved) => ({
          saved: {
            ...saved,
            guide: await configSuitsApi.getWorkflowGuide(profileId),
          },
          draft,
        }));
    },
    onSuccess: ({ saved, draft }) => {
      const knownPackageFileIds = new Set(draft.knownPackageFileIds);
      const file = saved.guide.package_files.find(
        (candidate) => !knownPackageFileIds.has(candidate.package_file_id),
      );
      if (!file) {
        notifyError(
          t("profiles:detail.workflow.guide.fileSaveFailed", {
            defaultValue: "Failed to save package file",
          }),
        );
        return;
      }
      queryClient.setQueryData(["workflowGuide", profileId], saved.guide);
      setPackageFiles(saved.guide.package_files);
      insert(`[${file.title}](${file.relative_path})`);
      notifySuccess(
        t("profiles:detail.workflow.guide.fileSaved", {
          defaultValue: "Package file saved",
        }),
      );
    },
    onError: (error) =>
      notifyError(
        error instanceof Error
          ? error.message
          : t("profiles:detail.workflow.guide.fileSaveFailed", {
              defaultValue: "Failed to save package file",
            }),
      ),
  });
  const saveExternalDocumentMutation = useMutation({
    mutationFn: (
      reclamationConfirmation?: WorkflowGuideReclamationConfirmation,
    ) => {
      if (!activeExternalDocument)
        throw new Error("Select an external Markdown document first");
      const formData = new FormData();
      formData.append("profile_id", profileId);
      formData.append(
        "package_file_id",
        activeExternalDocument.package_file_id,
      );
      formData.append(
        "expected_file_revision",
        String(activeExternalDocument.file_revision),
      );
      formData.append(
        "expected_guide_revision",
        String(guideQuery.data!.guide_revision),
      );
      formData.append("capabilities", JSON.stringify(capabilityBindings));
      if (reclamationConfirmation) {
        formData.append(
          "reclamation_confirmation",
          JSON.stringify(reclamationConfirmation),
        );
      }
      formData.append("title", activeExternalDocument.title);
      formData.append("category", "reference");
      formData.append(
        "file",
        new File(
          [activeMarkdown],
          activeExternalDocument.relative_path.split("/").pop() ??
            "reference.md",
          { type: "text/markdown" },
        ),
      );
      return configSuitsApi.uploadWorkflowGuidePackageFile(formData);
    },
    onSuccess: (saved) => {
      const file = saved.guide.package_files.find(
        (candidate) =>
          candidate.package_file_id === activeExternalDocument?.package_file_id,
      );
      if (!file || !activeExternalDocument) return;
      setExternalDocuments((current) => ({
        ...current,
        [file.relative_path]: {
          ...activeExternalDocument,
          title: file.title,
          file_revision: file.file_revision,
          relative_path: file.relative_path,
        },
      }));
      queryClient.setQueryData(["workflowGuide", profileId], saved.guide);
      setPackageFiles(saved.guide.package_files);
      notifySuccess(
        t("profiles:detail.workflow.guide.documentSaved", {
          defaultValue: "External Markdown document saved",
        }),
      );
      setPendingReclamation(null);
    },
    onError: (error) => {
      if (captureReclamation(error, "external")) return;
      notifyError(
        error instanceof Error
          ? error.message
          : t("profiles:detail.workflow.guide.documentSaveFailed", {
              defaultValue: "Failed to save external Markdown document",
            }),
      );
    },
  });
  const createExternalDocumentMutation = useMutation({
    mutationFn: async (titleInput: string) => {
      const title = titleInput.trim();
      if (!title)
        throw new Error("External Markdown document title is required");
      const formData = new FormData();
      formData.append("profile_id", profileId);
      formData.append(
        "expected_guide_revision",
        String(guideQuery.data!.guide_revision),
      );
      formData.append("title", title);
      formData.append("category", "reference");
      const markdown = `# ${title}\n`;
      formData.append(
        "file",
        new File([markdown], `${title}.md`, { type: "text/markdown" }),
      );
      const saved =
        await configSuitsApi.uploadWorkflowGuidePackageFile(formData);
      const guide = await configSuitsApi.getWorkflowGuide(profileId);
      const knownPackageFileIds = new Set(
        packageFiles.map((file) => file.package_file_id),
      );
      const file = guide.package_files.find(
        (candidate) => !knownPackageFileIds.has(candidate.package_file_id),
      );
      if (!file)
        throw new Error("Created external Markdown document was not returned");
      return {
        saved: { ...saved, guide },
        document: {
          package_file_id: file.package_file_id,
          file_revision: file.file_revision,
          title: file.title,
          relative_path: file.relative_path,
          markdown,
        },
      };
    },
    onSuccess: ({ saved, document }) => {
      queryClient.setQueryData(["workflowGuide", profileId], saved.guide);
      setPackageFiles(saved.guide.package_files);
      setExternalDocuments((current) => ({
        ...current,
        [document.relative_path]: document,
      }));
      insert(`[${document.title}](${document.relative_path})`);
      setActiveDocumentPath(document.relative_path);
      setEditorMode("notebook");
      setEditingCellId(null);
      notifySuccess(
        t("profiles:detail.workflow.guide.documentCreated", {
          defaultValue: "External Markdown document created",
        }),
      );
    },
    onSettled: async () => {
      const guide = await configSuitsApi.getWorkflowGuide(profileId);
      queryClient.setQueryData(["workflowGuide", profileId], guide);
      setPackageFiles(guide.package_files);
    },
    onError: (error) =>
      notifyError(
        error instanceof Error
          ? error.message
          : t("profiles:detail.workflow.guide.documentCreateFailed", {
              defaultValue: "Failed to create external Markdown document",
            }),
      ),
  });
  const repairMutation = useMutation({
    mutationFn: () => configSuitsApi.repairWorkflowGuide(profileId),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ["workflowGuide", profileId],
      });
      notifySuccess(
        t("profiles:detail.workflow.guide.repaired", {
          defaultValue: "Skill package repaired",
        }),
      );
    },
    onError: (error) =>
      notifyError(
        error instanceof Error
          ? error.message
          : t("profiles:detail.workflow.guide.repairFailed", {
              defaultValue: "Failed to repair Skill package",
            }),
      ),
  });

  const insert = (value: string) => {
    const editor = editorRef.current;
    const { start, end } = selectionRef.current;
    updateActiveMarkdown(
      (current) => `${current.slice(0, start)}${value}${current.slice(end)}`,
    );
    if (!editor) return;
    requestAnimationFrame(() => {
      editor.focus();
      const cursor = start + value.length;
      const localCursor = cursor - editorOffsetRef.current;
      editor.setSelectionRange(localCursor, localCursor);
      selectionRef.current = { start: cursor, end: cursor };
    });
  };

  const updateCell = (cell: WorkflowGuideDocumentCell, value: string) => {
    updateActiveMarkdown(
      (current) =>
        `${current.slice(0, cell.startOffset)}${value}${current.slice(cell.endOffset)}`,
    );
  };

  const updateWorkflowStep = (
    cell: WorkflowGuideDocumentCell,
    title: string,
    body: string,
  ) => {
    if (!cell.step) return;
    updateCell(cell, workflowStepSource(cell.step.key, title, body));
  };

  const beginCellEdit = (cell: WorkflowGuideDocumentCell) => {
    const offset = cell.step
      ? cell.startOffset +
        workflowStepOpening(cell.step.key, cell.step.title).length
      : cell.startOffset;
    editorOffsetRef.current = offset;
    const cursor = cell.step ? offset + cell.step.body.length : cell.endOffset;
    selectionRef.current = { start: cursor, end: cursor };
    setEditingCellId(cell.id);
  };

  const trackSelection = (
    event: SyntheticEvent<HTMLTextAreaElement>,
    offset: number,
  ) => {
    const target = event.currentTarget;
    editorOffsetRef.current = offset;
    selectionRef.current = {
      start: offset + target.selectionStart,
      end: offset + target.selectionEnd,
    };
  };

  const insertWorkflowStep = () => {
    const existingKeys = new Set(guide.steps.map((step) => step.key));
    let ordinal = 1;
    let key = "new-step";
    while (existingKeys.has(key)) {
      ordinal += 1;
      key = `new-step-${ordinal}`;
    }
    insert(
      workflowStepSource(
        key,
        "New workflow step",
        "Describe the action and insert references here.",
      ),
    );
  };

  const insertCapability = (
    capability: WorkflowCapabilityOption,
    bindingPolicy: "direct" | "meta_on_demand",
  ) => {
    const existing = capabilityBindings.find(
      (binding) => binding.ref_id === capability.ref_id,
    );
    const alias =
      existing?.alias ?? nextAlias(capability.label, capabilityBindings);
    setCapabilityBindings((current) =>
      existing
        ? current.map((binding) =>
            binding.ref_id === capability.ref_id
              ? { ...binding, binding_policy: bindingPolicy }
              : binding,
          )
        : [
            ...current,
            {
              alias,
              display_name: capability.label,
              ref_id: capability.ref_id,
              binding_policy: bindingPolicy,
            },
          ],
    );
    insert(`{{capability:${alias}}}`);
  };
  const openOccurrence = (path: string, offset: number) => {
    setEditingCellId(null);
    setPendingLocation({ path, offset });
    if (path === "SKILL.md") {
      setActiveDocumentPath(path);
      return;
    }
    const file = guideQuery.data?.package_files.find(
      (candidate) => candidate.relative_path === path,
    );
    if (file) externalDocumentMutation.mutate(file);
  };
  const openOutlineHeading = (offset: number) => {
    setEditorMode("notebook");
    setEditingCellId(null);
    setPendingLocation({ path: activeDocumentPath, offset });
  };
  const confirmReclamation = () => {
    if (!pendingReclamation) return;
    const confirmation: WorkflowGuideReclamationConfirmation = {
      package_files: pendingReclamation.packageFiles.map((file) => ({
        package_file_id: file.package_file_id,
        file_revision: file.file_revision,
      })),
      capability_aliases: pendingReclamation.capabilities.map(
        (capability) => capability.alias,
      ),
    };
    const target = pendingReclamation.target;
    setPendingReclamation(null);
    if (target === "external") {
      saveExternalDocumentMutation.mutate(confirmation);
    } else {
      saveMutation.mutate(confirmation);
    }
  };

  if (guideQuery.isLoading) {
    return (
      <p className="p-4 text-sm text-muted-foreground">
        {t("common:loading", { defaultValue: "Loading..." })}
      </p>
    );
  }
  if (guideQuery.isError || !guideQuery.data) {
    return (
      <p className="p-4 text-sm text-destructive">
        {t("profiles:detail.workflow.guide.loadFailed", {
          defaultValue: "Failed to load Workflow Guide.",
        })}
      </p>
    );
  }

  return (
    <section
      className="flex min-h-0 flex-1 flex-col overflow-hidden bg-background"
      aria-label="Workflow Guide"
    >
      <header className="flex items-center justify-between border-b px-4 py-2">
        <div>
          <h3 className="text-sm font-semibold">
            {t("profiles:detail.workflow.guide.title", {
              defaultValue: "Workflow Guide",
            })}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t("profiles:detail.workflow.guide.description", {
              defaultValue:
                "Write the Skill narrative and insert readable references where they are needed.",
            })}
          </p>
        </div>
        <Segment
          className="w-48 shrink-0"
          options={[
            {
              value: "notebook",
              label: t("profiles:detail.workflow.guide.notebook", {
                defaultValue: "Notebook",
              }),
              icon: <BookOpenText className="h-3.5 w-3.5" />,
            },
            {
              value: "preview",
              label: t("profiles:detail.workflow.guide.preview", {
                defaultValue: "Preview",
              }),
              icon: <Eye className="h-3.5 w-3.5" />,
            },
          ]}
          showDots={false}
          value={editorMode}
          onValueChange={(value) => {
            setEditingCellId(null);
            const nextMode = value as "notebook" | "preview";
            setEditorMode(nextMode);
            if (nextMode === "preview") previewMutation.mutate();
          }}
        />
      </header>
      <ResizableSplitPane
        className="min-h-0 flex-1"
        dividerAriaLabel="Resize outline panel"
        initialLeftWidth={280}
        minLeftWidth={208}
        maxLeftWidth={520}
        preferRightPanelSpace
      >
        <nav className="overflow-auto border-r p-3" aria-label="Guide outline">
          <ol className="space-y-0.5 text-sm">
            <li className="group">
              <button
                aria-label="Open main Guide"
                className={cn(
                  "flex w-full items-center gap-1.5 rounded-sm px-1.5 py-1 text-left hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                  activeDocumentPath === "SKILL.md" && "bg-muted",
                )}
                onClick={() => {
                  setActiveDocumentPath("SKILL.md");
                  setEditorMode("notebook");
                  setEditingCellId(null);
                }}
                type="button"
              >
                <span className="min-w-0 flex-1 truncate">
                  {mainGuideTitle}
                </span>
                <span className="shrink-0 rounded-sm bg-muted px-1 text-[10px] text-muted-foreground">
                  SKILL.md
                </span>
              </button>
            </li>
            {guide.headings.map((heading, index) => (
              <li
                className="relative"
                key={`${heading.text}-${index}`}
                style={{
                  marginLeft: `${Math.max(0, heading.level - 1) * 0.9}rem`,
                }}
              >
                {heading.level > 1 ? (
                  <span
                    aria-hidden="true"
                    className="absolute inset-y-0 left-0 border-l border-border"
                  />
                ) : null}
                <button
                  className="relative flex w-full items-center gap-1.5 rounded-sm px-1.5 py-1 text-left hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  onClick={() => openOutlineHeading(heading.offset)}
                  type="button"
                >
                  <span
                    aria-hidden="true"
                    className={cn(
                      "h-1.5 w-1.5 shrink-0 rounded-full",
                      heading.level === 1
                        ? "bg-foreground/70"
                        : "bg-muted-foreground/50",
                    )}
                  />
                  <span className="min-w-0 flex-1 truncate">
                    {heading.text}
                  </span>
                  <span className="shrink-0 font-mono text-[10px] text-muted-foreground">
                    H{heading.level}
                  </span>
                </button>
              </li>
            ))}
            {packageFiles
              .filter(
                (file) =>
                  file.category === "reference" && file.extension === "md",
              )
              .map((file) => (
                <li className="group" key={file.package_file_id}>
                  <button
                    className={cn(
                      "flex w-full items-center gap-1.5 rounded-sm px-1.5 py-1 text-left hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                      activeDocumentPath === file.relative_path && "bg-muted",
                    )}
                    onClick={() => externalDocumentMutation.mutate(file)}
                    type="button"
                  >
                    <span className="min-w-0 flex-1 truncate">
                      {file.title}
                    </span>
                    <span className="shrink-0 rounded-sm bg-muted px-1 text-[10px] text-muted-foreground">
                      Reference
                    </span>
                  </button>
                </li>
              ))}
          </ol>
        </nav>
        <div className="flex min-w-0 min-h-0 flex-col">
          <nav
            aria-label="Guide document breadcrumb"
            className="flex h-9 shrink-0 items-center gap-1 border-b px-4 text-xs text-muted-foreground"
          >
            <button
              className="hover:text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              onClick={() => {
                setActiveDocumentPath("SKILL.md");
                setEditorMode("notebook");
                setEditingCellId(null);
              }}
              type="button"
            >
              SKILL.md
            </button>
            {activeExternalDocument ? (
              <>
                <ChevronRight className="h-3.5 w-3.5" />
                <span className="truncate text-foreground">
                  {activeExternalDocument.relative_path}
                </span>
              </>
            ) : null}
          </nav>
          <div className="flex min-w-0 min-h-0 flex-1">
            <div className="flex min-w-0 flex-1 flex-col">
              <div className="min-h-0 flex-1 overflow-auto p-4">
                {editorMode === "preview" ? (
                  <section
                    aria-label={t("profiles:detail.workflow.guide.preview", {
                      defaultValue: "Preview",
                    })}
                    className="rounded-md border bg-muted/20 p-4"
                  >
                    <p className="mb-3 text-xs text-muted-foreground">
                      {t("profiles:detail.workflow.guide.previewDescription", {
                        defaultValue:
                          "Rendered from the current draft without saving.",
                      })}
                    </p>
                    {previewMutation.isPending ? (
                      <p className="text-sm text-muted-foreground">
                        {t("common:loading", { defaultValue: "Loading..." })}
                      </p>
                    ) : null}
                    {previewMutation.data ? (
                      <SkillPreview
                        content={
                          activeExternalDocument
                            ? previewMutation.data.active_document.markdown
                            : previewMutation.data.projected_skill.markdown
                        }
                      />
                    ) : null}
                  </section>
                ) : (
                  <div
                    className="space-y-0"
                    aria-label={t("profiles:detail.workflow.guide.notebook", {
                      defaultValue: "Workflow Guide notebook",
                    })}
                  >
                    <GuideBoundaryInsert
                      capabilities={capabilities}
                      capabilitiesLoading={capabilitiesLoading}
                      files={packageFiles}
                      onInsert={insert}
                      onInsertCapability={insertCapability}
                      onInsertStep={insertWorkflowStep}
                      onCreateExternalDocument={(title) =>
                        createExternalDocumentMutation.mutate(title)
                      }
                      creatingExternalDocument={
                        createExternalDocumentMutation.isPending
                      }
                      onCreatePackageFile={(draft) =>
                        packageFileMutation.mutate({
                          ...draft,
                          knownPackageFileIds: packageFiles.map(
                            (file) => file.package_file_id,
                          ),
                        })
                      }
                      creatingPackageFile={packageFileMutation.isPending}
                      onSetInsertionPoint={(offset) => {
                        selectionRef.current = { start: offset, end: offset };
                      }}
                      offset={0}
                    />
                    {documentCells.map((cell) => (
                      <div key={cell.id}>
                        <article
                          className={cn(
                            "group relative rounded-sm px-3 py-2 transition-colors hover:bg-muted/20 focus-within:bg-muted/20",
                            editingCellId === cell.id && "bg-muted/20",
                          )}
                          id={`guide-cell-${cell.id}`}
                        >
                          <header className="absolute right-2 top-1 flex items-center gap-2 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
                            <span className="text-[10px] text-muted-foreground">
                              {cell.kind === "workflow_step"
                                ? t(
                                    "profiles:detail.workflow.guide.workflowStep",
                                    { defaultValue: "Workflow step" },
                                  )
                                : cell.kind === "external_reference"
                                  ? t(
                                      "profiles:detail.workflow.guide.externalReference",
                                      { defaultValue: "External Markdown" },
                                    )
                                  : t(
                                      "profiles:detail.workflow.guide.markdownBlock",
                                      { defaultValue: "Markdown" },
                                    )}
                            </span>
                            <Button
                              aria-label={
                                editingCellId === cell.id
                                  ? t("common:done", { defaultValue: "Done" })
                                  : t(
                                      "profiles:detail.workflow.guide.editBlock",
                                      { defaultValue: "Edit block" },
                                    )
                              }
                              size="icon"
                              variant="ghost"
                              onClick={() =>
                                editingCellId === cell.id
                                  ? setEditingCellId(null)
                                  : beginCellEdit(cell)
                              }
                            >
                              {editingCellId === cell.id ? (
                                <Check className="h-3.5 w-3.5" />
                              ) : (
                                <Pencil className="h-3.5 w-3.5" />
                              )}
                            </Button>
                          </header>
                          <div className="min-w-0 pr-8">
                            {editingCellId === cell.id &&
                            cell.kind === "workflow_step" &&
                            cell.step ? (
                              <div className="space-y-3">
                                <div className="grid gap-2 sm:grid-cols-[10rem_minmax(0,1fr)] sm:items-center">
                                  <p className="text-xs font-medium text-muted-foreground">
                                    {t(
                                      "profiles:detail.workflow.guide.stepKey",
                                      { defaultValue: "Step key" },
                                    )}
                                  </p>
                                  <code className="truncate font-mono text-xs">
                                    {cell.step.key}
                                  </code>
                                </div>
                                <label className="block space-y-1 text-xs font-medium text-muted-foreground">
                                  {t(
                                    "profiles:detail.workflow.guide.stepTitle",
                                    { defaultValue: "Step title" },
                                  )}
                                  <Input
                                    value={cell.step.title}
                                    onChange={(event) =>
                                      updateWorkflowStep(
                                        cell,
                                        event.target.value,
                                        cell.step!.body,
                                      )
                                    }
                                  />
                                </label>
                                <label className="block space-y-1 text-xs font-medium text-muted-foreground">
                                  {t(
                                    "profiles:detail.workflow.guide.stepInstructions",
                                    { defaultValue: "Instructions" },
                                  )}
                                  <Textarea
                                    autoFocus
                                    ref={editorRef}
                                    aria-label={t(
                                      "profiles:detail.workflow.guide.stepInstructions",
                                      {
                                        defaultValue:
                                          "Workflow step instructions",
                                      },
                                    )}
                                    className="min-h-36 resize-y font-mono text-sm"
                                    value={cell.step.body}
                                    onChange={(event) => {
                                      const bodyOffset =
                                        cell.startOffset +
                                        workflowStepOpening(
                                          cell.step!.key,
                                          cell.step!.title,
                                        ).length;
                                      trackSelection(event, bodyOffset);
                                      updateWorkflowStep(
                                        cell,
                                        cell.step!.title,
                                        event.target.value,
                                      );
                                    }}
                                    onSelect={(event) =>
                                      trackSelection(
                                        event,
                                        cell.startOffset +
                                          workflowStepOpening(
                                            cell.step!.key,
                                            cell.step!.title,
                                          ).length,
                                      )
                                    }
                                  />
                                </label>
                              </div>
                            ) : editingCellId === cell.id ? (
                              <Textarea
                                autoFocus
                                ref={editorRef}
                                aria-label={t(
                                  "profiles:detail.workflow.guide.markdownSource",
                                  { defaultValue: "Markdown block source" },
                                )}
                                className="min-h-36 resize-y font-mono text-sm"
                                value={cell.source}
                                onChange={(event) => {
                                  trackSelection(event, cell.startOffset);
                                  updateCell(cell, event.target.value);
                                }}
                                onSelect={(event) =>
                                  trackSelection(event, cell.startOffset)
                                }
                              />
                            ) : cell.kind === "workflow_step" && cell.step ? (
                              <GuideMarkdownPreview
                                capabilityNames={capabilityNames}
                                content={`## ${cell.step.title}\n\n${cell.step.body}`}
                              />
                            ) : cell.kind === "external_reference" &&
                              cell.externalReference ? (
                              <button
                                className="text-left text-sm font-medium text-primary underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                                onClick={() => {
                                  const file = packageFiles.find(
                                    (candidate) =>
                                      candidate.relative_path ===
                                      cell.externalReference!.relativePath,
                                  );
                                  if (file)
                                    externalDocumentMutation.mutate(file);
                                }}
                                type="button"
                              >
                                {cell.externalReference.title}
                              </button>
                            ) : (
                              <GuideMarkdownPreview
                                capabilityNames={capabilityNames}
                                content={cell.source}
                                emptyLabel={t(
                                  "profiles:detail.workflow.guide.emptyMarkdownBlock",
                                  { defaultValue: "Empty Markdown block" },
                                )}
                              />
                            )}
                          </div>
                        </article>
                        <GuideBoundaryInsert
                          capabilities={capabilities}
                          capabilitiesLoading={capabilitiesLoading}
                          files={packageFiles}
                          onInsert={insert}
                          onInsertCapability={insertCapability}
                          onInsertStep={insertWorkflowStep}
                          onCreateExternalDocument={(title) =>
                            createExternalDocumentMutation.mutate(title)
                          }
                          creatingExternalDocument={
                            createExternalDocumentMutation.isPending
                          }
                          onCreatePackageFile={(draft) =>
                            packageFileMutation.mutate({
                              ...draft,
                              knownPackageFileIds: packageFiles.map(
                                (file) => file.package_file_id,
                              ),
                            })
                          }
                          creatingPackageFile={packageFileMutation.isPending}
                          onSetInsertionPoint={(offset) => {
                            selectionRef.current = {
                              start: offset,
                              end: offset,
                            };
                          }}
                          offset={cell.endOffset}
                        />
                      </div>
                    ))}
                  </div>
                )}
                {guide.errors.map((error) => (
                  <p className="text-sm text-destructive" key={error}>
                    {error}
                  </p>
                ))}
              </div>
            </div>
            <aside
              className="w-64 shrink-0 overflow-auto border-l px-3 py-3"
              aria-label="Guide inspector"
            >
              <section>
                <h4 className="px-1.5 text-xs font-medium">
                  {t("profiles:detail.workflow.guide.capabilities", {
                    defaultValue: "Capabilities",
                  })}{" "}
                  ({capabilityOccurrences.size})
                </h4>
                <div className="mt-1 space-y-0.5">
                  {[...capabilityOccurrences.entries()].map(
                    ([alias, occurrences]) => {
                      const binding = capabilityBindings.find(
                        (candidate) => candidate.alias === alias,
                      );
                      const description = binding
                        ? capabilitiesByRef.get(binding.ref_id)?.description
                        : undefined;
                      return (
                        <div
                          className="group rounded-sm px-1.5 py-1 hover:bg-muted focus-within:bg-muted"
                          key={alias}
                        >
                          <div className="flex items-center gap-1.5">
                            <p className="min-w-0 flex-1 truncate text-xs font-medium">
                              {capabilityNames[alias] ?? alias}
                            </p>
                            <span className="shrink-0 text-[10px] text-muted-foreground">
                              {occurrences.length}
                            </span>
                          </div>
                          <div className="grid grid-rows-[0fr] overflow-hidden opacity-0 transition-[grid-template-rows,opacity] duration-150 group-hover:grid-rows-[1fr] group-hover:opacity-100 group-focus-within:grid-rows-[1fr] group-focus-within:opacity-100">
                            <div className="min-h-0">
                              <div className="space-y-1 pt-1.5">
                                {description ? (
                                  <p className="line-clamp-2 text-[11px] text-muted-foreground">
                                    {description}
                                  </p>
                                ) : null}
                                <p className="text-[11px] text-muted-foreground">
                                  {binding?.binding_policy === "direct"
                                    ? "Direct exposure"
                                    : "Meta on demand"}
                                </p>
                                <div className="flex flex-wrap gap-1">
                                  {occurrences.map((occurrence, index) => (
                                    <Button
                                      className="h-6 px-1.5 text-[11px]"
                                      key={`${occurrence.path}-${occurrence.offset}`}
                                      size="sm"
                                      variant="ghost"
                                      onClick={() =>
                                        openOccurrence(
                                          occurrence.path,
                                          occurrence.offset,
                                        )
                                      }
                                    >
                                      {t(
                                        "profiles:detail.workflow.guide.place",
                                        { defaultValue: "Place" },
                                      )}{" "}
                                      {index + 1} · {occurrence.path}
                                    </Button>
                                  ))}
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                      );
                    },
                  )}
                  {capabilityOccurrences.size === 0 ? (
                    <p className="px-1.5 py-1 text-xs text-muted-foreground">
                      {t("profiles:detail.workflow.guide.noCapabilities", {
                        defaultValue: "No capability references yet.",
                      })}
                    </p>
                  ) : null}
                </div>
              </section>
              <section className="mt-4">
                <h4 className="px-1.5 text-xs font-medium">
                  {t("profiles:detail.workflow.guide.materials", {
                    defaultValue: "Materials",
                  })}{" "}
                  ({materialOccurrences.size})
                </h4>
                <div className="mt-1 space-y-0.5">
                  {[...materialOccurrences.entries()].map(
                    ([path, occurrences]) => {
                      const file = packageFiles.find(
                        (candidate) => candidate.relative_path === path,
                      );
                      return (
                        <div
                          className="group rounded-sm px-1.5 py-1 hover:bg-muted focus-within:bg-muted"
                          key={path}
                        >
                          <div className="flex items-center gap-1.5">
                            <p className="min-w-0 flex-1 truncate text-xs font-medium">
                              {file?.title ?? path}
                            </p>
                            <span className="shrink-0 text-[10px] text-muted-foreground">
                              {occurrences.length}
                            </span>
                          </div>
                          <div className="grid grid-rows-[0fr] overflow-hidden opacity-0 transition-[grid-template-rows,opacity] duration-150 group-hover:grid-rows-[1fr] group-hover:opacity-100 group-focus-within:grid-rows-[1fr] group-focus-within:opacity-100">
                            <div className="min-h-0">
                              <div className="space-y-1 pt-1.5">
                                {file ? (
                                  <p className="break-all text-[11px] text-muted-foreground">
                                    {file.category} · {file.relative_path}
                                  </p>
                                ) : null}
                                <div className="flex flex-wrap gap-1">
                                  {occurrences.map((occurrence, index) => (
                                    <Button
                                      className="h-6 px-1.5 text-[11px]"
                                      key={`${occurrence.path}-${occurrence.offset}`}
                                      size="sm"
                                      variant="ghost"
                                      onClick={() =>
                                        openOccurrence(
                                          occurrence.path,
                                          occurrence.offset,
                                        )
                                      }
                                    >
                                      {t(
                                        "profiles:detail.workflow.guide.place",
                                        { defaultValue: "Place" },
                                      )}{" "}
                                      {index + 1} · {occurrence.path}
                                    </Button>
                                  ))}
                                </div>
                                {file?.category === "reference" &&
                                file.extension === "md" ? (
                                  <Button
                                    className="h-6 px-1.5 text-[11px]"
                                    size="sm"
                                    variant="ghost"
                                    onClick={() =>
                                      externalDocumentMutation.mutate(file)
                                    }
                                  >
                                    {t(
                                      "profiles:detail.workflow.guide.openDocument",
                                      { defaultValue: "Open document" },
                                    )}
                                  </Button>
                                ) : null}
                              </div>
                            </div>
                          </div>
                        </div>
                      );
                    },
                  )}
                  {materialOccurrences.size === 0 ? (
                    <p className="px-1.5 py-1 text-xs text-muted-foreground">
                      {t("profiles:detail.workflow.guide.noMaterials", {
                        defaultValue: "No material references yet.",
                      })}
                    </p>
                  ) : null}
                </div>
              </section>
            </aside>
          </div>
          <footer className="flex shrink-0 items-center justify-between border-t px-4 py-3">
            <Button
              size="sm"
              variant="ghost"
              onClick={() => repairMutation.mutate()}
              disabled={repairMutation.isPending}
            >
              <Wrench className="mr-1 h-3.5 w-3.5" />
              {t("profiles:detail.workflow.guide.repair", {
                defaultValue: "Repair",
              })}
            </Button>
            <Button
              size="sm"
              onClick={() =>
                activeExternalDocument
                  ? saveExternalDocumentMutation.mutate()
                  : saveMutation.mutate()
              }
              disabled={
                saveMutation.isPending ||
                saveExternalDocumentMutation.isPending ||
                guide.errors.length > 0
              }
            >
              <Save className="mr-1 h-3.5 w-3.5" />
              {t("common:save", { defaultValue: "Save" })}
            </Button>
          </footer>
        </div>
      </ResizableSplitPane>
      <AlertDialog
        open={pendingReclamation !== null}
        onOpenChange={(open) => {
          if (!open) setPendingReclamation(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("profiles:detail.workflow.guide.reclamationTitle", {
                defaultValue: "Confirm removed references",
              })}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("profiles:detail.workflow.guide.reclamationDescription", {
                defaultValue:
                  "Saving will remove Profile bindings and move package files that are no longer reachable to Trash.",
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="flex max-h-72 flex-col gap-4 overflow-auto text-sm">
            {pendingReclamation?.capabilities.length ? (
              <section className="flex flex-col gap-2">
                <h4 className="font-medium">
                  {t("profiles:detail.workflow.guide.capabilities", {
                    defaultValue: "Capabilities",
                  })}
                </h4>
                <ul className="flex flex-col gap-1 text-muted-foreground">
                  {pendingReclamation.capabilities.map((capability) => (
                    <li key={capability.alias}>{capability.display_name}</li>
                  ))}
                </ul>
              </section>
            ) : null}
            {pendingReclamation?.packageFiles.length ? (
              <section className="flex flex-col gap-2">
                <h4 className="font-medium">
                  {t("profiles:detail.workflow.guide.materials", {
                    defaultValue: "Materials",
                  })}
                </h4>
                <ul className="flex flex-col gap-1 text-muted-foreground">
                  {pendingReclamation.packageFiles.map((file) => (
                    <li key={file.package_file_id}>
                      {file.title} ({file.relative_path})
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel>
              {t("common:cancel", { defaultValue: "Cancel" })}
            </AlertDialogCancel>
            <AlertDialogAction onClick={confirmReclamation}>
              {t("profiles:detail.workflow.guide.confirmSave", {
                defaultValue: "Confirm and save",
              })}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  );
}

function GuideBoundaryInsert({
  capabilities,
  capabilitiesLoading,
  files,
  offset,
  onInsert,
  onInsertCapability,
  onInsertStep,
  onCreateExternalDocument,
  creatingExternalDocument,
  onCreatePackageFile,
  creatingPackageFile,
  onSetInsertionPoint,
}: {
  capabilities: WorkflowCapabilityOption[];
  capabilitiesLoading: boolean;
  files: WorkflowGuidePackageFile[];
  offset: number;
  onInsert: (value: string) => void;
  onInsertCapability: (
    capability: WorkflowCapabilityOption,
    bindingPolicy: "direct" | "meta_on_demand",
  ) => void;
  onInsertStep: () => void;
  onCreateExternalDocument: (title: string) => void;
  creatingExternalDocument: boolean;
  onCreatePackageFile: (draft: {
    title: string;
    category: WorkflowGuidePackageCategory;
    file: File;
  }) => void;
  creatingPackageFile: boolean;
  onSetInsertionPoint: (offset: number) => void;
}) {
  const [externalDocumentTitle, setExternalDocumentTitle] = useState("");
  const [packageTitle, setPackageTitle] = useState("");
  const [packageCategory, setPackageCategory] =
    useState<WorkflowGuidePackageCategory>("reference");
  const [packageUpload, setPackageUpload] = useState<File | null>(null);
  const [selectedCapability, setSelectedCapability] =
    useState<WorkflowCapabilityOption | null>(null);
  const [bindingPolicy, setBindingPolicy] = useState<
    "direct" | "meta_on_demand"
  >("meta_on_demand");

  return (
    <div
      className="group/boundary flex h-6 items-center justify-center"
      onFocus={() => onSetInsertionPoint(offset)}
      onMouseEnter={() => onSetInsertionPoint(offset)}
    >
      <div className="h-px flex-1 bg-transparent transition-colors group-hover/boundary:bg-border group-focus-within/boundary:bg-border" />
      <Popover>
        <PopoverTrigger asChild>
          <Button
            aria-label="Insert at this position"
            className="mx-1 h-5 w-5 rounded-full border bg-background p-0 opacity-0 shadow-none transition-opacity group-hover/boundary:opacity-100 group-focus-within/boundary:opacity-100"
            onClick={() => onSetInsertionPoint(offset)}
            size="icon"
            variant="ghost"
          >
            <Plus className="h-3 w-3" />
          </Button>
        </PopoverTrigger>
        <PopoverContent align="center" className="w-64 p-2">
          <div className="space-y-1">
            <p className="px-2 py-1 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
              Insert
            </p>
            <Button
              className="w-full justify-start"
              onClick={() => onInsert("\n\n## New section\n\n")}
              size="sm"
              variant="ghost"
            >
              <FileText className="mr-2 h-3.5 w-3.5" />
              Markdown section
            </Button>
            <Button
              className="w-full justify-start"
              onClick={onInsertStep}
              size="sm"
              variant="ghost"
            >
              <FilePlus2 className="mr-2 h-3.5 w-3.5" />
              Workflow step
            </Button>
          </div>
          <details className="mt-2 border-t pt-2">
            <summary className="cursor-pointer px-2 py-1 text-xs font-medium">
              Capabilities
            </summary>
            <div className="mt-1 max-h-40 space-y-1 overflow-auto">
              {capabilitiesLoading ? (
                <p className="px-2 py-1 text-xs text-muted-foreground">
                  Loading…
                </p>
              ) : (
                capabilities.map((capability) => (
                  <Button
                    className="w-full justify-start"
                    key={capability.ref_id}
                    onClick={() => setSelectedCapability(capability)}
                    size="sm"
                    variant={
                      selectedCapability?.ref_id === capability.ref_id
                        ? "secondary"
                        : "ghost"
                    }
                  >
                    {capability.label}
                  </Button>
                ))
              )}
            </div>
            {selectedCapability ? (
              <div className="mt-2 space-y-2 border-t px-2 pt-2">
                <p className="text-xs font-medium">
                  {selectedCapability.label}
                </p>
                {selectedCapability.description ? (
                  <p className="text-[11px] leading-4 text-muted-foreground">
                    <span className="font-medium">
                      Tool usage description:{" "}
                    </span>
                    {selectedCapability.description}
                  </p>
                ) : null}
                <label
                  className="block text-[11px] font-medium text-muted-foreground"
                  htmlFor={`capability-exposure-${offset}`}
                >
                  Exposure
                </label>
                <select
                  aria-label="Capability exposure"
                  className="h-8 w-full rounded-md border bg-background px-2 text-xs"
                  id={`capability-exposure-${offset}`}
                  value={bindingPolicy}
                  onChange={(event) =>
                    setBindingPolicy(
                      event.target.value as "direct" | "meta_on_demand",
                    )
                  }
                >
                  <option value="meta_on_demand">Meta on demand</option>
                  <option value="direct">Direct exposure</option>
                </select>
                <Button
                  className="w-full justify-start"
                  onClick={() =>
                    onInsertCapability(selectedCapability, bindingPolicy)
                  }
                  size="sm"
                >
                  <Plus className="mr-2 h-3.5 w-3.5" />
                  Insert capability
                </Button>
              </div>
            ) : null}
          </details>
          <details className="mt-2 border-t pt-2">
            <summary className="cursor-pointer px-2 py-1 text-xs font-medium">
              Materials
            </summary>
            <div className="mt-1 max-h-40 space-y-1 overflow-auto">
              {files.map((file) => (
                <Button
                  className="w-full justify-start"
                  key={file.package_file_id}
                  onClick={() =>
                    onInsert(`[${file.title}](${file.relative_path})`)
                  }
                  size="sm"
                  variant="ghost"
                >
                  {file.title}
                </Button>
              ))}
              {files.length === 0 ? (
                <p className="px-2 py-1 text-xs text-muted-foreground">
                  No package files yet.
                </p>
              ) : null}
            </div>
            <div className="mt-2 border-t px-2 pt-2">
              <label
                className="block text-[11px] font-medium text-muted-foreground"
                htmlFor={`external-document-${offset}`}
              >
                New external Markdown
              </label>
              <div className="mt-1 flex gap-1">
                <Input
                  id={`external-document-${offset}`}
                  value={externalDocumentTitle}
                  onChange={(event) =>
                    setExternalDocumentTitle(event.target.value)
                  }
                  placeholder="Section name"
                />
                <Button
                  aria-label="Create external Markdown"
                  disabled={
                    !externalDocumentTitle.trim() || creatingExternalDocument
                  }
                  onClick={() => {
                    onCreateExternalDocument(externalDocumentTitle);
                    setExternalDocumentTitle("");
                  }}
                  size="icon"
                  type="button"
                >
                  <FilePlus2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>
            <div className="mt-2 space-y-2 border-t px-2 pt-2">
              <p className="text-[11px] font-medium text-muted-foreground">
                Upload package file
              </p>
              <Input
                value={packageTitle}
                onChange={(event) => setPackageTitle(event.target.value)}
                placeholder="File title"
              />
              <select
                aria-label="Package file category"
                className="h-8 w-full rounded-md border bg-background px-2 text-xs"
                value={packageCategory}
                onChange={(event) =>
                  setPackageCategory(
                    event.target.value as WorkflowGuidePackageCategory,
                  )
                }
              >
                <option value="reference">
                  Reference (.md, .json, .yaml, .toml)
                </option>
                <option value="script">Script (.js, .mjs, .cjs, .py)</option>
                <option value="asset">Asset (.pdf, .docx, .xlsx)</option>
              </select>
              <Input
                aria-label="Package file upload"
                accept={acceptedExtensions(packageCategory)}
                type="file"
                onChange={(event) =>
                  setPackageUpload(event.target.files?.[0] ?? null)
                }
              />
              <Button
                className="w-full justify-start"
                disabled={!packageUpload || creatingPackageFile}
                onClick={() => {
                  if (!packageUpload) return;
                  onCreatePackageFile({
                    title: packageTitle,
                    category: packageCategory,
                    file: packageUpload,
                  });
                  setPackageTitle("");
                  setPackageUpload(null);
                }}
                size="sm"
              >
                <Upload className="mr-2 h-3.5 w-3.5" />
                Upload and insert
              </Button>
            </div>
          </details>
        </PopoverContent>
      </Popover>
      <div className="h-px flex-1 bg-transparent transition-colors group-hover/boundary:bg-border group-focus-within/boundary:bg-border" />
    </div>
  );
}

function acceptedExtensions(category: WorkflowGuidePackageCategory) {
  if (category === "reference") return ".md,.json,.yaml,.yml,.toml";
  if (category === "script") return ".js,.mjs,.cjs,.py";
  return ".pdf,.docx,.xlsx";
}

function collectOccurrences(
  documents: Array<{ path: string; title: string; markdown: string }>,
  expression: RegExp,
) {
  const occurrences = new Map<
    string,
    Array<{ path: string; offset: number }>
  >();
  for (const document of documents) {
    expression.lastIndex = 0;
    for (const match of document.markdown.matchAll(expression)) {
      const key = match[1];
      const offset = match.index ?? 0;
      occurrences.set(key, [
        ...(occurrences.get(key) ?? []),
        { path: document.path, offset },
      ]);
    }
  }
  return occurrences;
}

function collectMaterialOccurrences(
  documents: Array<{ path: string; title: string; markdown: string }>,
) {
  const occurrences = collectOccurrences(
    documents,
    /\[[^\]\n]+\]\(((?:references|scripts|assets)\/[^\s)#]+)(?:#[^\s)]+)?\)/g,
  );
  const siblingReference = /\[[^\]\n]+\]\(((?:\.\/)?[^/\s)#]+\.md)(?:#[^\s)]+)?\)/g;
  for (const document of documents) {
    if (document.path === "SKILL.md") continue;
    siblingReference.lastIndex = 0;
    const parent = document.path.slice(0, document.path.lastIndexOf("/"));
    for (const match of document.markdown.matchAll(siblingReference)) {
      const fileName = match[1].replace(/^\.\//, "");
      const key = `${parent}/${fileName}`;
      occurrences.set(key, [
        ...(occurrences.get(key) ?? []),
        { path: document.path, offset: match.index ?? 0 },
      ]);
    }
  }
  return occurrences;
}

function workflowStepOpening(key: string, title: string) {
  return `:::workflow-step {key="${key}" title="${title}"}\n`;
}

function workflowStepSource(key: string, title: string, body: string) {
  return `${workflowStepOpening(key, title)}${body}\n:::\n`;
}

function GuideMarkdownPreview({
  capabilityNames,
  content,
  emptyLabel,
}: {
  capabilityNames: Record<string, string>;
  content: string;
  emptyLabel?: string;
}) {
  const renderedContent = content.replace(
    /\{\{capability:([a-z0-9][a-z0-9-]{0,62})\}\}/g,
    (_reference, alias: string) => {
      return `**Capability: ${capabilityNames[alias] ?? alias}**`;
    },
  );
  if (!renderedContent.trim()) {
    return (
      <p className="text-sm italic text-muted-foreground">
        {emptyLabel ?? "Empty"}
      </p>
    );
  }
  return (
    <div className="min-w-0 text-sm leading-6 text-slate-700 dark:text-slate-300">
      <ReactMarkdown
        rehypePlugins={[rehypeSanitize]}
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => (
            <h1 className="mb-3 text-xl font-semibold">{children}</h1>
          ),
          h2: ({ children }) => (
            <h2 className="mb-3 text-lg font-semibold">{children}</h2>
          ),
          h3: ({ children }) => (
            <h3 className="mb-2 text-base font-semibold">{children}</h3>
          ),
          p: ({ children }) => <p className="mb-3 last:mb-0">{children}</p>,
          code: ({ children }) => (
            <code className="rounded bg-muted px-1 font-mono text-xs">
              {children}
            </code>
          ),
          pre: ({ children }) => (
            <pre className="overflow-x-auto rounded bg-muted p-3 font-mono text-xs">
              {children}
            </pre>
          ),
        }}
      >
        {renderedContent}
      </ReactMarkdown>
    </div>
  );
}

function SkillPreview({ content }: { content: string }) {
  const { frontMatter, body } = stripLeadingSkillFrontMatter(content);
  return (
    <div>
      <dl className="mb-5 grid gap-3 border-b pb-4 text-sm sm:grid-cols-[7rem_minmax(0,1fr)]">
        <dt className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          Name
        </dt>
        <dd className="min-w-0">
          <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
            {frontMatter.name ?? "—"}
          </code>
        </dd>
        <dt className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          Description
        </dt>
        <dd className="leading-6 text-slate-700 dark:text-slate-300">
          {frontMatter.description ?? "—"}
        </dd>
      </dl>
      <GuideMarkdownPreview capabilityNames={{}} content={body} />
    </div>
  );
}

function stripLeadingSkillFrontMatter(content: string) {
  if (!content.startsWith("---\n"))
    return { frontMatter: {} as Record<string, string>, body: content };
  const closingOffset = content.indexOf("\n---\n", 4);
  if (closingOffset < 0)
    return { frontMatter: {} as Record<string, string>, body: content };
  const frontMatter = Object.fromEntries(
    content
      .slice(4, closingOffset)
      .split("\n")
      .flatMap((line) => {
        const match = /^(name|description):\s*(.+)$/.exec(line);
        return match ? [[match[1], match[2].replace(/^['"]|['"]$/g, "")]] : [];
      }),
  );
  return { frontMatter, body: content.slice(closingOffset + 5) };
}

function nextAlias(
  label: string,
  existing: WorkflowGuideCapabilitySaveRequest[],
) {
  const base =
    label
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "capability";
  const aliases = new Set(existing.map((binding) => binding.alias));
  for (let ordinal = 1; ; ordinal += 1) {
    const candidate = ordinal === 1 ? base : `${base}-${ordinal}`;
    if (!aliases.has(candidate)) return candidate;
  }
}
