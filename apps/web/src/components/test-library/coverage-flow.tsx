import { Badge, Card, Group, Select, Stack, Text, Title } from '@mantine/core'
import { useMediaQuery } from '@mantine/hooks'
import {
  Background,
  Controls,
  Handle,
  MarkerType,
  Position,
  ReactFlow,
  type CoordinateExtent,
  type Edge,
  type Node,
  type NodeProps,
  type NodeTypes,
} from '@xyflow/react'
import {
  AlertTriangle,
  Check,
  Circle,
  Layers3,
  LoaderCircle,
  Play,
  Plus,
  Route,
} from 'lucide-react'
import { useMemo, type ReactNode } from 'react'
import type {
  AttemptResponse,
  CaseSelection,
  JobState,
  LibraryDraftDefinition,
  LibraryOptionsResponse,
  LibraryVersionResponse,
  RunResponse,
} from '@/api/generated/types.gen'
import styles from './coverage-flow.module.css'

export type CoverageFlowStatus =
  | 'ready'
  | 'needs_setup'
  | 'missing'
  | 'queued'
  | 'running'
  | 'passed'
  | 'failed'
  | 'blocked'
  | 'inconclusive'
  | 'skipped'
  | 'canceled'
  | 'recovery'
  | 'not_started'
export type FlowOrientation = 'horizontal' | 'vertical'

type CoverageNodeData = {
  kind: 'root' | 'group' | 'case'
  orientation: FlowOrientation
  eyebrow: string
  title: string
  version?: number
  count?: number
  order?: number
  required?: boolean
  status?: CoverageFlowStatus
  finished?: boolean
  detail?: string
  band?: boolean
  onActivate?: () => void
}
type CoverageNode = Node<CoverageNodeData, 'root' | 'group' | 'case'>
export type CoverageFlowModel = {
  nodes: CoverageNode[]
  edges: Edge[]
  itemCount: number
  orientation: FlowOrientation
  contentHeight: number
  contentWidth: number
}

