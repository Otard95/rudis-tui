use serde_json::{Value, from_str, to_string_pretty};
use tui::{
    backend::Backend,
    widgets::{Block, Borders, Paragraph, Tabs, Table, Row, Wrap, block::Title, Clear},
    layout::{Layout, Alignment, Constraint, Rect},
    text::{Span, Spans}, Frame, style::{Style, Color},
};

use crate::app::{App, RedisServer};

fn ui_tabs<B>(f: &mut Frame<B>, area: Rect, app: &App)
where
    B: Backend,
{
    let current_tab = app.current_tab();
    let servers = app.get_servers();

    let titles = servers.into_iter().map(|s| {
        Spans::from(Span::raw(s.name.clone()))
    }).collect::<Vec<Spans>>();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" Servers "))
        .select(current_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow));

    f.render_widget(tabs, area);
}

fn ui_server_disconnected<B>(f: &mut Frame<B>, area: Rect, server: &mut RedisServer)
where
    B: Backend,
{

    let title = server.name.clone();
    let host = server.host.clone();

    let text = vec![
        Spans::from(vec![
            Span::raw("Server is not connected"),
        ]),
        Spans::default(),
        Spans::from(vec![
            Span::raw("Host: "),
            Span::styled(host, Style::default().fg(Color::Yellow)),
        ]),
        Spans::default(),
        Spans::from(vec![
            Span::raw("Press "),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::raw(" to connect"),
        ]),
    ];

    let title = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default().title(format!(" {} ", title)).borders(Borders::ALL));

    f.render_widget(title, area);
}

fn ui_server_connected<B>(f: &mut Frame<B>, area: Rect, server: &mut RedisServer)
where
    B: Backend,
{
    let title = server.name.clone();
    let host = server.host.clone();

    let session = server
        .get_session_mut()
        .expect("Server is not connected");

    while !session.done() && area.height > session.count() as u16 {
        session.next().expect("to get next");
    }

    let filter = session.pattern.clone();

    let widths = vec![
        Constraint::Length(area.width - 8),
        Constraint::Length(6),
    ];

    let key_list = Table::new(
        session.iter_keys()
            .map(|(key, meta)| Row::new(vec![
                key.to_string(),
                meta.ttl_as_human_delta(),
            ]))
            .collect::<Vec<Row>>()
    )
    .header(
        Row::new(vec!["Key", "TTL"])
            .style(Style::default().fg(Color::Yellow))
    )
    .block(
        Block::default()
            .title(format!(" {} - {} ", title, host))
            .title(
                Title::from(
                    Spans::from(vec![
                        Span::styled(
                            " f ",
                            Style::default().fg(Color::Yellow)
                        ),
                        Span::raw(filter),
                        Span::raw(" "),
                    ])
                ).alignment(Alignment::Right)
            )
            .borders(Borders::ALL)
    )
    .widths(widths.as_ref())
    .highlight_style(Style::default().fg(Color::Cyan).add_modifier(tui::style::Modifier::BOLD));

    f.render_stateful_widget(key_list, area, &mut session.table_state);
}

fn ui_view_key<B>(f: &mut Frame<B>, area: Rect, server: &mut RedisServer)
where
    B: Backend,
{
    let title = server.name.clone();

    let session = server
        .get_session_mut()
        .expect("Server is not connected");

    let key = session.viewing_key.clone().expect("to get viewing key");
    let key_value = session.get_viewing_key().expect("to get viewing key value");
    let key_value_pretty = {
        let key_value_parsed: Value = from_str(key_value.as_str())
            .unwrap_or(Value::String(key_value.clone()));
        to_string_pretty(&key_value_parsed).unwrap_or(key_value.clone())
    };

    let view = Paragraph::new(key_value_pretty)
        .block(
            Block::default()
            .title(format!(" {} - {} ", title, key))
            .borders(Borders::ALL)
        )
        .wrap(Wrap { trim: false })
        .scroll((session.viewing_key_scroll, 0));

    f.render_widget(view, area);
}

fn ui_server<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let server = app.get_current_server_mut();

    if server.is_connected() {
        if !server.get_session().expect("to get session").viewing_key.is_none() {
            ui_view_key(f, area, server);
        } else {
            ui_server_connected(f, area, server);
        }
    } else {
        ui_server_disconnected(f, area, server);
    }
}

fn ui_body<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    ui_server(f, area, app);
}

fn ui_filter<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let filter = app.filter.clone();

    let paragraph = Paragraph::new(filter)
        .block(Block::default().borders(Borders::ALL).title(" Filter "));

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area)
}

pub fn ui<B>(f: &mut Frame<B>, app: &mut App)
where
    B: Backend,
{
    let size = f.size();
    let chunks = Layout::default()
        .direction(tui::layout::Direction::Vertical)
        .margin(1)
        .constraints([
           Constraint::Length(3),
           Constraint::Min(10)
        ].as_ref())
        .split(size);

    ui_tabs(f, chunks[0], app);
    ui_body(f, chunks[1], app);

    if app.entering_filter {
        // Show filter input on top of everything
        //   Create layout for centered box
        let vert = Layout::default()
            .direction(tui::layout::Direction::Vertical)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Length(3),
                Constraint::Percentage(50),
            ].as_ref())
            .split(size);

        let chunk = Layout::default()
            .direction(tui::layout::Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(50),
                Constraint::Percentage(25),
            ].as_ref())
            .split(vert[1])[1];

        ui_filter(f, chunk, app);
    }
}
