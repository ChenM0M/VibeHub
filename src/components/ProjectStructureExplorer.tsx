import { useEffect, useMemo, useState } from 'react';
import {
    AlertCircle,
    Boxes,
    ChevronDown,
    ChevronRight,
    ExternalLink,
    File,
    Folder,
    FolderOpen,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { labelOrFallback } from './VibehubCockpitDialog';
import {
    VibehubProjectStructureGraphNode,
    VibehubProjectStructureTreeNode,
    VibehubProjectStructureViewData,
} from '@/types';
import { tauriApi } from '@/services/tauri';

export interface ProjectStructureExplorerProps {
    projectStructure: VibehubProjectStructureViewData | null;
    projectPath: string;
    t: (key: string, options?: Record<string, unknown>) => string;
}

type StructureNode = VibehubProjectStructureGraphNode | VibehubProjectStructureTreeNode;

export function ProjectStructureExplorer({ projectStructure, projectPath, t }: ProjectStructureExplorerProps) {
    const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
    const [expandedPaths, setExpandedPaths] = useState<Set<string>>(() => new Set());
    const [actionError, setActionError] = useState<string | null>(null);
    const graphNodes = projectStructure?.graph_nodes || [];
    const treeNodes = projectStructure?.tree || [];
    const allNodes = useMemo(() => dedupeNodes([...graphNodes, ...flattenTreeNodes(treeNodes)]), [graphNodes, treeNodes]);
    const selectedNode = allNodes.find((node) => node.id === selectedNodeId)
        || allNodes.find((node) => node.kind !== 'root')
        || allNodes[0]
        || null;

    useEffect(() => {
        if (!selectedNode && selectedNodeId) {
            setSelectedNodeId(null);
        }
        if (!selectedNodeId && selectedNode) {
            setSelectedNodeId(selectedNode.id);
        }
    }, [selectedNode, selectedNodeId]);

    useEffect(() => {
        if (!selectedNode?.path) return;
        const next = new Set(expandedPaths);
        const parts = selectedNode.path.split('/').filter(Boolean);
        let current = '';
        parts.slice(0, -1).forEach((part) => {
            current = current ? `${current}/${part}` : part;
            next.add(current);
        });
        if (next.size !== expandedPaths.size) {
            setExpandedPaths(next);
        }
    }, [selectedNode?.path]);

    const toggleExpanded = (node: VibehubProjectStructureTreeNode) => {
        setExpandedPaths((current) => {
            const next = new Set(current);
            if (next.has(node.path)) {
                next.delete(node.path);
            } else {
                next.add(node.path);
            }
            return next;
        });
    };

    const runFileAction = async (mode: 'reveal' | 'open') => {
        if (!selectedNode) return;
        setActionError(null);
        try {
            if (mode === 'reveal') {
                await tauriApi.vibehubRevealProjectFile(projectPath, selectedNode.path);
            } else {
                await tauriApi.vibehubOpenProjectFile(projectPath, selectedNode.path);
            }
        } catch (error) {
            setActionError(String(error));
        }
    };

    if (!projectStructure) {
        return (
            <div className="border border-border/70 px-4 py-6 text-sm text-muted-foreground">
                {labelOrFallback(t, 'vibehub.projectMap.structureEmpty', 'No structure data found.')}
            </div>
        );
    }

    return (
        <div className="flex h-full min-h-[min(42rem,calc(100vh-8rem))] flex-col bg-background text-foreground">
            <header className="border-b border-border/70 pb-4">
                <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                    <div className="min-w-0">
                        <div className="flex items-center gap-2 text-xs font-semibold uppercase text-muted-foreground">
                            <Boxes className="h-4 w-4" />
                            {labelOrFallback(t, 'vibehub.projectMap.structure', 'Project structure')}
                        </div>
                        <h3 className="mt-2 truncate text-xl font-semibold tracking-tight">{projectPath}</h3>
                        <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                            <span>{labelOrFallback(t, 'vibehub.projectMap.structureCounts', '{{dirs}} dirs · {{files}} files', {
                                dirs: projectStructure.scanned_dirs_count,
                                files: projectStructure.scanned_files_count,
                            })}</span>
                            <Badge variant="outline">{projectStructure.source}</Badge>
                            {projectStructure.truncated && (
                                <Badge variant="secondary">{labelOrFallback(t, 'vibehub.projectMap.structureTruncated', 'Truncated')}</Badge>
                            )}
                        </div>
                    </div>
                    <div className="flex flex-wrap gap-2">
                        <Button variant="outline" size="sm" onClick={() => runFileAction('reveal')} disabled={!selectedNode}>
                            <FolderOpen className="mr-2 h-4 w-4" />
                            {labelOrFallback(t, 'vibehub.projectMap.structureRevealFile', 'Show in file manager')}
                        </Button>
                        <Button variant="outline" size="sm" onClick={() => runFileAction('open')} disabled={!selectedNode || selectedNode.kind !== 'file'}>
                            <ExternalLink className="mr-2 h-4 w-4" />
                            {labelOrFallback(t, 'vibehub.projectMap.structureOpenFile', 'Open file')}
                        </Button>
                    </div>
                </div>
                {!projectStructure.semantic_graph_available && (
                    <div className="mt-4 flex items-start gap-2 border border-border/70 bg-muted/20 px-3 py-2 text-xs leading-5 text-muted-foreground">
                        <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                        {labelOrFallback(t, 'vibehub.projectMap.structureSource', 'Filesystem scan; semantic graph is not configured yet.')}
                    </div>
                )}
                {projectStructure.warnings.length > 0 && (
                    <div className="mt-3 space-y-2">
                        {projectStructure.warnings.map((warning) => (
                            <div key={warning} className="border border-amber-500/30 bg-amber-500/5 px-3 py-2 text-xs text-amber-700 dark:text-amber-400">
                                {warning}
                            </div>
                        ))}
                    </div>
                )}
                {actionError && (
                    <div className="mt-3 border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">
                        {actionError}
                    </div>
                )}
            </header>

            <div className="grid min-h-0 flex-1 gap-4 pt-4 xl:grid-cols-[minmax(18rem,0.8fr)_minmax(0,1.25fr)_minmax(18rem,0.7fr)]">
                <section className="min-h-0 border border-border/70">
                    <PanelHeader title={labelOrFallback(t, 'vibehub.projectMap.moduleMap', 'Module map')} count={graphNodes.length} />
                    <div className="max-h-[34rem] overflow-auto px-2 py-2">
                        {graphNodes.length ? graphNodes.map((node) => (
                            <StructureListButton
                                key={node.id}
                                node={node}
                                selected={selectedNode?.id === node.id}
                                onSelect={() => setSelectedNodeId(node.id)}
                            />
                        )) : (
                            <div className="px-3 py-4 text-sm text-muted-foreground">
                                {labelOrFallback(t, 'vibehub.projectMap.structureEmpty', 'No structure data found.')}
                            </div>
                        )}
                    </div>
                </section>

                <section className="min-h-0 border border-border/70">
                    <PanelHeader title={labelOrFallback(t, 'vibehub.projectMap.structureDirectory', 'Directory')} count={treeNodes.length} />
                    <div className="max-h-[34rem] overflow-auto px-2 py-2">
                        {treeNodes.length ? treeNodes.map((node) => (
                            <DirectoryTreeNode
                                key={node.id}
                                node={node}
                                selectedId={selectedNode?.id || null}
                                expandedPaths={expandedPaths}
                                onToggle={toggleExpanded}
                                onSelect={setSelectedNodeId}
                            />
                        )) : (
                            <div className="px-3 py-4 text-sm text-muted-foreground">
                                {labelOrFallback(t, 'vibehub.projectMap.structureEmpty', 'No structure data found.')}
                            </div>
                        )}
                    </div>
                </section>

                <aside className="min-h-0 border border-border/70">
                    <PanelHeader title={labelOrFallback(t, 'vibehub.projectMap.structureSelected', 'Selected')} />
                    {selectedNode ? (
                        <div className="space-y-4 px-3 py-3">
                            <div>
                                <div className="mb-2 flex items-center gap-2 text-sm font-medium">
                                    {selectedNode.kind === 'file' ? <File className="h-4 w-4" /> : <Folder className="h-4 w-4" />}
                                    <span className="min-w-0 truncate">{selectedNode.label}</span>
                                </div>
                                <div className="break-all font-mono text-xs text-muted-foreground">{selectedNode.path}</div>
                            </div>
                            <div className="grid grid-cols-2 gap-2 text-xs">
                                <MetaCell label="Kind" value={selectedNode.kind} />
                                <MetaCell label="Depth" value={String(selectedNode.depth)} />
                                <MetaCell label="Files" value={String(selectedNode.file_count)} />
                                <MetaCell label="Dirs" value={String(selectedNode.directory_count)} />
                            </div>
                            {selectedNode.changed && (
                                <div className="border border-primary/30 bg-primary/5 px-3 py-2 text-xs text-primary">
                                    {labelOrFallback(t, 'vibehub.projectMap.structureChanged', 'Changed')}
                                </div>
                            )}
                            <div className="border-t border-border/70 pt-3 text-xs leading-5 text-muted-foreground">
                                {labelOrFallback(t, 'vibehub.projectMap.structureHistoryReserved', 'Module/file history is reserved for a later slice.')}
                            </div>
                        </div>
                    ) : (
                        <div className="px-3 py-4 text-sm text-muted-foreground">
                            {labelOrFallback(t, 'vibehub.projectMap.structureEmpty', 'No structure data found.')}
                        </div>
                    )}
                </aside>
            </div>
        </div>
    );
}

function PanelHeader({ title, count }: { title: string; count?: number }) {
    return (
        <div className="flex items-center justify-between gap-3 border-b border-border/70 px-3 py-2">
            <div className="text-xs font-semibold uppercase text-muted-foreground">{title}</div>
            {typeof count === 'number' && <Badge variant="secondary">{count}</Badge>}
        </div>
    );
}

function StructureListButton({
    node,
    selected,
    onSelect,
}: {
    node: VibehubProjectStructureGraphNode;
    selected: boolean;
    onSelect: () => void;
}) {
    return (
        <button
            type="button"
            onClick={onSelect}
            className={`mb-1 flex w-full items-center gap-2 px-2 py-2 text-left text-sm transition hover:bg-muted/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${selected ? 'bg-primary/10 text-primary' : ''}`}
        >
            <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${node.changed ? 'bg-primary' : 'bg-muted-foreground/40'}`} />
            <span className="min-w-0 flex-1">
                <span className="block truncate font-medium">{node.label}</span>
                <span className="block truncate font-mono text-[11px] text-muted-foreground">{node.path}</span>
            </span>
            <span className="shrink-0 text-xs text-muted-foreground">{node.file_count}</span>
        </button>
    );
}

function DirectoryTreeNode({
    node,
    selectedId,
    expandedPaths,
    onToggle,
    onSelect,
}: {
    node: VibehubProjectStructureTreeNode;
    selectedId: string | null;
    expandedPaths: Set<string>;
    onToggle: (node: VibehubProjectStructureTreeNode) => void;
    onSelect: (id: string) => void;
}) {
    const isDir = node.kind === 'directory';
    const expanded = !isDir || expandedPaths.has(node.path) || node.depth <= 1;
    const selected = selectedId === node.id;
    const indent = Math.min(Math.max(node.depth - 1, 0), 8) * 14;

    return (
        <div>
            <div
                className={`flex items-center gap-2 px-2 py-1.5 text-sm transition hover:bg-muted/30 ${selected ? 'bg-primary/10 text-primary' : node.changed ? 'text-primary' : ''}`}
                style={{ paddingLeft: `${indent + 8}px` }}
            >
                <button
                    type="button"
                    onClick={() => isDir && onToggle(node)}
                    className="flex h-4 w-4 shrink-0 items-center justify-center text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    aria-label={node.label}
                >
                    {isDir ? (expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />) : null}
                </button>
                <button
                    type="button"
                    onClick={() => onSelect(node.id)}
                    className="flex min-w-0 flex-1 items-center gap-2 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                    {isDir ? <Folder className="h-4 w-4 shrink-0 text-muted-foreground" /> : <File className="h-4 w-4 shrink-0 text-muted-foreground" />}
                    <span className="min-w-0 flex-1 truncate font-mono text-xs">{node.label}</span>
                    {node.truncated && <Badge variant="secondary" className="shrink-0 text-[10px]">...</Badge>}
                </button>
            </div>
            {isDir && expanded && node.children.length > 0 && (
                <div>
                    {node.children.map((child) => (
                        <DirectoryTreeNode
                            key={child.id}
                            node={child}
                            selectedId={selectedId}
                            expandedPaths={expandedPaths}
                            onToggle={onToggle}
                            onSelect={onSelect}
                        />
                    ))}
                </div>
            )}
        </div>
    );
}

function MetaCell({ label, value }: { label: string; value: string }) {
    return (
        <div className="border border-border/70 px-2 py-1.5">
            <div className="text-[10px] uppercase text-muted-foreground">{label}</div>
            <div className="truncate text-xs font-medium">{value}</div>
        </div>
    );
}

function flattenTreeNodes(nodes: VibehubProjectStructureTreeNode[]): VibehubProjectStructureTreeNode[] {
    const out: VibehubProjectStructureTreeNode[] = [];
    const visit = (items: VibehubProjectStructureTreeNode[]) => {
        items.forEach((item) => {
            out.push(item);
            visit(item.children || []);
        });
    };
    visit(nodes);
    return out;
}

function dedupeNodes(nodes: StructureNode[]): StructureNode[] {
    const seen = new Set<string>();
    return nodes.filter((node) => {
        const key = node.id || node.path;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
    });
}