const STATUS_LABEL: Record<CoverageFlowStatus, string> = {
  ready: 'Ready',
  needs_setup: 'Needs setup',
  missing: 'Missing',
  queued: 'Queued',
  running: 'Running',
  passed: 'Passed',
  failed: 'Failed',
  blocked: 'Blocked',
  inconclusive: 'Inconclusive',
  skipped: 'Skipped',
  canceled: 'Canceled',
  recovery: 'Needs attention',
  not_started: 'Not started',
}
const STATUS_ORDER: CoverageFlowStatus[] = [
  'running',
  'failed',
  'recovery',
  'blocked',
  'inconclusive',
  'canceled',
  'passed',
  'queued',
  'not_started',
  'skipped',
  'needs_setup',
  'missing',
  'ready',
]
function caseCount(count: number) {
  return `${count} ${count === 1 ? 'case' : 'cases'}`
}
function StatusMark({
  status,
  prominent = false,
}: {
  status: CoverageFlowStatus
  prominent?: boolean
}) {
  const Icon =
    status === 'passed'
      ? Check
      : status === 'running'
        ? LoaderCircle
        : status === 'failed' ||
            status === 'blocked' ||
            status === 'inconclusive' ||
            status === 'missing' ||
            status === 'recovery'
          ? AlertTriangle
          : status === 'ready'
            ? Play
            : Circle
  return (
    <span
      className={`${styles.status} ${prominent ? styles.resultBadge : ''}`}
      data-status={status}
    >
      <Icon
        size={prominent ? 15 : 12}
        aria-hidden
        className={status === 'running' ? styles.spin : undefined}
      />
      {STATUS_LABEL[status]}
    </span>
  )
}
function handlePositions(orientation: FlowOrientation) {
  return orientation === 'vertical'
    ? { target: Position.Top, source: Position.Bottom }
    : { target: Position.Left, source: Position.Right }
}
function NodeShell({ data, className = '' }: { data: CoverageNodeData; className?: string }) {
  const handles = handlePositions(data.orientation)
  const nodeClassName = `${styles.node} ${className} ${data.orientation === 'vertical' ? styles.verticalNode : ''} ${data.finished ? styles.finishedNode : ''}`
  const body = (
    <>
      <Handle type="target" position={handles.target} isConnectable={false} />
      <div className={styles.nodeHeader}>
        <span className={styles.eyebrow}>{data.eyebrow}</span>
        {data.version != null && <span className={styles.version}>v{data.version}</span>}
      </div>
      <div className={styles.nodeTitle} title={data.title}>
        {data.title}
      </div>
      <div className={styles.nodeMeta}>
        {data.required != null && (
          <span className={data.required ? styles.required : styles.optional}>
            {data.required ? 'Required' : 'Optional'}
          </span>
        )}
        {data.detail && <span>{data.detail}</span>}
        {data.status && <StatusMark status={data.status} prominent={data.finished} />}
      </div>
      <Handle type="source" position={handles.source} isConnectable={false} />
    </>
  )
  return data.onActivate ? (
    <button
      type="button"
      className={`${nodeClassName} ${styles.actionNode}`}
      data-result={data.finished ? data.status : undefined}
      onClick={data.onActivate}
      aria-label={`${data.order ? `Case ${data.order}, ` : ''}${data.title}${data.version != null ? `, version ${data.version}` : ''}${data.required != null ? `, ${data.required ? 'required' : 'optional'}` : ''}${data.status ? `, ${STATUS_LABEL[data.status]}` : ''}. View attempt details`}
    >
      {body}
    </button>
  ) : (
    <div className={nodeClassName} data-result={data.finished ? data.status : undefined}>
      {body}
    </div>
  )
}
function RootNode({ data }: NodeProps<CoverageNode>) {
  const handles = handlePositions(data.orientation)
  return (
    <div
      className={`${styles.node} ${styles.rootNode} ${data.orientation === 'vertical' ? styles.verticalNode : ''}`}
    >
      <Handle type="source" position={handles.source} isConnectable={false} />
      <div className={styles.rootGlyph} aria-hidden>
        <Route size={17} />
      </div>
      <span className={styles.eyebrow}>{data.eyebrow}</span>
      <div className={styles.rootTitle} title={data.title}>
        {data.title}
      </div>
      <div className={styles.rootMeta}>
        {data.version != null && <span>Version {data.version}</span>}
        <span>{caseCount(data.count ?? 0)}</span>
        {data.status && <StatusMark status={data.status} />}
      </div>
    </div>
  )
}
function GroupNode({ data }: NodeProps<CoverageNode>) {
  if (!data.band) return <NodeShell data={data} className={styles.groupNode} />
  const handles = handlePositions(data.orientation)
  return (
    <div
      className={`${styles.groupBand} ${data.orientation === 'vertical' ? styles.verticalBand : ''}`}
    >
      <Handle type="target" position={handles.target} isConnectable={false} />
      <div className={styles.groupRail} aria-hidden />
      <div className={styles.groupHeader}>
        <div className={styles.nodeHeader}>
          <span className={styles.eyebrow}>{data.eyebrow}</span>
          {data.version != null && <span className={styles.version}>v{data.version}</span>}
        </div>
        <div className={styles.nodeTitle} title={data.title}>
          {data.title}
        </div>
        <div className={styles.nodeMeta}>
          {data.detail && <span>{data.detail}</span>}
          {data.status && <StatusMark status={data.status} />}
        </div>
      </div>
    </div>
  )
}
function CaseNode({ data }: NodeProps<CoverageNode>) {
  return (
    <div className={styles.caseWrap}>
      <Handle
        id="sequence-in"
        className={styles.sequenceHandle}
        type="target"
        position={Position.Top}
        isConnectable={false}
      />
      <span className={styles.ordinal} aria-hidden>
        {data.order}
      </span>
      <NodeShell data={data} className={styles.caseNode} />
      <Handle
        id="sequence-out"
        className={styles.sequenceHandle}
        type="source"
        position={Position.Bottom}
        isConnectable={false}
      />
    </div>
  )
}
const nodeTypes: NodeTypes = { root: RootNode, group: GroupNode, case: CaseNode }
const edge = (id: string, source: string, target: string, emphasized = false): Edge => ({
  id,
  source,
  target,
  type: 'smoothstep',
  animated: false,
  style: {
    stroke: emphasized ? 'var(--flow-rail-strong)' : 'var(--flow-rail)',
    strokeWidth: emphasized ? 2 : 1.4,
  },
})
const sequenceEdge = (source: string, target: string): Edge => ({
  id: `sequence-${source}-${target}`,
  source,
  target,
  sourceHandle: 'sequence-out',
  targetHandle: 'sequence-in',
  type: 'smoothstep',
  animated: false,
  markerEnd: {
    type: MarkerType.ArrowClosed,
    color: 'var(--flow-rail-strong)',
    width: 14,
    height: 14,
  },
  style: {
    stroke: 'var(--flow-rail-strong)',
    strokeWidth: 1.6,
  },
})
function caseVersion(options: LibraryOptionsResponse, id: string) {
  const value = options.saved_versions.find((version) => version.version.id === id)
  return value?.version.definition.kind === 'case' ? value : undefined
}
function suiteVersion(options: LibraryOptionsResponse, id: string) {
  const value = options.saved_versions.find((version) => version.version.id === id)
  return value?.version.definition.kind === 'suite' ? value : undefined
}
function savedStatus(value: LibraryVersionResponse | undefined): CoverageFlowStatus {
  if (!value) return 'missing'
  return value.entry.needs_setup || value.issues.length || value.coverage.issues.length
    ? 'needs_setup'
    : 'ready'
}
function sameSelection(
  previous: { definitionId: string; required: boolean; dataVariant: string },
  value: LibraryVersionResponse,
  selection: CaseSelection,
) {
  return (
    previous.definitionId === value.version.id &&
    previous.required === selection.required &&
    previous.dataVariant === selection.data_variant
  )
}
type SeenCase = {
  definitionId: string
  required: boolean
  dataVariant: string
  node: CoverageNode
  ownerId: string
}

