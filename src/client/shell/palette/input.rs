use super::super::*;

impl ClientShellState {
    pub(in crate::client::shell) fn open_palette_overlay(
        &mut self,
        outcome: &mut ClientShellInput,
    ) {
        self.overlay = Some(ClientShellOverlay::Palette(ClientPaletteOverlay {
            recent_command_ids: self.recent_command_ids.clone(),
            plugins: super::PalettePlugins {
                destructive_actions: self.config.destructive_palette_actions.clone(),
                ..super::PalettePlugins::default()
            },
            ..ClientPaletteOverlay::default()
        }));
        self.chrome_drag = None;
        // Plugin actions and panes are server-owned facts; ask for them once
        // per open rather than caching a list that goes stale on plugin
        // enable/disable. The palette renders its core rows meanwhile.
        let list = crate::api::schema::Method::PluginList(crate::api::schema::PluginListParams {
            plugin_id: None,
        });
        // An endpoint too old to list plugins simply has no plugin rows to
        // offer; that is not worth an "action unavailable" notice over a
        // palette the operator opened for its core commands.
        if self.supports_endpoint_method(&list) {
            self.push_endpoint_method_with_kind(
                list,
                PendingEndpointKind::PalettePluginList,
                outcome,
            );
        }
    }

    pub(in crate::client::shell) fn receive_palette_plugins(
        &mut self,
        installed: Vec<crate::api::schema::InstalledPluginInfo>,
        host_platform: Option<crate::api::schema::PluginPlatform>,
    ) -> bool {
        let destructive_actions = self.config.destructive_palette_actions.clone();
        let Some(ClientShellOverlay::Palette(palette)) = self.overlay.as_mut() else {
            return false;
        };
        palette.plugins = super::PalettePlugins {
            installed,
            host_platform,
            destructive_actions,
        };
        palette.selected = 0;
        palette.scroll = 0;
        true
    }

    pub(in crate::client::shell) fn filtered_palette_commands(&self) -> Vec<super::PaletteRow> {
        let (Some(snapshot), Some(ClientShellOverlay::Palette(palette))) =
            (self.snapshot.as_deref(), self.overlay.as_ref())
        else {
            return Vec::new();
        };
        super::filtered_palette_commands(
            &palette.query,
            &palette.recent_command_ids,
            &self.config.keybinds,
            &palette.plugins,
            snapshot,
        )
    }

    fn palette_body_height(&self) -> usize {
        self.last_composed_size
            .and_then(|(cols, rows)| super::palette_geometry(Rect::new(0, 0, cols, rows)))
            .map(|(_, _, body)| usize::from(body.height.max(1)))
            .unwrap_or(1)
    }

    /// Takes the row count from the caller: rebuilding the command list is the
    /// expensive part, and every caller has already built it.
    fn ensure_palette_selection_visible(&mut self, count: usize) {
        let viewport = self.palette_body_height();
        let max_scroll = count.saturating_sub(viewport);
        let Some(ClientShellOverlay::Palette(palette)) = self.overlay.as_mut() else {
            return;
        };
        let adjusted = if palette.selected < palette.scroll {
            palette.selected
        } else if palette.selected >= palette.scroll + viewport {
            palette.selected + 1 - viewport
        } else {
            return;
        };
        palette.scroll = adjusted.min(max_scroll);
    }

    pub(in crate::client::shell) fn move_palette_selection(&mut self, delta: isize) {
        let count = self.filtered_palette_commands().len();
        let Some(ClientShellOverlay::Palette(palette)) = self.overlay.as_mut() else {
            return;
        };
        if count == 0 {
            palette.selected = 0;
            palette.scroll = 0;
            return;
        }
        let current = palette.selected.min(count - 1) as isize;
        palette.selected = (current + delta).rem_euclid(count as isize) as usize;
        self.ensure_palette_selection_visible(count);
    }

    fn reset_palette_selection(&mut self) {
        if let Some(ClientShellOverlay::Palette(palette)) = self.overlay.as_mut() {
            palette.selected = 0;
            palette.scroll = 0;
        }
    }

