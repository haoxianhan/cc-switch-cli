use super::super::theme;
use super::super::*;

pub(super) fn render_mcp_env_picker_overlay(
    frame: &mut Frame<'_>,
    app: &App,
    content_area: Rect,
    theme: &theme::Theme,
    selected: usize,
) {
    let area = centered_rect_fixed(64, 16, content_area);
    frame.render_widget(Clear, area);

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(overlay_border_style(theme, false))
        .title(texts::tui_mcp_env_title());
    frame.render_widget(outer.clone(), area);
    let inner = outer.inner(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);

    render_key_bar_center(
        frame,
        chunks[0],
        theme,
        &[
            ("↑↓", texts::tui_key_select()),
            ("a", texts::tui_key_add()),
            ("Enter", texts::tui_key_edit()),
            ("Del", texts::tui_key_delete()),
            ("Esc", texts::tui_key_close()),
        ],
    );

    let Some(FormState::McpAdd(mcp)) = app.form.as_ref() else {
        return;
    };

    if mcp.env_rows.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::raw(texts::tui_mcp_env_empty_state()))
                .alignment(Alignment::Center),
            inset_top(chunks[1], 1),
        );
        return;
    }

    let items = mcp
        .env_rows
        .iter()
        .map(|row| ListItem::new(Line::raw(format!("{} = {}", row.key, row.value))));

    let list = List::new(items)
        .highlight_style(selection_style(theme))
        .highlight_symbol(highlight_symbol(theme));

    let mut state = ListState::default();
    state.select(Some(selected.min(mcp.env_rows.len().saturating_sub(1))));
    frame.render_stateful_widget(list, inset_top(chunks[1], 1), &mut state);
}

pub(super) fn render_mcp_env_entry_editor_overlay(
    frame: &mut Frame<'_>,
    content_area: Rect,
    theme: &theme::Theme,
    overlay: &Overlay,
) {
    let Overlay::McpEnvEntryEditor(editor) = overlay else {
        return;
    };

    let area = centered_rect_fixed(64, 12, content_area);
    frame.render_widget(Clear, area);

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(overlay_border_style(theme, false))
        .title(if editor.row.is_some() {
            texts::tui_mcp_env_edit_entry_title()
        } else {
            texts::tui_mcp_env_add_entry_title()
        });
    frame.render_widget(outer.clone(), area);
    let inner = outer.inner(area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(inner);

    render_key_bar_center(
        frame,
        chunks[0],
        theme,
        &[
            ("Tab", texts::tui_key_select()),
            ("Enter", texts::tui_key_apply()),
            ("Esc", texts::tui_key_cancel()),
        ],
    );

    let fields = [
        (
            texts::tui_mcp_env_key_label(),
            &editor.key,
            editor.key_active(),
        ),
        (
            texts::tui_mcp_env_value_label(),
            &editor.value,
            editor.value_active(),
        ),
    ];

    for (idx, (label, input, active)) in fields.into_iter().enumerate() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(if active {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.dim)
            })
            .title(label);
        let input_area = chunks[idx + 1];
        let input_inner = block.inner(input_area);
        frame.render_widget(block, input_area);

        let (visible, cursor_x) =
            visible_text_window(&input.value, input.cursor, input_inner.width as usize);
        frame.render_widget(
            Paragraph::new(Line::raw(visible)).wrap(Wrap { trim: false }),
            input_inner,
        );

        if active {
            frame.set_cursor_position((
                input_inner.x + cursor_x.min(input_inner.width.saturating_sub(1)),
                input_inner.y,
            ));
        }
    }
}
