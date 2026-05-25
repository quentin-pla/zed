use super::*;
use crate::code_context_menus::{AvailableCodeAction, CodeActionContents, CodeActionsMenu};
use project::lsp_store::SymbolLocation;
use std::path::{Path, PathBuf};

pub struct ImportSuggestion {
    edit_range: Range<text::Anchor>,
    edit_text: SharedString,
}

pub struct ImportSuggestionsProvider {
    suggestions: Rc<Vec<(String, ImportSuggestion)>>,
}

impl CodeActionProvider for ImportSuggestionsProvider {
    fn id(&self) -> Arc<str> {
        "import_suggestions".into()
    }

    fn code_actions(
        &self,
        _buffer: &Entity<Buffer>,
        _range: Range<text::Anchor>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Vec<CodeAction>>> {
        Task::ready(Ok(vec![]))
    }

    fn apply_code_action(
        &self,
        buffer_handle: Entity<Buffer>,
        action: CodeAction,
        push_to_history: bool,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<ProjectTransaction>> {
        let title = action.lsp_action.title().to_owned();
        let suggestion = self
            .suggestions
            .iter()
            .find(|(key, _)| key == &title)
            .map(|(_, s)| (s.edit_range.clone(), s.edit_text.clone()));

        let Some((edit_range, edit_text)) = suggestion else {
            return Task::ready(Err(anyhow::anyhow!("import suggestion not found")));
        };

        let transaction_id = buffer_handle.update(cx, |buffer, cx| {
            buffer.start_transaction();
            buffer.edit([(edit_range, edit_text.as_ref())], None, cx);
            buffer.end_transaction(cx)
        });

        let mut project_transaction = ProjectTransaction::default();
        if push_to_history {
            if let Some(tid) = transaction_id {
                if let Some(transaction) = buffer_handle.read(cx).get_transaction(tid) {
                    project_transaction
                        .0
                        .insert(buffer_handle, transaction.clone());
                }
            }
        }

        Task::ready(Ok(project_transaction))
    }
}

impl Editor {
    /// Shows import suggestions for the word under the cursor (JS/TS files only).
    pub fn show_import_suggestions(
        &mut self,
        _: &ShowImportSuggestions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.project.clone() else {
            return;
        };

        let singleton_buffer = self.buffer().read(cx).as_singleton();
        let Some(singleton_buffer) = singleton_buffer else {
            return;
        };

        let file = singleton_buffer.read(cx).file().cloned();
        let Some(file) = file else {
            return;
        };

        let full_path = file.full_path(cx);
        let extension = full_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !is_js_ts_extension(&extension) {
            return;
        }

        let display_snapshot = self.display_snapshot(cx);
        let head = self
            .selections
            .newest::<Point>(&display_snapshot)
            .head();

        let buffer_snapshot = singleton_buffer.read(cx).snapshot();

        let (word_range, _) = buffer_snapshot.surrounding_word(head, None);
        if word_range.is_empty() {
            return;
        }

        let word = buffer_snapshot
            .text_for_range(word_range.clone())
            .collect::<String>();
        if word.is_empty() {
            return;
        }

        let from_file_path = full_path.clone();
        let symbols_task = project.update(cx, |project, cx| project.symbols(&word, cx));

        let worktree_id = file.worktree_id(cx);
        let worktree_root = {
            let Some(worktree) = project.read(cx).worktree_for_id(worktree_id, cx) else {
                return;
            };
            worktree.read(cx).abs_path().to_path_buf()
        };

        let weak_buffer = singleton_buffer.downgrade();
        cx.spawn_in(window, async move |editor, cx| {
            let symbols = symbols_task.await?;

            let mut suggestions: Vec<(String, ImportSuggestion)> = Vec::new();

            for symbol in symbols {
                if symbol.name != word {
                    continue;
                }

                let target_path: PathBuf = match &symbol.path {
                    SymbolLocation::InProject(project_path) => {
                        worktree_root.join(project_path.path.as_ref())
                    }
                    SymbolLocation::OutsideProject { abs_path, .. } => {
                        abs_path.to_path_buf()
                    }
                };

                let target_ext = target_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();

                if !is_js_ts_extension(&target_ext) {
                    continue;
                }

                if target_path == from_file_path {
                    continue;
                }

                let Some(import_path) =
                    compute_import_path(&from_file_path, &target_path, &worktree_root)
                else {
                    continue;
                };

                let snapshot = weak_buffer.read_with(cx, |buf, _| buf.snapshot()).ok();
                let Some(snapshot) = snapshot else {
                    continue;
                };

                if is_already_imported(&snapshot, &word, &import_path) {
                    continue;
                }

                let (insert_point, _) = find_import_insert_position(&snapshot);
                let anchor = snapshot.anchor_before(insert_point);

                let import_text: SharedString =
                    format!("import {{ {} }} from '{}';\n", word, import_path).into();
                let title = format!("Import '{}' from '{}'", word, import_path);

                suggestions.push((
                    title,
                    ImportSuggestion {
                        edit_range: anchor.clone()..anchor.clone(),
                        edit_text: import_text,
                    },
                ));
            }

            if suggestions.is_empty() {
                return anyhow::Ok(());
            }

            let provider = Rc::new(ImportSuggestionsProvider {
                suggestions: Rc::new(suggestions),
            });

            let actions_vec: Vec<AvailableCodeAction> = provider
                .suggestions
                .iter()
                .map(|(title, suggestion)| {
                    let server_id = lsp::LanguageServerId(0);
                    let lsp_action = project::LspAction::Action(Box::new(lsp::CodeAction {
                        title: title.clone(),
                        ..Default::default()
                    }));
                    let anchor = suggestion.edit_range.start.clone();
                    AvailableCodeAction {
                        action: project::CodeAction {
                            server_id,
                            range: anchor.clone()..anchor.clone(),
                            lsp_action,
                            resolved: true,
                        },
                        provider: provider.clone() as Rc<dyn CodeActionProvider>,
                    }
                })
                .collect();

            let actions = CodeActionContents::new(
                None,
                Some(Rc::from(actions_vec.as_slice())),
                vec![],
                Default::default(),
            );

            editor.update_in(cx, |editor, _window, cx| {
                let buffer = weak_buffer
                    .upgrade()
                    .ok_or_else(|| anyhow::anyhow!("buffer dropped"))?;
                *editor.context_menu.borrow_mut() =
                    Some(CodeContextMenu::CodeActions(CodeActionsMenu {
                        buffer,
                        actions,
                        selected_item: Default::default(),
                        scroll_handle: UniformListScrollHandle::default(),
                        deployed_from: None,
                    }));
                cx.notify();
                anyhow::Ok(())
            })??;

            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }
}

fn is_js_ts_extension(ext: &str) -> bool {
    matches!(ext, "js" | "jsx" | "mjs" | "ts" | "tsx" | "mts")
}

fn compute_import_path(from_file: &Path, target: &Path, _worktree_root: &Path) -> Option<String> {
    let target_str = target.to_string_lossy();

    if let Some(idx) = target_str.rfind("node_modules/") {
        let after = &target_str[idx + "node_modules/".len()..];
        let segments: Vec<&str> = after.splitn(4, '/').collect();
        if segments.is_empty() {
            return None;
        }
        if segments[0].starts_with('@') && segments.len() >= 2 {
            return Some(format!("{}/{}", segments[0], segments[1]));
        }
        return Some(segments[0].to_owned());
    }

    let mut target_stripped = target.to_path_buf();
    if let Some(ext) = target_stripped.extension().and_then(|e| e.to_str()) {
        if matches!(ext, "ts" | "tsx" | "js" | "jsx" | "mjs" | "mts") {
            target_stripped.set_extension("");
        }
    }

    let stem = target_stripped.file_stem().and_then(|s| s.to_str());
    if stem == Some("index") {
        target_stripped = target_stripped.parent()?.to_path_buf();
    }

    let from_dir = from_file.parent()?;

    let from_components: Vec<_> = from_dir.components().collect();
    let target_components: Vec<_> = target_stripped.components().collect();

    let common_len = from_components
        .iter()
        .zip(target_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let up_count = from_components.len() - common_len;
    let down_parts: Vec<_> = target_components[common_len..]
        .iter()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();

    let mut result = String::new();
    if up_count == 0 {
        result.push_str("./");
        result.push_str(&down_parts.join("/"));
    } else {
        for i in 0..up_count {
            if i > 0 {
                result.push('/');
            }
            result.push_str("..");
        }
        if !down_parts.is_empty() {
            result.push('/');
            result.push_str(&down_parts.join("/"));
        }
    }

    Some(result)
}

fn find_import_insert_position(snapshot: &BufferSnapshot) -> (Point, bool) {
    let row_count = snapshot.max_point().row;
    let limit = row_count.min(99);
    let mut last_import_row: Option<u32> = None;

    let mut row = 0u32;
    while row <= limit {
        let line = snapshot
            .text_for_range(Point::new(row, 0)..Point::new(row, u32::MAX))
            .collect::<String>();
        let trimmed = line.trim_start();

        if trimmed.starts_with("import ")
            || trimmed.starts_with("import\"")
            || trimmed.starts_with("import'")
            || trimmed.starts_with("import{")
        {
            let mut scan_row = row;
            loop {
                let scan_line = snapshot
                    .text_for_range(
                        Point::new(scan_row, 0)..Point::new(scan_row, u32::MAX),
                    )
                    .collect::<String>();
                let scan_trimmed = scan_line.trim_end();
                if scan_trimmed.ends_with(';')
                    || scan_trimmed.ends_with('"')
                    || scan_trimmed.ends_with('\'')
                {
                    last_import_row = Some(scan_row);
                    row = scan_row + 1;
                    break;
                }
                scan_row += 1;
                if scan_row > limit + 10 {
                    last_import_row = Some(row);
                    row = scan_row;
                    break;
                }
            }
        } else {
            row += 1;
        }
    }

    match last_import_row {
        Some(r) => (Point::new(r + 1, 0), true),
        None => (Point::zero(), false),
    }
}

fn is_already_imported(snapshot: &BufferSnapshot, name: &str, import_path: &str) -> bool {
    let row_count = snapshot.max_point().row;
    let limit = row_count.min(99);

    for row in 0..=limit {
        let line = snapshot
            .text_for_range(Point::new(row, 0)..Point::new(row, u32::MAX))
            .collect::<String>();

        if line.contains(import_path) && line.contains(name) {
            return true;
        }
    }

    false
}
