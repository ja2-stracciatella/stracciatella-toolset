import {
  ArrowLeftOutlined,
  RedoOutlined,
  UndoOutlined,
} from '@ant-design/icons';
import { Alert, Button, Flex, Segmented, Spin, Typography } from 'antd';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  attach,
  createMap,
  DEFAULT_RENDER_OPTIONS,
  detach,
  isEditableTarget,
  loadMap,
  MapRenderOptions,
  MapViewerStatus,
  NewMapOptions,
  redo,
  saveMap,
  setRenderOptions,
  subscribeEditorEvents,
  subscribeStatus,
  undo,
} from '../../lib/mapViewer';
import { HelpIcon } from '../common/HelpIcon';
import { EditorContent, SaveButton } from '../layout/EditorContent';
import {
  EditorModeName,
  MapEditorSidebar,
  MODE_OPTIONS,
} from './mapEditor/MapEditorSidebar';

const INSTRUCTIONS = [
  'Each mode edits a part of the map with its own tools, the question mark next to the tools ' +
    'describes the chosen tool.',
  'Move with W / A / S / D or by moving the mouse to the edge of the map. Zoom with the mouse ' +
    'wheel or + / -. Page Up and Page Down switch the floor.',
  'Undo with Ctrl+Z, redo with Ctrl+Y or Ctrl+Shift+Z and save with Ctrl+S. F3 toggles the ' +
    'debug overlay.',
];

export interface TacticalMapViewerProps {
  file: string;
  /** Creates a new map that is saved to `file` instead of loading `file` */
  newMap?: NewMapOptions;
  onBack: () => unknown;
}