/** Build a deterministic presentation model using the server's logical-case conflict rules. */
export function buildLibraryCoverageFlow(
  definition: LibraryDraftDefinition,
  options: LibraryOptionsResponse,
  orientation: FlowOrientation = 'horizontal',
): CoverageFlowModel {
  if (definition.kind === 'case')
    return { nodes: [], edges: [], itemCount: 0, orientation, contentHeight: 0, contentWidth: 0 }
  const rootId = 'coverage-root'
  const root: CoverageNode = {
    id: rootId,
    type: 'root',
    position: { x: orientation === 'vertical' ? 28 : 38, y: orientation === 'vertical' ? 20 : 0 },
    draggable: false,
    connectable: false,
    data: {
      kind: 'root',
      orientation,
      eyebrow: definition.kind === 'suite' ? 'Reusable suite' : 'Release check',
      title:
        definition.content.title ||
        (definition.kind === 'suite' ? 'Untitled suite' : 'Untitled release check'),
      version: definition.content.version,
      count: 0,
      status: 'ready',
    },
  }
  const nodes: CoverageNode[] = [root]
  const edges: Edge[] = []
  const seen = new Map<string, SeenCase>()
  const attentionOwners = new Set<string>()
  let order = 0
  let horizontalLane = 0
  let verticalY = 190
  const addCase = (
    selection: CaseSelection,
    ownerId: string,
    localIndex: number,
    nested: boolean,
  ) => {
    const value = caseVersion(options, selection.case_version_id)
    const logicalKey =
      value?.version.definition.kind === 'case'
        ? value.version.definition.content.key
        : `missing:${selection.case_version_id}`
    const previous = seen.get(logicalKey)
    if (previous && value) {
      if (!sameSelection(previous, value, selection)) {
        previous.node.data.status = 'needs_setup'
        previous.node.data.detail = 'Conflicting selection'
        attentionOwners.add(previous.ownerId)
        attentionOwners.add(ownerId)
      }
      return undefined
    }
    if (previous) return undefined
    order += 1
    const nodeId = `case-${order}-${selection.case_version_id}`
    const status = savedStatus(value)
    const position = nested
      ? orientation === 'vertical'
        ? { x: 14, y: 82 + localIndex * 114 }
        : { x: 318, y: 48 + localIndex * 104 }
      : orientation === 'vertical'
        ? { x: 28, y: verticalY + localIndex * 114 }
        : { x: 470, y: horizontalLane * 104 }
    const node: CoverageNode = {
      id: nodeId,
      type: 'case',
      position,
      parentId: nested ? ownerId : undefined,
      extent: nested ? 'parent' : undefined,
      draggable: false,
      connectable: false,
      ariaLabel: `Case ${order}, ${value?.version.definition.content.title ?? 'missing saved case'}${value?.version.definition.content.version != null ? `, version ${value.version.definition.content.version}` : ''}, ${selection.required ? 'required' : 'optional'}, ${STATUS_LABEL[status]}`,
      data: {
        kind: 'case',
        orientation,
        eyebrow: `Step ${order}`,
        title: value?.version.definition.content.title ?? 'Unavailable saved case',
        version: value?.version.definition.content.version,
        order,
        required: selection.required,
        status,
        detail: selection.data_variant === 'default' ? undefined : selection.data_variant,
      },
    }
    nodes.push(node)
    seen.set(logicalKey, {
      definitionId: value?.version.id ?? selection.case_version_id,
      required: selection.required,
      dataVariant: selection.data_variant,
      node,
      ownerId,
    })
    if (status !== 'ready') attentionOwners.add(ownerId)
    horizontalLane += nested ? 0 : 1
    return nodeId
  }
  if (definition.kind === 'suite') {
    let localIndex = 0
    let previousCaseId: string | undefined
    definition.content.cases.forEach((selection) => {
      const nodeId = addCase(selection, rootId, localIndex, false)
      if (!nodeId) return
      edges.push(
        previousCaseId
          ? sequenceEdge(previousCaseId, nodeId)
          : edge(`edge-${rootId}-${nodeId}`, rootId, nodeId, true),
      )
      previousCaseId = nodeId
      localIndex += 1
    })
    verticalY += localIndex * 114
  } else {
    const groups: Array<{
      id: string
      eyebrow: string
      title: string
      version?: number
      status: CoverageFlowStatus
      selections: CaseSelection[]
    }> = []
    // The API manifest resolves direct selections before expanding pinned suites.
    if (definition.content.cases.length)
      groups.push({
        id: 'direct-cases',
        eyebrow: 'Outside suites',
        title: 'Direct cases',
        status: 'ready',
        selections: definition.content.cases,
      })
    groups.push(
      ...definition.content.suite_version_ids.map((id, index) => {
        const value = suiteVersion(options, id)
        return {
          id: `suite-${index}-${id}`,
          eyebrow: `Suite ${index + 1}`,
          title: value?.version.definition.content.title ?? 'Unavailable saved suite',
          version: value?.version.definition.content.version,
          status: savedStatus(value),
          selections:
            value?.version.definition.kind === 'suite'
              ? value.version.definition.content.cases
              : [],
        }
      }),
    )
    groups.forEach((group) => {
      const groupNode: CoverageNode = {
        id: group.id,
        type: 'group',
        position: {
          x: orientation === 'vertical' ? 14 : 360,
          y: orientation === 'vertical' ? verticalY : horizontalLane * 104,
        },
        style: { width: orientation === 'vertical' ? 324 : 690, height: 112 },
        draggable: false,
        connectable: false,
        data: {
          kind: 'group',
          orientation,
          eyebrow: group.eyebrow,
          title: group.title,
          version: group.version,
          count: 0,
          detail: caseCount(0),
          status: group.status,
          band: true,
        },
      }
      nodes.push(groupNode)
      edges.push(edge(`edge-${rootId}-${group.id}`, rootId, group.id, true))
      const before = order
      let localIndex = 0
      let previousCaseId: string | undefined
      group.selections.forEach((selection) => {
        const nodeId = addCase(selection, group.id, localIndex, true)
        if (!nodeId) return
        if (previousCaseId) edges.push(sequenceEdge(previousCaseId, nodeId))
        previousCaseId = nodeId
        localIndex += 1
      })
      const uniqueCount = order - before
      const groupHeight =
        orientation === 'vertical'
          ? Math.max(112, 94 + uniqueCount * 114)
          : Math.max(112, 58 + uniqueCount * 104)
      groupNode.style = { ...groupNode.style, height: groupHeight }
      groupNode.data.count = uniqueCount
      groupNode.data.detail =
        uniqueCount === 0 && group.selections.length
          ? 'Resolved in an earlier group'
          : caseCount(uniqueCount)
      if (group.status !== 'ready') attentionOwners.add(group.id)
      if (orientation === 'vertical') verticalY += groupHeight + 22
      else horizontalLane += Math.max(uniqueCount, 1) + 0.3
    })
  }
  const caseNodes = nodes.filter((node) => node.type === 'case')
  const membershipNeedsSetup =
    order === 0 || order > 100 || !caseNodes.some((node) => node.data.required)
  const planProfileNeedsSetup =
    definition.kind === 'plan' &&
    (!definition.content.profile_id ||
      !options.profiles.some(
        (profile) => profile.id === definition.content.profile_id && profile.qualified,
      ))
  if (
    attentionOwners.size ||
    membershipNeedsSetup ||
    planProfileNeedsSetup ||
    !definition.content.title.trim()
  )
    root.data.status = 'needs_setup'
  nodes.forEach((node) => {
    if (attentionOwners.has(node.id) && node.data.status === 'ready')
      node.data.status = 'needs_setup'
    if (node.type === 'case')
      node.ariaLabel = `Case ${node.data.order}, ${node.data.title}${node.data.version != null ? `, version ${node.data.version}` : ''}, ${node.data.required ? 'required' : 'optional'}, ${STATUS_LABEL[node.data.status ?? 'ready']}`
    if (node.type === 'group')
      node.ariaLabel = `${node.data.eyebrow}, ${node.data.title}${node.data.version != null ? `, version ${node.data.version}` : ''}, ${node.data.detail ?? caseCount(node.data.count ?? 0)}${node.data.status ? `, ${STATUS_LABEL[node.data.status]}` : ''}`
  })
  root.data.count = order
  root.ariaLabel = `${definition.kind === 'suite' ? 'Suite' : 'Release check'}, ${root.data.title}, version ${definition.content.version}, ${caseCount(order)}, ${STATUS_LABEL[root.data.status ?? 'ready']}`
  if (orientation === 'horizontal')
    root.position = { x: 38, y: Math.max(0, (Math.max(horizontalLane, 1) * 104 - 126) / 2) }
  return {
    nodes,
    edges,
    itemCount: order,
    orientation,
    contentHeight:
      orientation === 'vertical'
        ? Math.max(360, verticalY + 30)
        : Math.max(310, horizontalLane * 104 + 30),
    contentWidth: orientation === 'vertical' ? 360 : definition.kind === 'plan' ? 1080 : 830,
  }
}

