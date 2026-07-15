use std::io;

use anyhow::Result;
use chrono::{Local, TimeZone};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use haqor_core::tutor::IssueReport;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// Show reports and return the IDs marked resolved when the user presses `s`.
/// Quitting discards any in-memory selections.
pub(super) fn review(reports: &[IssueReport]) -> Result<Vec<String>> {
    if reports.is_empty() {
        println!("No active app issue reports to review.");
        return Ok(Vec::new());
    }

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    let _guard = TerminalGuard;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut selected = 0_usize;
    let mut resolving = vec![false; reports.len()];

    loop {
        terminal.draw(|frame| draw(frame, reports, selected, &resolving))?;
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(Vec::new()),
            KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => selected = (selected + 1).min(reports.len() - 1),
            KeyCode::Char('d') | KeyCode::Char(' ') => resolving[selected] = !resolving[selected],
            KeyCode::Char('s') => {
                return Ok(reports
                    .iter()
                    .zip(resolving)
                    .filter(|(_, resolved)| *resolved)
                    .map(|(report, _)| report.id.clone())
                    .collect());
            }
            _ => {}
        }
    }
}

fn draw(frame: &mut ratatui::Frame, reports: &[IssueReport], selected: usize, resolving: &[bool]) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(10),
            Constraint::Length(2),
        ])
        .split(frame.area());
    let items = reports
        .iter()
        .enumerate()
        .map(|(index, report)| {
            let marker = if resolving[index] {
                "[resolve]"
            } else {
                "[       ]"
            };
            let note = report.note.lines().next().unwrap_or_default();
            ListItem::new(format!(
                "{marker} {} {:<4} {note}",
                format_timestamp(report.created_epoch),
                report.report_type,
            ))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select(Some(selected));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" App issue reports "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_stateful_widget(list, areas[0], &mut state);

    let report = &reports[selected];
    let context = serde_json::from_str::<serde_json::Value>(&report.context_json)
        .map(|value| {
            serde_json::to_string_pretty(&value).unwrap_or_else(|_| report.context_json.clone())
        })
        .unwrap_or_else(|_| report.context_json.clone());
    let detail = Text::from(vec![
        Line::from(format!(
            "{}  {}  id: {}",
            report.report_type,
            format_timestamp(report.created_epoch),
            report.id
        )),
        Line::from(report.note.clone()),
        Line::from(""),
        Line::from(context),
    ]);
    frame.render_widget(
        Paragraph::new(detail)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Selected report "),
            )
            .wrap(Wrap { trim: false }),
        areas[1],
    );
    frame.render_widget(
        Paragraph::new("↑/↓ or j/k move · d/space mark resolved · s save and sync · q cancel"),
        areas[2],
    );
}

fn format_timestamp(epoch: i64) -> String {
    Local
        .timestamp_opt(epoch, 0)
        .single()
        .map(|timestamp| timestamp.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| format!("epoch {epoch}"))
}

#[cfg(test)]
mod tests {
    use super::format_timestamp;

    #[test]
    fn formats_created_epoch_for_the_review_table() {
        assert_ne!(format_timestamp(0), "epoch 0");
    }
}