/** Shows a tactical map in the wasm map viewer, remount it (via `key`) to show another map */
export function TacticalMapViewer({
  file,
  newMap,
  onBack,
}: TacticalMapViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [status, setStatus] = useState<MapViewerStatus>(() => ({
    state: 'loading',
    map: file,
  }));
  const [mode, setMode] = useState<EditorModeName>('view');
  const [startError, setStartError] = useState<Error | null>(null);
  const [renderOptions, setRenderOptionsState] = useState<MapRenderOptions>(
    DEFAULT_RENDER_OPTIONS,
  );
  // The options to send after loading the map, without loading it again when they change
  const renderOptionsRef = useRef(renderOptions);
  const loadedRef = useRef(false);
  // The revision of the editor state and the one that was saved last, the map is loaded with 0.
  // A new map was never saved, so it can be saved right away.
  const [revision, setRevision] = useState(0);
  const [savedRevision, setSavedRevision] = useState(newMap ? -1 : 0);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<Error | null>(null);
  const [canUndo, setCanUndo] = useState(false);
  const [canRedo, setCanRedo] = useState(false);

  useEffect(() => {
    return subscribeEditorEvents((event) => {
      if (event.type === 'state' && event.map === file) {
        setRevision(event.revision);
        setCanUndo(event.canUndo);
        setCanRedo(event.canRedo);
      }
    });
  }, [file]);

  const onSave = useCallback(async () => {
    setSaving(true);
    setSaveError(null);
    try {
      // Edits made while saving stay modified
      setSavedRevision(await saveMap(file));
    } catch (e: unknown) {
      setSaveError(e instanceof Error ? e : new Error(String(e)));
    } finally {
      setSaving(false);
    }
  }, [file]);

  useEffect(() => {
    return subscribeStatus((s) => {
      // Ignore statuses of maps that were requested before
      if (s.map === file) {
        setStatus(s);
      }
    });
  }, [file]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }
    attach(container)
      .then(() => (newMap ? createMap(file, newMap) : loadMap(file)))
      .then(() => {
        loadedRef.current = true;
        return setRenderOptions(renderOptionsRef.current);
      })
      .catch((e: unknown) => {
        setStartError(e instanceof Error ? e : new Error(String(e)));
      });
    return () => {
      loadedRef.current = false;
      detach();
    };
  }, [file, newMap]);

  const onRenderOptionsChange = useCallback((options: MapRenderOptions) => {
    renderOptionsRef.current = options;
    setRenderOptionsState(options);
    // Before the map is loaded, the options are sent after loading it
    if (loadedRef.current) {
      setRenderOptions(options).catch((e: unknown) => {
        setStartError(e instanceof Error ? e : new Error(String(e)));
      });
    }
  }, []);

  const onUndo = useCallback(() => {
    undo().catch((e: unknown) => {
      setStartError(e instanceof Error ? e : new Error(String(e)));
    });
  }, []);
  const onRedo = useCallback(() => {
    redo().catch((e: unknown) => {
      setStartError(e instanceof Error ? e : new Error(String(e)));
    });
  }, []);

  // Ctrl+Z undoes and Ctrl+Y or Ctrl+Shift+Z redoes anywhere on the page, except while typing,
  // where they change the text
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey || isEditableTarget(e.target)) {
        return;
      }
      const key = e.key.toLowerCase();
      if (key === 'z' && !e.shiftKey) {
        e.preventDefault();
        onUndo();
      } else if ((key === 'z' && e.shiftKey) || (key === 'y' && !e.shiftKey)) {
        e.preventDefault();
        onRedo();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onRedo, onUndo]);

  const flexStyle = useMemo(() => ({ height: '100%' }) as const, []);
  const bodyStyle = useMemo(() => ({ flexGrow: 1, minHeight: 0 }) as const, []);
  const wrapperStyle = useMemo(
    () =>
      ({
        position: 'relative',
        flexGrow: 1,
        minHeight: '300px',
        overflow: 'hidden',
      }) as const,
    [],
  );
  const containerStyle = useMemo(
    () => ({ position: 'absolute', inset: 0 }) as const,
    [],
  );
  // Translucent, so the progress shown by the viewer itself stays visible
  const overlayStyle = useMemo(
    () =>
      ({
        position: 'absolute',
        inset: 0,
        zIndex: 1,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: '#FFF',
      }) as const,
    [],
  );
  const loading = status.state === 'loading' && !startError;
  const editable = status.state === 'loaded' && !startError;
  const modified = editable && revision !== savedRevision;
  const alert = useMemo(() => {
    if (startError) {
      return (
        <Alert
          type="error"
          message="Could not start the map viewer"
          description={startError.message}
        />
      );
    }
    if (status.state === 'error') {
      return (
        <Alert
          type="error"
          message={`Could not load ${file}`}
          description={status.message}
        />
      );
    }
    return null;
  }, [file, startError, status]);

  const header = useMemo(
    () => (
      <Flex gap="small" align="center">
        <SaveButton
          modified={modified}
          saving={saving}
          error={saveError}
          onSave={onSave}
        />
        <Button
          aria-label="Undo"
          title="Undo (Ctrl + Z)"
          disabled={!editable || !canUndo}
          onClick={onUndo}
        >
          <UndoOutlined />
        </Button>
        <Button
          aria-label="Redo"
          title="Redo (Ctrl + Y)"
          disabled={!editable || !canRedo}
          onClick={onRedo}
        >
          <RedoOutlined />
        </Button>
      </Flex>
    ),
    [
      canRedo,
      canUndo,
      editable,
      modified,
      onRedo,
      onSave,
      onUndo,
      saveError,
      saving,
    ],
  );

  return (
    <EditorContent header={header}>
      <Flex vertical gap="small" style={flexStyle}>
        <Flex gap="middle" align="center">
          <Button icon={<ArrowLeftOutlined />} onClick={onBack}>
            Back to overview
          </Button>
          <Typography.Title level={4} style={{ margin: 0 }}>
            {savedRevision < 0 ? `${file} (new)` : file}
          </Typography.Title>
        </Flex>
        {alert}
        <Flex gap="small" align="center">
          <Segmented<EditorModeName>
            options={MODE_OPTIONS}
            value={mode}
            onChange={setMode}
            aria-label="mode"
          />
          <HelpIcon
            title="Map editor"
            paragraphs={INSTRUCTIONS}
            label="usage"
          />
        </Flex>
        <Flex gap="middle" style={bodyStyle}>
          {/* The canvas is appended to the container outside of React, so the overlay is a sibling */}
          <div style={wrapperStyle}>
            <div ref={containerRef} style={containerStyle} />
            {loading ? (
              <div style={overlayStyle} role="status" aria-label="loading map">
                <Spin size="large" />
              </div>
            ) : null}
          </div>
          <MapEditorSidebar
            file={file}
            mode={mode}
            renderOptions={renderOptions}
            onRenderOptionsChange={onRenderOptionsChange}
          />
        </Flex>
      </Flex>
    </EditorContent>
  );
}