function attemptStatus(
  attempt: AttemptResponse | undefined,
  runState: JobState,
): CoverageFlowStatus {
  // An outcome can arrive before cleanup; only the finished state is a final result.
  if (attempt?.state === 'finished') return attempt.outcome ?? 'inconclusive'
  if (
    attempt?.state === 'leased' ||
    attempt?.state === 'running' ||
    attempt?.state === 'finalizing'
  )
    return 'running'
  if (attempt?.state === 'recovery_required') return 'recovery'
  if (attempt?.state === 'cancel_requested') return 'canceled'
  if (
    runState === 'finished' ||
    runState === 'recovery_required' ||
    runState === 'cancel_requested'
  )
    return 'not_started'
  return 'queued'
}
export function buildRunCoverageFlow(
  run: RunResponse,
  activate?: (attemptId: string) => void,
  orientation: FlowOrientation = 'horizontal',
): CoverageFlowModel {
  const rootId = 'run-root'
  const groupId = 'run-sequence'
  const itemCount = run.manifest.cases.length
  const groupHeight =
    orientation === 'vertical'
      ? Math.max(112, 94 + itemCount * 114)
      : Math.max(112, 58 + itemCount * 104)
  const latestAttempts = new Map<string, AttemptResponse>()
  run.attempts.forEach((attempt) => {
    const current = latestAttempts.get(attempt.case_version_id)
    if (!current || attempt.number > current.number)
      latestAttempts.set(attempt.case_version_id, attempt)
  })
  const nodes: CoverageNode[] = [
    {
      id: rootId,
      type: 'root',
      position: {
        x: orientation === 'vertical' ? 28 : 38,
        y: orientation === 'vertical' ? 20 : Math.max(0, (groupHeight - 126) / 2),
      },
      draggable: false,
      connectable: false,
      ariaLabel: `Run, ${run.summary}, ${caseCount(itemCount)}`,
      data: {
        kind: 'root',
        orientation,
        eyebrow: 'Frozen run',
        title: run.summary,
        count: itemCount,
      },
    },
    {
      id: groupId,
      type: 'group',
      position: {
        x: orientation === 'vertical' ? 14 : 360,
        y: orientation === 'vertical' ? 190 : 0,
      },
      style: { width: orientation === 'vertical' ? 324 : 690, height: groupHeight },
      draggable: false,
      connectable: false,
      ariaLabel: `Execution order, ${caseCount(itemCount)}`,
      data: {
        kind: 'group',
        orientation,
        eyebrow: 'Manifest',
        title: 'Execution order',
        detail: caseCount(itemCount),
        band: true,
      },
    },
  ]
  const edges: Edge[] = [edge(`edge-${rootId}-${groupId}`, rootId, groupId, true)]
  let previousCaseId: string | undefined
  run.manifest.cases.forEach((resolved, index) => {
    const attempt = latestAttempts.get(resolved.definition_id)
    const status = attemptStatus(attempt, run.state)
    const nodeId = `run-case-${index}-${resolved.definition_id}`
    nodes.push({
      id: nodeId,
      type: 'case',
      parentId: groupId,
      extent: 'parent',
      position:
        orientation === 'vertical'
          ? { x: 14, y: 82 + index * 114 }
          : { x: 318, y: 48 + index * 104 },
      draggable: false,
      connectable: false,
      focusable: !attempt,
      ariaLabel: `Case ${index + 1}, ${resolved.case.title}, version ${resolved.case.version}, ${resolved.required ? 'required' : 'optional'}, ${STATUS_LABEL[status]}`,
      data: {
        kind: 'case',
        orientation,
        eyebrow: `Step ${index + 1}`,
        title: resolved.case.title,
        version: resolved.case.version,
        order: index + 1,
        required: resolved.required,
        status,
        finished: attempt?.state === 'finished',
        detail: resolved.data_variant === 'default' ? undefined : resolved.data_variant,
        onActivate: attempt && activate ? () => activate(attempt.id) : undefined,
      },
    })
    if (previousCaseId) edges.push(sequenceEdge(previousCaseId, nodeId))
    previousCaseId = nodeId
  })
  return {
    nodes,
    edges,
    itemCount,
    orientation,
    contentHeight: orientation === 'vertical' ? 190 + groupHeight + 30 : groupHeight + 30,
    contentWidth: orientation === 'vertical' ? 360 : 1080,
  }
}

