//! Webstorm-style "Run Configurations" state.
//!
//! Holds the per-workspace UI state (active label + pinned order) for the
//! titlebar dropdown. Script definitions themselves live in `.zed/tasks.json`
//! (the existing `TaskTemplate` schema). This module:
//!
//! * Persists `active` + `pinned` to `.zed/run_configurations.json`.
//! * Wraps start / stop / add operations using existing task + terminal infra.
//! * Notifies observers (the titlebar) whenever active or running state changes
//!   so the play/stop glyph stays in sync.

use anyhow::Context as _;
use fs::Fs;
use gpui::{App, AppContext as _, Context, Entity, SharedString, Subscription, Task, WeakEntity, Window};
use paths::local_run_configurations_file_relative_path;
use project::{Project, TaskSourceKind, WorktreeId};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};
use task::{RevealStrategy, TaskContext, TaskTemplate, TaskTemplates, TaskVariables, VariableName};
use std::time::Duration;
use terminal::TaskStatus;
use terminal_view::terminal_panel::TerminalPanel;
use util::{ResultExt, rel_path::RelPath};
use workspace::Workspace;
use zed_actions::PinnedRunConfigurations;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct PersistedState {
    #[serde(default)]
    active: Option<String>,
    #[serde(default)]
    pinned: Vec<String>,
}

pub struct RunConfigurations {
    workspace: WeakEntity<Workspace>,
    active: Option<SharedString>,
    pinned: Vec<SharedString>,
    /// Cached list of `(worktree_id, template)` populated by reload_templates().
    templates: Vec<(WorktreeId, TaskTemplate)>,
    _subscriptions: Vec<Subscription>,
}

