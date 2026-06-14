import { useRef, useState, type JSX } from 'react';
import type { Prompt } from '@velata/core';
import type { SettingsApi } from '../hooks.js';
import { HotkeyRecorder } from '../components/HotkeyRecorder.js';
import { filterPrompts } from '../transformView.js';

/** Collapses whitespace and truncates a multi-line instruction for the card. */
function preview(instruction: string): string {
  const oneLine = instruction.replace(/\s+/g, ' ').trim();
  if (oneLine === '') return 'Acts like Polish — fix grammar and spelling.';
  return oneLine.length > 90 ? `${oneLine.slice(0, 90)}…` : oneLine;
}

/** Identifies the create card while open, so it never collides with a real id. */
const NEW_ID = 'new';

/**
 * Transform page: one uniform card per prompt in `settings.prompts`, built-in
 * Polish and custom alike, in creation order. Each prompt rewrites the current
 * selection through the active AI profile — no voice. The header's See-changes
 * recorder edits the same `changeOverlayHotkey` as Settings → Dictation, so the
 * two stay in sync through the settings subscription.
 */
export function TransformsTab({ api }: { api: SettingsApi }): JSX.Element {
  const { settings, update } = api;
  const [query, setQuery] = useState('');
  // Which card is in its editor: a prompt id, NEW_ID for the create card, or
  // null when none is open.
  const [editId, setEditId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');
  const [editInstruction, setEditInstruction] = useState('');
  const [editShortcut, setEditShortcut] = useState('');
  const listRef = useRef<HTMLDivElement>(null);

  const matches = filterPrompts(settings.prompts, query);
  const hasCustom = settings.prompts.some((p) => !p.builtIn);

  const patchPrompt = (id: string, patch: Partial<Prompt>): void => {
    void update({
      prompts: settings.prompts.map((p) => (p.id === id ? { ...p, ...patch } : p)),
    });
  };

  const removePrompt = (id: string): void => {
    if (editId === id) cancelEdit();
    void update({ prompts: settings.prompts.filter((p) => p.id !== id) });
  };

  const startEdit = (prompt: Prompt): void => {
    setEditId(prompt.id);
    setEditName(prompt.name);
    setEditInstruction(prompt.instruction);
    setEditShortcut(prompt.shortcut);
  };

  const startCreate = (): void => {
    setEditId(NEW_ID);
    setEditName('');
    setEditInstruction('');
    setEditShortcut('');
  };

  // Leaving the editor unmounts the focused control; parking focus on the list
  // container keeps keyboard users in the list instead of dropping to <body>.
  const cancelEdit = (): void => {
    setEditId(null);
    listRef.current?.focus();
  };

  const saveEdit = (): void => {
    if (editId === null) return;
    const name = editName.trim();
    if (editId === NEW_ID) {
      const prompt: Prompt = {
        id: crypto.randomUUID(),
        name: name === '' ? 'New prompt' : name,
        instruction: editInstruction,
        shortcut: editShortcut,
        builtIn: false,
      };
      void update({ prompts: [...settings.prompts, prompt] });
    } else {
      const existing = settings.prompts.find((p) => p.id === editId);
      // The built-in keeps its fixed name; only custom prompts adopt the field.
      const nextName = existing?.builtIn ? existing.name : name === '' ? 'New prompt' : name;
      patchPrompt(editId, {
        name: nextName,
        instruction: editInstruction,
        shortcut: editShortcut,
      });
    }
    // Clear the search so a just-created or renamed prompt isn't hidden by an
    // active filter that no longer matches it.
    setQuery('');
    setEditId(null);
    listRef.current?.focus();
  };

  const renderEditor = (builtIn: boolean): JSX.Element => (
    <div className="transform-card transform-card-editing">
      <label className="transform-field">
        <span className="transform-field-label">Name</span>
        <input
          type="text"
          autoFocus={!builtIn}
          maxLength={40}
          value={editName}
          disabled={builtIn}
          placeholder="Name"
          aria-label="Prompt name"
          onChange={(e) => {
            setEditName(e.target.value);
          }}
        />
      </label>
      <div className="transform-field">
        <span className="transform-field-label">Shortcut</span>
        <HotkeyRecorder
          value={editShortcut}
          label={editName || 'Prompt'}
          onChange={setEditShortcut}
        />
      </div>
      <label className="transform-field">
        <span className="transform-field-label">Prompt</span>
        <textarea
          className="transform-instruction"
          rows={3}
          maxLength={2000}
          value={editInstruction}
          // The built-in's name is read-only, so focus the first editable field.
          autoFocus={builtIn}
          placeholder="How should this rewrite the selection? (leave empty to act like Polish)"
          aria-label="Prompt instruction"
          onChange={(e) => {
            setEditInstruction(e.target.value);
          }}
        />
      </label>
      <div className="transform-card-actions">
        <button className="btn btn-primary" onClick={saveEdit}>
          Save
        </button>
        <button className="btn btn-quiet" onClick={cancelEdit}>
          Cancel
        </button>
      </div>
    </div>
  );

  return (
    <div className="tab-body">
      <header className="transform-header">
        <div className="transform-header-text">
          <h1 className="transform-title">Transform</h1>
          <p className="row-hint">
            Rewrite text with a shortcut, or run one automatically after you dictate (set on the HUD
            circle).
          </p>
        </div>
        <label className="transform-see-changes">
          <span className="row-hint">See changes</span>
          <HotkeyRecorder
            value={settings.changeOverlayHotkey}
            label="See changes"
            onChange={(accelerator) => void update({ changeOverlayHotkey: accelerator })}
          />
        </label>
      </header>

      <div className="transform-toolbar">
        <input
          type="search"
          className="transform-search"
          placeholder="Search prompts"
          aria-label="Search prompts"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
          }}
        />
        <button className="btn btn-primary" onClick={startCreate}>
          + New
        </button>
      </div>

      <div className="transform-list" ref={listRef} tabIndex={-1}>
        {matches.map((p) =>
          editId === p.id ? (
            <div key={`edit-${p.id}`}>{renderEditor(p.builtIn)}</div>
          ) : (
            <div key={p.id} className="transform-card">
              <div className="transform-card-main">
                <div className="transform-card-head">
                  <span className="transform-card-name">
                    {p.builtIn && (
                      <span
                        className="transform-builtin"
                        role="img"
                        aria-label="Built-in"
                        title="Built-in"
                      >
                        ✦
                      </span>
                    )}
                    {p.name}
                  </span>
                  <HotkeyRecorder
                    value={p.shortcut}
                    label={p.name}
                    emptyLabel="+ Add shortcut"
                    onChange={(shortcut) => {
                      patchPrompt(p.id, { shortcut });
                    }}
                  />
                </div>
                <span className="transform-preview">{preview(p.instruction)}</span>
              </div>
              <div className="transform-card-actions">
                <button
                  className="btn btn-quiet"
                  onClick={() => {
                    startEdit(p);
                  }}
                >
                  Edit
                </button>
                {!p.builtIn && (
                  <button
                    className="btn btn-quiet"
                    aria-label={`Delete ${p.name}`}
                    onClick={() => {
                      removePrompt(p.id);
                    }}
                  >
                    ×
                  </button>
                )}
              </div>
            </div>
          ),
        )}

        {editId === NEW_ID && <div key="edit-new">{renderEditor(false)}</div>}

        {matches.length === 0 &&
          editId !== NEW_ID &&
          (query.trim() === '' ? (
            <p className="row-hint">No prompts yet.</p>
          ) : (
            <p className="row-hint">No matches.</p>
          ))}
      </div>

      {!hasCustom && editId !== NEW_ID && query.trim() === '' && (
        <div className="transform-empty">
          <button className="btn" onClick={startCreate}>
            + Create your prompt
          </button>
          <p className="row-hint">Saved prompts each get their own shortcut.</p>
        </div>
      )}
    </div>
  );
}