function FlowCanvas({
  model,
  label,
  editor,
}: {
  model: CoverageFlowModel
  label: string
  editor?: ReactNode
}) {
  const height = Math.min(680, Math.max(360, model.contentHeight))
  const translateExtent: CoordinateExtent = [
    [-80, -80],
    [model.contentWidth + 80, model.contentHeight + 80],
  ]
  return (
    <div className={styles.canvas} aria-label={label}>
      {editor && <div className={`${styles.canvasEditor} nodrag nopan nowheel`}>{editor}</div>}
      <div className={styles.flowViewport} style={{ height }}>
        <ReactFlow
          key={model.orientation}
          nodes={model.nodes}
          edges={model.edges}
          nodeTypes={nodeTypes}
          nodesConnectable={false}
          nodesDraggable={false}
          elementsSelectable
          fitView={model.orientation === 'horizontal'}
          defaultViewport={{ x: 0, y: 0, zoom: 1 }}
          fitViewOptions={{ padding: 0.12, minZoom: 0.72, maxZoom: 1 }}
          minZoom={0.6}
          maxZoom={1.2}
          panOnScroll={false}
          zoomOnScroll={false}
          preventScrolling={false}
          translateExtent={translateExtent}
          proOptions={{ hideAttribution: true }}
        >
          <Background color="var(--flow-grid)" gap={22} size={1} />
          <Controls showInteractive={false} position="bottom-right" />
        </ReactFlow>
      </div>
    </div>
  )
}