impl RunConfigurations {
    /// Called from inside `TitleBar::new` while `Workspace` is mid-update; we
    /// must NOT `read(cx)` the workspace entity synchronously here (would
    /// panic with a double-lease). Read directly through the `&Workspace`
    /// reference for what we need synchronously, and defer the initial reload
    /// + persisted-state load to the next foreground tick via `cx.spawn`.
    pub fn new(workspace: &Workspace, cx: &mut Context<Self>) -> Self {
        let weak_workspace = workspace.weak_handle();
        let project = workspace.project().clone();
        let store = project.read(cx).task_store().clone();
        let terminal_panel = workspace.panel::<TerminalPanel>(cx);

        let mut this = Self {
            workspace: weak_workspace,
            active: None,
            pinned: Vec::new(),
            templates: Vec::new(),
            _subscriptions: Vec::new(),
        };

        this._subscriptions.push(cx.observe(&store, |this, _, cx| {
            this.reload_templates(cx);
            cx.notify();
        }));
        if let Some(terminal_panel) = terminal_panel {
            this._subscriptions
                .push(cx.observe(&terminal_panel, |_, _, cx| cx.notify()));
        }

        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.reload_templates(cx);
                this.load_persisted_state(cx).detach();
                cx.notify();
            })
            .ok();
        })
        .detach();

        this
    }

    pub fn pinned_labels(&self) -> &[SharedString] {
        &self.pinned
    }

    pub fn active_label(&self) -> Option<&SharedString> {
        self.active.as_ref()
    }

    pub fn all_templates(&self) -> &[(WorktreeId, TaskTemplate)] {
        &self.templates
    }

    pub fn resolved_pinned(&self) -> Vec<(WorktreeId, TaskTemplate)> {
        self.pinned
            .iter()
            .filter_map(|label| {
                self.templates
                    .iter()
                    .find(|(_, t)| t.label == **label)
                    .cloned()
            })
            .collect()
    }

    pub fn active_template(&self) -> Option<(WorktreeId, TaskTemplate)> {
        let label = self.active.as_ref()?;
        self.templates
            .iter()
            .find(|(_, t)| t.label == **label)
            .cloned()
    }

    pub fn set_active(&mut self, label: SharedString, cx: &mut Context<Self>) {
        if self.active.as_ref() == Some(&label) {
            return;
        }
        self.active = Some(label.clone());
        if !self.pinned.iter().any(|p| p == &label) {
            self.pinned.push(label);
        }
        self.persist(cx);
        self.sync_global(cx);
        cx.notify();
    }

    /// Sync the App-level `PinnedRunConfigurations` global so cross-crate
    /// readers (e.g. the Spawn-Task picker swapping its pin icon) see the
    /// current pinned set without needing a direct dep on title_bar.
    fn sync_global(&self, cx: &mut App) {
        let snapshot: HashSet<String> = self.pinned.iter().map(|s| s.to_string()).collect();
        cx.set_global(PinnedRunConfigurations(snapshot));
    }

    pub fn toggle_pin(&mut self, label: SharedString, cx: &mut Context<Self>) {
        if let Some(pos) = self.pinned.iter().position(|p| *p == label) {
            self.pinned.remove(pos);
            if self.active.as_ref() == Some(&label) {
                self.active = self.pinned.first().cloned();
            }
        } else {
            self.pinned.push(label);
        }
        self.persist(cx);
        self.sync_global(cx);
        cx.notify();
    }

    /// True if a terminal tab with the given label has its task in
    /// `TaskStatus::Running`. Tabs of terminated tasks are NOT counted —
    /// `terminals_for_task` only matches by label, but Zed keeps the tab
    /// around after the child exits (showing the post-mortem banner) so
    /// existence != running.
    pub fn is_running(&self, label: &str, cx: &mut App) -> bool {
        let Some(workspace) = self.workspace.upgrade() else {
            return false;
        };
        let terminal_panel = workspace.read(cx).panel::<TerminalPanel>(cx);
        let Some(terminal_panel) = terminal_panel else {
            return false;
        };
        terminal_panel.update(cx, |panel, cx| {
            panel.terminals_for_task(label, cx).iter().any(|(_, _, view)| {
                view.read(cx)
                    .terminal()
                    .read(cx)
                    .task()
                    .is_some_and(|t| t.status == TaskStatus::Running)
            })
        })
    }

    /// Spawn the script with the given label in its own terminal tab.
    ///
    /// We do NOT dispatch `task::Spawn::ByName` here: its handler resolves
    /// `task_contexts(workspace, ...)` which depends on the *currently active*
    /// editor's worktree. Without an open file, or with the editor pointing
    /// at a different worktree than the pinned task, `Spawn::ByName` finds no
    /// match and falls back to opening the picker — which is exactly the bug
    /// the user reported.
    ///
    /// Instead, look up the cached `(worktree_id, template)` we recorded when
    /// reloading inventory, build a `TaskContext` with that worktree's root
    /// as `cwd` + `$ZED_WORKTREE_ROOT`, and schedule directly. This makes
    /// run-config start independent of the editor's state.
    pub fn start_label(&mut self, label: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.start_label_with(label, false, window, cx);
    }

    /// Same as [`start_label`] but `suppress_focus = true` overrides the
    /// template's `reveal` strategy to `Never`, so spawning the terminal does
    /// not steal focus from the caller. Used when the run-config popover is
    /// held open via Cmd-click for multi-start: stealing focus to the new
    /// terminal would otherwise dismiss the popover.
    pub fn start_label_with(
        &mut self,
        label: &str,
        suppress_focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((worktree_id, template)) = self
            .templates
            .iter()
            .find(|(_, t)| t.label == label)
            .cloned()
        else {
            // Not in our cached worktree templates (e.g. user pinned a
            // language-provided task). Fall back to the modal-aware action;
            // it'll resolve via the picker if needed.
            window.dispatch_action(
                Box::new(zed_actions::Spawn::ByName {
                    task_name: label.to_string(),
                    reveal_target: None,
                }),
                cx,
            );
            return;
        };

        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        let worktree_abs_path = workspace
            .read(cx)
            .project()
            .read(cx)
            .worktree_for_id(worktree_id, cx)
            .map(|w| w.read(cx).abs_path().to_path_buf());

        workspace.update(cx, |workspace, cx| {
            let task_source_kind = TaskSourceKind::Worktree {
                id: worktree_id,
                directory_in_worktree: Arc::from(RelPath::empty()),
                id_base: format!("worktree:{worktree_id:?}").into(),
            };
            let mut task_variables = TaskVariables::default();
            if let Some(ref path) = worktree_abs_path {
                task_variables.insert(
                    VariableName::WorktreeRoot,
                    path.to_string_lossy().to_string(),
                );
            }
            let task_context = TaskContext {
                cwd: worktree_abs_path,
                task_variables,
                project_env: Default::default(),
            };
            let mut template = template;
            if suppress_focus {
                template.reveal = RevealStrategy::Never;
            }
            workspace.schedule_task(task_source_kind, &template, &task_context, false, window, cx);
        });
    }

    /// Kill any terminal(s) running the given label.
    pub fn stop_label(&mut self, label: &str, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        let terminal_panel = workspace.read(cx).panel::<TerminalPanel>(cx);
        let Some(terminal_panel) = terminal_panel else {
            return;
        };
        let terminals = terminal_panel.update(cx, |panel, cx| {
            panel.terminals_for_task(label, cx)
        });
        for (_, _, terminal_view) in terminals {
            terminal_view.update(cx, |view, cx| {
                view.terminal().update(cx, |terminal, _| {
                    // Clear the scrollback first so the only output left after
                    // the kill is the "Task terminated…" banner Zed appends on
                    // child exit + the command line. Mirrors Webstorm's
                    // "stop + flush" behavior.
                    terminal.clear();
                    terminal.kill_active_task();
                });
            });
        }
        // The kill is async — the child reports exit a few ms later, at which
        // point `TaskStatus` flips from Running to Completed. Neither the
        // Terminal entity nor TerminalPanel calls `cx.notify` to bubble this
        // change up to us, so the titlebar would keep showing Stop until
        // another unrelated render. Poll briefly and notify ourselves until
        // `is_running` flips, capped at ~3 s.
        let label_owned = label.to_string();
        cx.spawn(async move |this, cx| {
            for _ in 0..30 {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                let still_running = this
                    .update(cx, |this, cx| this.is_running(&label_owned, cx))
                    .unwrap_or(false);
                this.update(cx, |_, cx| cx.notify()).ok();
                if !still_running {
                    break;
                }
            }
        })
        .detach();
    }

    /// Append a new `TaskTemplate` to the project's `.zed/tasks.json` and pin
    /// it. Creates the file (and the `.zed/` directory) if missing.
    pub fn add_script(
        &mut self,
        template: TaskTemplate,
        cx: &mut Context<Self>,
    ) -> Task<anyhow::Result<()>> {
        let label_string = template.label.clone();
        let label = SharedString::from(label_string.clone());
        let fs = <dyn Fs>::global(cx);
        let Some(workspace) = self.workspace.upgrade() else {
            return Task::ready(Err(anyhow::anyhow!("workspace gone")));
        };
        let worktree = workspace.read(cx).visible_worktrees(cx).next();
        let Some(worktree) = worktree else {
            return Task::ready(Err(anyhow::anyhow!(
                "no worktree to write tasks.json into"
            )));
        };
        let worktree_root = worktree.read(cx).abs_path().to_path_buf();
        let target = worktree_root.join(".zed").join("tasks.json");
        cx.spawn(async move |this, cx| {
            append_template_to_tasks_json(fs.clone(), &target, template).await?;
            this.update(cx, |this, cx| {
                if !this.pinned.iter().any(|p| **p == label_string) {
                    this.pinned.push(label.clone());
                }
                this.active = Some(label);
                this.persist(cx);
                this.reload_templates(cx);
                cx.notify();
            })
            .ok();
            Ok(())
        })
    }

    pub fn reload_templates(&mut self, cx: &mut Context<Self>) {
        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };
        let workspace = workspace.read(cx);
        let project: Entity<Project> = workspace.project().clone();
        let task_store = project.read(cx).task_store().clone();
        let Some(inventory) = task_store.read(cx).task_inventory().cloned() else {
            return;
        };

        let worktrees: Vec<WorktreeId> = workspace
            .visible_worktrees(cx)
            .map(|w| w.read(cx).id())
            .collect();

        let mut templates: Vec<(WorktreeId, TaskTemplate)> = Vec::new();
        for worktree_id in worktrees {
            for (_kind, template) in inventory
                .read(cx)
                .worktree_templates_from_settings(worktree_id)
            {
                templates.push((worktree_id, template.clone()));
            }
        }

        // Stable dedupe by label (first occurrence wins).
        let mut seen = std::collections::HashSet::new();
        templates.retain(|(_, t)| seen.insert(t.label.clone()));

        self.templates = templates;

        // Do NOT prune `self.pinned` / `self.active` against `self.templates`
        // here. `Inventory` doesn't notify when tasks.json finishes parsing,
        // so on startup `templates` is empty for several ticks; pruning would
        // silently nuke every persisted pinned label. `resolved_pinned()`
        // already filters the *display* set by `self.templates`, so
        // unresolved labels are invisible in the dropdown until tasks.json
        // is loaded, then reappear automatically.
        if self.active.is_none() {
            self.active = self.pinned.first().cloned();
        }
    }

    fn load_persisted_state(&self, cx: &mut Context<Self>) -> Task<()> {
        // Resolve fs + workspace path lazily inside the spawned task so we
        // don't try to `workspace.read(cx)` while the workspace entity may
        // still be held mid-update by an outer caller.
        cx.spawn(async move |this, cx| {
            let resolved = this
                .update(cx, |this, cx| {
                    let fs = <dyn Fs>::global(cx);
                    let workspace = this.workspace.upgrade()?;
                    let worktree = workspace.read(cx).visible_worktrees(cx).next()?;
                    let path = worktree
                        .read(cx)
                        .abs_path()
                        .join(local_run_configurations_file_relative_path().as_std_path());
                    Some((fs, path))
                })
                .ok()
                .flatten();
            let Some((fs, path)) = resolved else {
                return;
            };
            let bytes = match fs.load_bytes(&path).await {
                Ok(b) => b,
                Err(_) => return,
            };
            let parsed: Result<PersistedState, _> = serde_json_lenient::from_slice(&bytes);
            if let Ok(state) = parsed {
                this.update(cx, |this, cx| {
                    this.pinned = state
                        .pinned
                        .into_iter()
                        .map(SharedString::from)
                        .collect();
                    this.active = state.active.map(SharedString::from);
                    this.reload_templates(cx);
                    this.sync_global(cx);
                    cx.notify();
                })
                .ok();
            }
        })
    }

    /// Persistence cannot synchronously `workspace.read(cx)` because callers
    /// (e.g. workspace action handlers) hold the workspace entity's borrow.
    /// Capture the snapshot we need (state), then resolve `fs` + target path
    /// asynchronously on the next foreground tick when the workspace update
    /// has completed.
    fn persist(&self, cx: &mut Context<Self>) {
        let state = PersistedState {
            active: self.active.as_ref().map(|s| s.to_string()),
            pinned: self.pinned.iter().map(|s| s.to_string()).collect(),
        };
        cx.spawn(async move |this, cx| {
            let resolved = this
                .update(cx, |this, cx| {
                    let fs = <dyn Fs>::global(cx);
                    let workspace = this.workspace.upgrade()?;
                    let worktree = workspace.read(cx).visible_worktrees(cx).next()?;
                    let target = worktree
                        .read(cx)
                        .abs_path()
                        .join(local_run_configurations_file_relative_path().as_std_path());
                    Some((fs, target))
                })
                .ok()
                .flatten();
            let Some((fs, target)) = resolved else {
                return;
            };
            write_persisted_state(fs, &target, &state).await.log_err();
        })
        .detach();
    }
}

