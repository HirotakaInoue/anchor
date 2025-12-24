use crate::app::{App, AppTab};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Tabs},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Status bar
            Constraint::Length(2), // Help
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);

    match app.current_tab {
        AppTab::Ports => draw_ports_tab(f, app, chunks[1]),
        AppTab::Tunnels => draw_tunnels_tab(f, app, chunks[1]),
    }

    draw_status_bar(f, app, chunks[2]);
    draw_help(f, app, chunks[3]);

    // Draw dialogs on top
    if app.show_filter {
        draw_filter_dialog(f, app);
    }

    if app.show_input {
        draw_input_dialog(f, app);
    }

    if app.show_confirm {
        draw_confirm_dialog(f, app);
    }
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec!["[1] Ports", "[2] SSH Tunnels"];
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" PortMan ")
                .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        )
        .select(match app.current_tab {
            AppTab::Ports => 0,
            AppTab::Tunnels => 1,
        })
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn draw_ports_tab(f: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["Port", "PID", "Process", "Protocol", "State", "Address"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .filtered_ports
        .iter()
        .enumerate()
        .map(|(i, port)| {
            let style = if i == app.port_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let state_style = match port.state.as_str() {
                "LISTEN" => style.fg(Color::Green),
                "ESTABLISHED" => style.fg(Color::Cyan),
                _ => style.fg(Color::Gray),
            };

            Row::new(vec![
                Cell::from(port.port.to_string()).style(style),
                Cell::from(port.pid.to_string()).style(style),
                Cell::from(port.process_name.clone()).style(style),
                Cell::from(port.protocol.clone()).style(style),
                Cell::from(port.state.clone()).style(state_style),
                Cell::from(port.local_address.clone()).style(style),
            ])
            .height(1)
        })
        .collect();

    let title = if app.filter_text.is_empty() {
        format!(" Ports ({}) ", app.filtered_ports.len())
    } else {
        format!(
            " Ports ({}/{}) [filter: {}] ",
            app.filtered_ports.len(),
            app.ports.len(),
            app.filter_text
        )
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),  // Port
            Constraint::Length(8),  // PID
            Constraint::Length(20), // Process
            Constraint::Length(10), // Protocol
            Constraint::Length(14), // State
            Constraint::Min(20),    // Address
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(title))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(table, area);
}

fn draw_tunnels_tab(f: &mut Frame, app: &App, area: Rect) {
    let header_cells = ["Name", "SSH Host", "Local Port", "Remote Target", "Status"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .tunnel_manager
        .tunnels
        .iter()
        .enumerate()
        .map(|(i, tunnel)| {
            let style = if i == app.tunnel_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let status_style = if tunnel.is_connected() {
                style.fg(Color::Green)
            } else {
                style.fg(Color::Gray)
            };

            Row::new(vec![
                Cell::from(tunnel.name.clone()).style(style),
                Cell::from(tunnel.ssh_host.clone()).style(style),
                Cell::from(tunnel.local_port.to_string()).style(style),
                Cell::from(tunnel.remote_target.clone()).style(style),
                Cell::from(tunnel.status_string()).style(status_style),
            ])
            .height(1)
        })
        .collect();

    let title = format!(" SSH Tunnels ({}) ", app.tunnel_manager.tunnels.len());

    let table = Table::new(
        rows,
        [
            Constraint::Length(15), // Name
            Constraint::Length(25), // SSH Host
            Constraint::Length(12), // Local Port
            Constraint::Length(20), // Remote Target
            Constraint::Min(15),    // Status
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(table, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = Paragraph::new(app.status_message.clone())
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title(" Status "));

    f.render_widget(status, area);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.current_tab {
        AppTab::Ports => {
            " ↑/↓:Navigate  K:Kill  r:Refresh  /:Filter  Tab:Switch  q:Quit "
        }
        AppTab::Tunnels => {
            " ↑/↓:Navigate  a:Add  c:Connect  d:Disconnect  x:Delete  Tab:Switch  q:Quit "
        }
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(help, area);
}

fn draw_filter_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 3, f.area());

    let filter_text = format!("/{}", app.filter_text);
    let input = Paragraph::new(filter_text)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Filter (Enter/Esc to close) ")
                .border_style(Style::default().fg(Color::Yellow)),
        );

    f.render_widget(Clear, area);
    f.render_widget(input, area);
}

fn draw_input_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 3, f.area());

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", app.input_prompt))
                .border_style(Style::default().fg(Color::Cyan)),
        );

    f.render_widget(Clear, area);
    f.render_widget(input, area);
}

fn draw_confirm_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 5, f.area());

    let text = vec![
        Line::from(app.confirm_message.clone()),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]es", Style::default().fg(Color::Green)),
            Span::raw("  "),
            Span::styled("[N]o", Style::default().fg(Color::Red)),
        ]),
    ];

    let dialog = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm ")
                .border_style(Style::default().fg(Color::Yellow)),
        );

    f.render_widget(Clear, area);
    f.render_widget(dialog, area);
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height.min(100)) / 2),
            Constraint::Length(height),
            Constraint::Percentage((100 - height.min(100)) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