const choiceLabel = (value: LibraryVersionResponse) =>
  `${value.version.definition.content.title} · v${value.version.definition.content.version}`

function CoverageCanvasEditor({
  definition,
  options,
  onAddCase,
  onAddSuite,
}: {
  definition: Exclude<LibraryDraftDefinition, { kind: 'case' }>
  options: LibraryOptionsResponse
  onAddCase?: (caseVersionId: string) => void
  onAddSuite?: (suiteVersionId: string) => void
}) {
  const selectedCaseIds = new Set(definition.content.cases.map((item) => item.case_version_id))
  const selectedSuiteIds = new Set(
    definition.kind === 'plan' ? definition.content.suite_version_ids : [],
  )
  const cases = options.saved_versions.filter(
    (value) => value.version.definition.kind === 'case' && !selectedCaseIds.has(value.version.id),
  )
  const suites = options.saved_versions.filter(
    (value) => value.version.definition.kind === 'suite' && !selectedSuiteIds.has(value.version.id),
  )
  const selectionCount =
    definition.content.cases.length +
    (definition.kind === 'plan' ? definition.content.suite_version_ids.length : 0)
  const full = selectionCount >= 100
  return (
    <Stack gap={6}>
      <Group gap={6}>
        <Plus size={14} aria-hidden />
        <Text size="xs" fw={700} tt="uppercase" lts="0.08em">
          Add to sequence
        </Text>
      </Group>
      {definition.kind === 'plan' && onAddSuite && (
        <Select
          label="Saved suite"
          aria-label="Add saved suite to sequence"
          size="xs"
          searchable
          disabled={full || !suites.length}
          placeholder={suites.length ? 'Choose suite' : 'No more saved suites'}
          data={suites.map((value) => ({ value: value.version.id, label: choiceLabel(value) }))}
          value={null}
          onChange={(id) => {
            if (id) onAddSuite(id)
          }}
        />
      )}
      {onAddCase && (
        <Select
          label="Saved case"
          aria-label="Add saved case to sequence"
          size="xs"
          searchable
          disabled={full || !cases.length}
          placeholder={cases.length ? 'Choose case' : 'Save another case first'}
          data={cases.map((value) => ({ value: value.version.id, label: choiceLabel(value) }))}
          value={null}
          onChange={(id) => {
            if (id) onAddCase(id)
          }}
        />
      )}
      <Text size="xs" c="dimmed">
        New items become the next numbered step.
      </Text>
    </Stack>
  )
}
function modelStatuses(model: CoverageFlowModel) {
  const present = new Set(
    model.nodes
      .map((node) => node.data.status)
      .filter((status): status is CoverageFlowStatus => status != null),
  )
  return STATUS_ORDER.filter((status) => present.has(status))
}
function FlowLegend({ statuses }: { statuses: CoverageFlowStatus[] }) {
  return (
    <div className={styles.legend} aria-label="Flow status legend">
      {statuses.map((status) => (
        <StatusMark key={status} status={status} />
      ))}
    </div>
  )
}
export function LibraryCoverageFlow({
  definition,
  options,
  frozen = false,
  onAddCase,
  onAddSuite,
  runControl,
}: {
  definition: LibraryDraftDefinition
  options: LibraryOptionsResponse
  frozen?: boolean
  onAddCase?: (caseVersionId: string) => void
  onAddSuite?: (suiteVersionId: string) => void
  runControl?: ReactNode
}) {
  const narrow = useMediaQuery('(max-width: 48em)')
  const orientation: FlowOrientation = narrow ? 'vertical' : 'horizontal'
  const model = useMemo(
    () => buildLibraryCoverageFlow(definition, options, orientation),
    [definition, options, orientation],
  )
  if (definition.kind === 'case') return null
  return (
    <Card className={styles.surface} padding={0}>
      <div className={styles.surfaceHeader}>
        <Stack gap={3}>
          <Group gap="xs">
            <Layers3 size={16} aria-hidden />
            <Text className={styles.kicker}>
              {frozen ? 'Pinned coverage map' : 'Live coverage map'}
            </Text>
          </Group>
          <Title order={2} size="h3">
            {definition.kind === 'plan' ? 'Release sequence' : 'Suite sequence'}
          </Title>
          <Text size="sm" c="dimmed">
            {frozen
              ? 'This map reflects exact saved versions. Lines show containment and order, never test dependencies.'
              : 'Add coverage on the canvas. Numbered arrows show run order; every case still starts from a clean state.'}
          </Text>
        </Stack>
        <div className={styles.surfaceActions}>
          <Badge variant="outline">
            {model.itemCount} unique {model.itemCount === 1 ? 'case' : 'cases'}
          </Badge>
          {runControl}
        </div>
      </div>
      <FlowLegend statuses={modelStatuses(model)} />
      <FlowCanvas
        model={model}
        label={`${definition.kind === 'plan' ? 'Release' : 'Suite'} coverage flow`}
        editor={
          !frozen && (onAddCase || onAddSuite) ? (
            <CoverageCanvasEditor
              definition={definition}
              options={options}
              onAddCase={onAddCase}
              onAddSuite={onAddSuite}
            />
          ) : undefined
        }
      />
    </Card>
  )
}
export function RunCoverageFlow({ run }: { run: RunResponse }) {
  const narrow = useMediaQuery('(max-width: 48em)')
  const orientation: FlowOrientation = narrow ? 'vertical' : 'horizontal'
  const model = useMemo(
    () => buildRunCoverageFlow(run, focusAttempt, orientation),
    [run, orientation],
  )
  return (
    <Card className={styles.surface} padding={0}>
      <div className={styles.surfaceHeader}>
        <Stack gap={3}>
          <Group gap="xs">
            <Route size={16} aria-hidden />
            <Text className={styles.kicker}>Live execution map</Text>
          </Group>
          <Title order={2} size="h3">
            Run suite
          </Title>
          <Text size="sm" c="dimmed">
            Real manifest order and latest attempt state. Select a started case to jump to its
            evidence.
          </Text>
        </Stack>
        <Badge variant="outline">{run.state.replaceAll('_', ' ')}</Badge>
      </div>
      <FlowLegend statuses={modelStatuses(model)} />
      <FlowCanvas model={model} label="Run execution flow" />
    </Card>
  )
}

export function focusAttempt(attemptId: string) {
  const target = document.getElementById(`attempt-${attemptId}`)
  target?.scrollIntoView({ behavior: 'smooth', block: 'start' })
  target?.focus({ preventScroll: true })
}