    /// Runs the highlighted palette row. A query matching nothing leaves the
    /// palette open, so an empty enter is not a silent dismissal.
    pub(in crate::client::shell) fn run_palette_selection(
        &mut self,
        outcome: &mut ClientShellInput,
    ) {
        let (query, selected) = match self.overlay.as_ref() {
            Some(ClientShellOverlay::Palette(palette)) => (palette.query.clone(), palette.selected),
            _ => return,
        };
        let rows = self.filtered_palette_commands();
        let Some(row) = rows.into_iter().nth(selected) else {
            return;
        };
        self.overlay = None;
        outcome.repaint = true;

        // A family row runs nothing on its own — it asks. Recording it here
        // would put a question at the head of the empty palette and record
        // the leaf a second time when the chooser runs it.
        if let super::PaletteAction::Chooser(family) = row.command.action {
            self.open_chooser_overlay(
                family.chooser_title().to_owned(),
                family.choices(),
                Some(PaletteReturn { query, selected }),
            );
            return;
        }

        // Cancel is offered first and selected by default, so the reflex
        // second Enter backs out rather than running the thing. History
        // records inside the "run anyway" outcome, never here — a cancelled
        // row must not lead the next empty palette.
        if row.command.destructive {
            self.open_chooser_overlay(
                row.command.name.into_owned(),
                vec![
                    ChooserChoice {
                        label: " cancel ",
                        outcome: ChooserOutcome::Cancel,
                    },
                    ChooserChoice {
                        label: " run anyway ",
                        outcome: ChooserOutcome::Palette {
                            action: row.command.action,
                            command_id: row.command.id,
                        },
                    },
                ],
                Some(PaletteReturn { query, selected }),
            );
            return;
        }

        self.remember_palette_command(row.command.id);
        self.run_palette_action(row.command.action, outcome);
    }

    pub(in crate::client::shell) fn remember_palette_command(&mut self, command_id: String) {
        crate::palette_history::remember(&mut self.recent_command_ids, command_id);
        self.persist_palette_history();
    }

    #[cfg(not(test))]
    fn persist_palette_history(&self) {
        if let Err(error) = crate::palette_history::save(&self.recent_command_ids) {
            tracing::warn!(
                path = %crate::palette_history::store_path().display(),
                error = %error,
                "Failed to save command palette history; the command still ran"
            );
        }
    }

    /// Tests must not write the operator's real history file.
    #[cfg(test)]
    fn persist_palette_history(&self) {}

    pub(in crate::client::shell) fn run_palette_action(
        &mut self,
        action: super::PaletteAction,
        outcome: &mut ClientShellInput,
    ) {
        match action {
            super::PaletteAction::Keybind(action) => {
                self.record_binding(crate::input::KeybindMatch::Action(action), outcome);
            }
            super::PaletteAction::PluginAction {
                plugin_id,
                action_id,
            } => {
                self.push_endpoint_method(
                    crate::api::schema::Method::PluginActionInvoke(
                        crate::api::schema::PluginActionInvokeParams {
                            action_id,
                            plugin_id: Some(plugin_id),
                            // The server merges the focused workspace, tab and
                            // pane into the context itself, and it is the only
                            // side that can see them authoritatively.
                            context: None,
                        },
                    ),
                    outcome,
                );
            }
            // Reached only through a chooser button, which resolves the
            // family to a leaf action before running anything.
            super::PaletteAction::Chooser(_) => {}
            super::PaletteAction::PluginPane {
                plugin_id,
                entrypoint,
            } => {
                self.push_endpoint_method(
                    crate::api::schema::Method::PluginPaneOpen(
                        crate::api::schema::PluginPaneOpenParams {
                            plugin_id,
                            entrypoint,
                            placement: None,
                            width: None,
                            height: None,
                            workspace_id: None,
                            target_pane_id: None,
                            direction: None,
                            cwd: None,
                            focus: true,
                            env: Default::default(),
                        },
                    ),
                    outcome,
                );
            }
        }
    }

    pub(in crate::client::shell) fn route_palette_key(
        &mut self,
        key: &crate::input::TerminalKey,
        outcome: &mut ClientShellInput,
    ) {
        use crossterm::event::KeyModifiers;

        let text_character = crate::input::keybind_help_text_char(key);
        let (code, modifiers) = crate::config::normalize_key_combo((key.code, key.modifiers));
        match code {
            KeyCode::Esc => {
                self.overlay = None;
            }
            KeyCode::Enter => {
                self.run_palette_selection(outcome);
                return;
            }
            KeyCode::Up | KeyCode::BackTab => self.move_palette_selection(-1),
            KeyCode::Down | KeyCode::Tab => self.move_palette_selection(1),
            KeyCode::PageUp => self.move_palette_selection(-8),
            KeyCode::PageDown => self.move_palette_selection(8),
            KeyCode::Backspace => {
                if let Some(ClientShellOverlay::Palette(palette)) = self.overlay.as_mut() {
                    palette.query.pop();
                }
                self.reset_palette_selection();
            }
            KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(ClientShellOverlay::Palette(palette)) = self.overlay.as_mut() {
                    palette.query.clear();
                }
                self.reset_palette_selection();
            }
            _ => {
                if let Some(character) = text_character {
                    if let Some(ClientShellOverlay::Palette(palette)) = self.overlay.as_mut() {
                        palette.query.push(character);
                    }
                    self.reset_palette_selection();
                }
            }
        }
        outcome.repaint = true;
    }
}