async fn write_persisted_state(
    fs: Arc<dyn Fs>,
    target: &std::path::Path,
    state: &PersistedState,
) -> anyhow::Result<()> {
    if let Some(parent) = target.parent() {
        fs.create_dir(parent).await.ok();
    }
    let serialized = serde_json::to_string_pretty(state)?;
    fs.atomic_write(target.to_path_buf(), serialized).await?;
    Ok(())
}

async fn append_template_to_tasks_json(
    fs: Arc<dyn Fs>,
    target: &std::path::Path,
    template: TaskTemplate,
) -> anyhow::Result<()> {
    if let Some(parent) = target.parent() {
        fs.create_dir(parent).await.ok();
    }
    let mut templates: Vec<TaskTemplate> = match fs.load_bytes(target).await {
        Ok(bytes) if !bytes.is_empty() => serde_json_lenient::from_slice::<TaskTemplates>(&bytes)
            .map(|t| t.0)
            .with_context(|| format!("parsing {}", target.display()))
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    if templates.iter().any(|t| t.label == template.label) {
        anyhow::bail!("a task labeled '{}' already exists", template.label);
    }
    templates.push(template);

    let serialized = serde_json::to_string_pretty(&TaskTemplates(templates))?;
    fs.atomic_write(target.to_path_buf(), serialized).await?;
    Ok(())
}
