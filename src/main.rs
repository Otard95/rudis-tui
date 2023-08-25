use std::{
    process::exit,
    io::{self, Stdout},
    time::{Duration},
    sync::mpsc::{self}
};
use tui::{
    backend::{CrosstermBackend},
    Terminal,
};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

mod database;
mod ui;
mod app;
mod input;

fn setup_terminal() -> Result<(), ()> {
    enable_raw_mode().expect("can run in raw mode");
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)
        .expect("can run in alternate screen with mouse capture");
    Ok(())
}

fn cleanup_terminal() -> Result<(), ()> {
    disable_raw_mode().expect("can disable raw mode");
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
        .expect("can leave alternate screen and disable mouse capture");
    Ok(())
}

fn create_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, std::io::Error> {
    let backend = CrosstermBackend::new(io::stdout());
    return Terminal::new(backend);
}


fn draw_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut app::App) -> Result<(), std::io::Error> {
    terminal.draw(|f| ui::ui(f, app))?;
    Ok(())
}

fn main_redis() {
    let mut app = app::App::new(
        vec![
            // app::RedisServer::new(
            //     "SMB Pro".to_string(),
            //     "smb-redis-pro-001.rzt4nj.0001.eun1.cache.amazonaws.com".to_string(),
            //     6379,
            //     None,
            //     None,
            // ),
            // app::RedisServer::new(
            //     "SMB Pre".to_string(),
            //     "smb-redis-pre.rzt4nj.ng.0001.eun1.cache.amazonaws.com".to_string(),
            //     6379,
            //     None,
            //     None,
            // ),
            app::RedisServer::new(
                "SMB Dev".to_string(),
                "smb-redis-dev.wrh1jp.ng.0001.euw1.cache.amazonaws.com".to_string(),
                6379,
                None,
                None,
            )
        ],
        None
    );

    app.get_current_server_mut()
        .connect()
        .expect("can connect to server");

    app.get_current_server_mut()
        .get_session_mut()
        .expect("can get connection")
        .scan("*".to_string())
        .expect("can scan");

    let iter = app.get_current_server()
        .get_session()
        .expect("can get connection")
        .iter_keys();

    for (key, meta) in iter {
        println!("{:?} -> {:?}", key, meta);
    }
}

fn main_rudis() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = mpsc::channel::<input::Event<KeyEvent>>();
    let tick_rate = Duration::from_millis(200);

    input::start_input_thread(tx, tick_rate);

    setup_terminal().or_else(|_| -> Result<(), io::Error> {
        cleanup_terminal().expect("can cleanup terminal");
        exit(1);
    })?;
    let mut terminal = create_terminal().or_else(|e| {
        cleanup_terminal().expect("can cleanup terminal");
        Err(e)
    })?;

    let mut app = app::App::new(
        vec![
            app::RedisServer::new(
                "SMB Pro".to_string(),
                "smb-redis-pro-001.rzt4nj.0001.eun1.cache.amazonaws.com".to_string(),
                6379,
                None,
                None,
            ),
            app::RedisServer::new(
                "SMB Pre".to_string(),
                "smb-redis-pre.rzt4nj.ng.0001.eun1.cache.amazonaws.com".to_string(),
                6379,
                None,
                None,
            ),
            app::RedisServer::new(
                "SMB Dev".to_string(),
                "smb-redis-dev.wrh1jp.ng.0001.euw1.cache.amazonaws.com".to_string(),
                6379,
                None,
                None,
            )
        ],
        None
    );


    while app.running {
        draw_terminal(&mut terminal, &mut app).or_else(|e| {
            cleanup_terminal().expect("can cleanup terminal");
            Err(e)
        })?;

        input::handle_input(&mut app, &rx);
    }

    cleanup_terminal().expect("can cleanup terminal");
    terminal.show_cursor()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)
        .expect("can leave alternate screen and disable mouse capture");

    Ok(())
}

fn main_db() {
    let db = database::DB::DB_V1_0(database::DB_V1_0 {
        version: database::DBVersions::V1_0,
        server_configs: Vec::new(),
    });

    println!("{:?}", db);
}

fn main() {
    main_rudis().expect("can run rudis");
    // main_db();
    // main_redis();
}
