use std::collections::HashMap;
use redis::{
    self,
    RedisResult
};
use crossterm::event::{KeyEvent, KeyCode};
use tui::widgets::TableState;

pub struct App {
    servers: Vec<RedisServer>,
    current_tab: usize,
    pub running: bool,
    pub entering_filter: bool,
    pub filter: String,
}

impl App {
    pub fn new(servers: Vec<RedisServer>, initial_tab: Option<usize>) -> App {
        App {
            servers,
            current_tab: initial_tab.unwrap_or(0),
            running: true,
            entering_filter: false,
            filter: "".to_string(),
        }
    }

    pub fn set_tab(&mut self, tab: usize) {
        self.current_tab = tab.clamp(0, self.servers.len() - 1);
    }

    pub fn next_tab(&mut self) {
        self.set_tab(self.current_tab + 1);
    }

    pub fn prev_tab(&mut self) {
        self.set_tab(self.current_tab - 1);
    }

    pub fn current_tab(&self) -> usize {
        self.current_tab
    }

    pub fn get_servers(&self) -> &Vec<RedisServer> {
        &self.servers
    }

    pub fn get_servers_mut(&mut self) -> &mut Vec<RedisServer> {
        &mut self.servers
    }

    pub fn push_server(&mut self, server: RedisServer) {
        self.servers.push(server);
    }

    pub fn get_current_server(&self) -> &RedisServer {
        &self.servers[self.current_tab]
    }

    pub fn get_current_server_mut(&mut self) -> &mut RedisServer {
        &mut self.servers[self.current_tab]
    }

    pub fn handle_input(&mut self, input: KeyEvent) {
        if self.entering_filter {
            match input.code {
                KeyCode::Esc => {
                    self.entering_filter = false;
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Char(c) => {
                    self.filter.push(c);
                }
                KeyCode::Enter => {
                    let filter = self.filter.clone();
                    let current_server = self.get_current_server_mut();
                    if current_server.is_connected() {
                        let session = current_server.get_session_mut().unwrap();
                        session.scan(filter).expect("to scan");
                    }
                    self.entering_filter = false;
                }
                _ => {}
            }
            return;
        }

        match input.code {
            KeyCode::Char('q') => {
                let current_server = self.get_current_server_mut();
                if current_server.is_connected() {
                    let session = current_server.get_session_mut().unwrap();
                    if session.viewing_key.is_some() {
                        session.viewing_key = None;
                        return;
                    }
                }
                self.running = false;
            }
            KeyCode::Char('h') => self.prev_tab(),
            KeyCode::Char('l') => self.next_tab(),
            KeyCode::Char('j') => {
                let current_server = self.get_current_server_mut();
                if current_server.is_connected() {
                    let session = current_server.get_session_mut().unwrap();
                    if session.viewing_key.is_some() {
                        session.viewing_key_scroll += 1;
                    } else {
                        session.select_next();
                    }
                }
            }
            KeyCode::Char('k') => {
                let current_server = self.get_current_server_mut();
                if current_server.is_connected() {
                    let session = current_server.get_session_mut().unwrap();
                    if session.viewing_key.is_some() {
                        session.viewing_key_scroll = session.viewing_key_scroll.saturating_sub(1);
                    } else {
                        session.select_prev();
                    }
                }
            }
            KeyCode::Char('c') => {
                self.get_current_server_mut().connect().expect("can connect to server");
            }
            KeyCode::Char('f') => {
                self.entering_filter = true;
                let current_session_pattern = self
                    .get_current_server()
                    .get_session()
                    .unwrap()
                    .pattern
                    .clone();
                self.filter = current_session_pattern;
            }
            KeyCode::Enter => {
                let current_server = self.get_current_server_mut();
                if current_server.is_connected() {
                    let session = current_server.get_session_mut().unwrap();
                    let selected = session.table_state.selected();
                    if let Some(selected) = selected {
                        let (key, _) = session.iter_keys().nth(selected).unwrap();
                        session.viewing_key = Some(key.clone());
                    }
                }
            }
            KeyCode::Esc => {
                let current_server = self.get_current_server_mut();
                if current_server.is_connected() {
                    let session = current_server.get_session_mut().unwrap();
                    session.viewing_key = None;
                }
            }
            _ => {}
        }
    }
}

pub struct RedisServer {
    pub name: String,
    pub host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    session: Option<RedisSession>,
}

impl RedisServer {
    pub fn new(name: String, host: String, port: u16, username: Option<String>, password: Option<String>) -> RedisServer {
        RedisServer {
            name,
            host,
            port,
            username,
            password,
            session: None,
        }
    }

    pub fn connect(&mut self) -> Result<(), redis::RedisError> {
        if self.is_connected() { return Ok(()); }

        let client = redis::Client::open(format!("redis://{}:{}", self.host, self.port))?;
        let con = client.get_connection()?;
        let mut session = RedisSession {
            client,
            con,
            pattern: "*".to_string(),
            keys: HashMap::new(),
            cursor: 0,
            table_state: TableState::default(),
            viewing_key: None,
            viewing_key_scroll: 0,
        };
        session.scan("*".to_string())?;
        self.session = Some(session);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.session = None;
    }

    pub fn is_connected(&self) -> bool {
        self.session.is_some()
    }

    pub fn get_session(&self) -> Option<&RedisSession> {
        self.session.as_ref()
    }

    pub fn get_session_mut(&mut self) -> Option<&mut RedisSession> {
        self.session.as_mut()
    }
}

#[derive(Debug)]
pub struct KeyMetadata {
    _type: Option<String>,
    ttl: Option<u64>,
    size: Option<u64>,
}

impl KeyMetadata {
    pub fn value_type(&self) -> String {
        match self._type {
            Some(ref t) => t.to_string(),
            None => "-".to_string(),
        }
    }

    pub fn ttl_as_human_delta(&self) -> String {
        match self.ttl {
            Some(ttl) => {
                let minute_threshold = 50;
                let hour_threshold = 50 * 60;
                let day_threshold = 23 * 60 * 60;
                let week_threshold = 6 * 24 * 60 * 60;
                let month_threshold = 4 * 7 * 24 * 60 * 60;
                let year_threshold = 12 * 30 * 24 * 60 * 60;

                if ttl > year_threshold {
                    format!("{}y", ttl / year_threshold)
                } else if ttl > month_threshold {
                    format!("{}M", ttl / month_threshold)
                } else if ttl > week_threshold {
                    format!("{}w", ttl / week_threshold)
                } else if ttl > day_threshold {
                    format!("{}d", ttl / day_threshold)
                } else if ttl > hour_threshold {
                    format!("{}h", ttl / hour_threshold)
                } else if ttl > minute_threshold {
                    format!("{}m", ttl / minute_threshold)
                } else {
                    format!("{}s", ttl)
                }
            }
            None => "N/A".to_string(),
        }
    }

    pub fn size_as_human(&self) -> String {
        match self.size {
            Some(size) => {
                let kb_threshold = 1024;
                let mb_threshold = 1024 * 1024;
                let gb_threshold = 1024 * 1024 * 1024;

                if size < kb_threshold {
                    format!("{}B", size)
                } else if size < mb_threshold {
                    format!("{}KB", size / kb_threshold)
                } else if size < gb_threshold {
                    format!("{}MB", size / mb_threshold)
                } else {
                    format!("{}GB", size / gb_threshold)
                }
            }
            None => "-".to_string(),
        }
    }
}

pub struct RedisSession {
    client: redis::Client,
    con: redis::Connection,
    pub pattern: String,
    keys: HashMap<String, KeyMetadata>,
    cursor: u64,
    pub table_state: TableState,
    pub viewing_key: Option<String>,
    pub viewing_key_scroll: u16,
    pub loading: bool,
}

impl RedisSession {
    fn get_next(&mut self) -> Result<(), redis::RedisError> {
        if self.loading {
            return Ok(());
        }
        self.loading = true;

        std::thread::spawn(|| {
        });

        // Do a SCAN command
        let result: RedisResult<(u64, Vec<String>)> = redis::cmd("SCAN")
            .cursor_arg(self.cursor)
            .arg("MATCH")
            .arg(&self.pattern)
            .query(&mut self.con);

        match result {
            Ok((new_cursor, keys)) => {
                self.cursor = new_cursor;
                for key in keys {
                    let ttl: Option<u64> = redis::cmd("TTL")
                        .arg(&key)
                        .query(&mut self.con)
                        .unwrap_or(None);
                    self.keys.insert(key, KeyMetadata { _type: None, ttl, size: None });
                }
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn scan(&mut self, pattern: String) -> Result<(), redis::RedisError> {
        if pattern != self.pattern {
            self.pattern = pattern;
            self.keys.clear();
            self.cursor = 0;
        }

        self.get_next()
    }

    pub fn next(&mut self) -> Result<(), redis::RedisError> {
        if self.cursor == 0 {
            return Ok(());
        }

        self.get_next()
    }

    pub fn iter_keys(&self) -> impl Iterator<Item = (&String, &KeyMetadata)> {
        self.keys.iter()
    }

    pub fn count(&self) -> usize {
        self.keys.len()
    }

    pub fn done(&self) -> bool {
        self.cursor == 0
    }

    pub fn select_next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.count() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn select_prev(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.count() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn get_viewing_key(&mut self) -> Option<String> {
        let result: RedisResult<String> = redis::cmd("GET")
            .arg(self.viewing_key.as_ref().unwrap())
            .query(&mut self.con);

        return result.ok();
    }
}
