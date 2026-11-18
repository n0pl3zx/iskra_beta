//! Сервер «Искры»: TCP + JSON-строки, хранение в JSON-файле.
//! Работает как отдельный процесс или внутри клиентского приложения.

use crate::protocol::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRec {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub salt: String,
    pub hash: String,
    pub color: u8,
    pub last_seen: i64,
    pub created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Db {
    pub users: Vec<UserRec>,
    pub messages: Vec<Message>,
    /// "кому:с_кем" -> время, до которого сообщения прочитаны
    pub reads: HashMap<String, i64>,
    pub next_user: u64,
    pub next_msg: u64,
}

pub struct Server {
    db: Mutex<Db>,
    /// id пользователя -> [(id соединения, канал отправки)]
    conns: Mutex<HashMap<u64, Vec<(u64, Sender<ServerMsg>)>>>,
    path: PathBuf,
    next_conn: std::sync::atomic::AtomicU64,
}

impl Server {
    pub fn new(path: PathBuf) -> Arc<Server> {
        let db = load_db(&path);
        Arc::new(Server {
            db: Mutex::new(db),
            conns: Mutex::new(HashMap::new()),
            path,
            next_conn: std::sync::atomic::AtomicU64::new(1),
        })
    }

    fn save(&self, db: &Db) {
        if let Some(dir) = self.path.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string(db) {
            let tmp = self.path.with_extension("tmp");
            if fs::write(&tmp, json).is_ok() {
                let _ = fs::rename(&tmp, &self.path);
            }
        }
    }

    fn online_ids(&self) -> Vec<u64> {
        self.conns.lock().unwrap().keys().copied().collect()
    }

    fn send_to(&self, user: u64, msg: &ServerMsg) {
        let conns = self.conns.lock().unwrap();
        if let Some(list) = conns.get(&user) {
            for (_, tx) in list {
                let _ = tx.send(msg.clone());
            }
        }
    }

    fn broadcast(&self, msg: &ServerMsg) {
        let conns = self.conns.lock().unwrap();
        for list in conns.values() {
            for (_, tx) in list {
                let _ = tx.send(msg.clone());
            }
        }
    }

    /// Собирает список диалогов для пользователя.
    fn chats_for(&self, me_id: u64) -> Vec<ChatSummary> {
        let online = self.online_ids();
        let db = self.db.lock().unwrap();
        let mut out = Vec::new();
        for peer in db.users.iter().filter(|u| u.id != me_id) {
            let read_key = format!("{}:{}", me_id, peer.id);
            let read_ts = *db.reads.get(&read_key).unwrap_or(&0);
            let unread = db
                .messages
                .iter()
                .filter(|m| m.from == peer.id && m.to == me_id && m.ts > read_ts)
                .count() as u32;
            let last = db
                .messages
                .iter()
                .filter(|m| {
                    (m.from == me_id && m.to == peer.id) || (m.from == peer.id && m.to == me_id)
                })
                .max_by_key(|m| m.id);
            out.push(ChatSummary {
                user: User {
                    id: peer.id,
                    username: peer.username.clone(),
                    name: peer.name.clone(),
                    color: peer.color,
                    last_seen: peer.last_seen,
                },
                last_text: last.map(|m| m.text.clone()).unwrap_or_default(),
                last_ts: last.map(|m| m.ts).unwrap_or(0),
                last_from: last.map(|m| m.from).unwrap_or(0),
                unread,
                online: online.contains(&peer.id),
            });
        }
        out.sort_by(|a, b| {
            let ka = if a.last_ts > 0 { a.last_ts } else { i64::MIN + a.user.id as i64 };
            let kb = if b.last_ts > 0 { b.last_ts } else { i64::MIN + b.user.id as i64 };
            kb.cmp(&ka)
        });
        out
    }

    fn broadcast_chats(&self) {
        let ids = self.online_ids();
        for id in ids {
            let chats = self.chats_for(id);
            self.send_to(id, &ServerMsg::Chats { chats });
        }
    }
}

fn load_db(path: &Path) -> Db {
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(db) = serde_json::from_str::<Db>(&data) {
            return db;
        }
    }
    Db::default()
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn gen_salt() -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    SystemTime::now().hash(&mut h);
    std::process::id().hash(&mut h);
    std::thread::current().id().hash(&mut h);
    format!("{:016x}", h.finish())
}

fn hash_password(password: &str, salt: &str) -> String {
    let mut digest = {
        let mut h = Sha256::new();
        h.update(salt.as_bytes());
        h.update(password.as_bytes());
        h.finalize()
    };
    for _ in 0..4096 {
        let mut h = Sha256::new();
        h.update(digest);
        h.update(salt.as_bytes());
        h.update(password.as_bytes());
        digest = h.finalize();
    }
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

fn valid_username(s: &str) -> bool {
    let len = s.chars().count();
    (3..=24).contains(&len)
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

fn send_line(stream: &mut TcpStream, msg: &ServerMsg) -> std::io::Result<()> {
    let line = serde_json::to_string(msg).unwrap_or_default();
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}

/// Пытается занять порт и запустить сервер.
/// Возвращает None, если порт уже занят (значит, сервер уже где-то работает).
pub fn try_start(port: u16) -> Option<Arc<Server>> {
    let listener = TcpListener::bind(("0.0.0.0", port)).ok()?;
    let server = Server::new(data_dir().join("server").join("db.json"));
    let srv = server.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let srv = srv.clone();
                    thread::spawn(move || handle_conn(srv, stream));
                }
                Err(_) => thread::sleep(Duration::from_millis(100)),
            }
        }
    });
    Some(server)
}

/// Режим «только сервер» (без окна).
pub fn run_headless(port: u16) {
    println!("=== {} — сервер ===", APP_NAME);
    println!("Порт: {}", port);
    println!("Данные: {}", data_dir().join("server").display());
    match try_start(port) {
        Some(_) => {
            if let Some(ip) = local_ip() {
                println!("Адрес для подключения: {}:{}", ip, port);
            }
            println!("Сервер запущен. Для остановки закройте окно или нажмите Ctrl+C.");
            loop {
                thread::sleep(Duration::from_secs(3600));
            }
        }
        None => {
            eprintln!("Не удалось занять порт {}. Возможно, сервер уже запущен.", port);
            thread::sleep(Duration::from_secs(5));
        }
    }
}

pub fn local_ip() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip().to_string())
}

fn handle_conn(server: Arc<Server>, stream: TcpStream) {
    let _ = stream.set_nodelay(true);
    let token = server
        .next_conn
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let read_half = match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut write_half = stream;
    let (tx, rx) = channel::<ServerMsg>();

    // Отдельный поток на запись, чтобы медленный клиент не блокировал сервер.
    thread::spawn(move || {
        for msg in rx {
            if send_line(&mut write_half, &msg).is_err() {
                break;
            }
        }
        let _ = write_half.shutdown(Shutdown::Both);
    });

    let mut user_id: Option<u64> = None;
    let reader = BufReader::new(read_half);
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<ClientMsg>(&line) {
            Ok(msg) => {
                if !handle_msg(&server, &mut user_id, &tx, token, msg) {
                    break;
                }
            }
            Err(_) => {
                let _ = tx.send(ServerMsg::Error {
                    message: "Некорректный запрос".into(),
                    fatal: false,
                });
            }
        }
    }

    if let Some(uid) = user_id {
        remove_conn(&server, uid, token);
    }
}

fn remove_conn(server: &Server, uid: u64, token: u64) {
    let mut conns = server.conns.lock().unwrap();
    let mut empty = false;
    if let Some(list) = conns.get_mut(&uid) {
        list.retain(|(id, _)| *id != token);
        empty = list.is_empty();
    }
    if empty {
        conns.remove(&uid);
    }
    drop(conns);
    if empty {
        let last_seen = unix_ms();
        {
            let mut db = server.db.lock().unwrap();
            if let Some(u) = db.users.iter_mut().find(|u| u.id == uid) {
                u.last_seen = last_seen;
            }
            server.save(&db);
        }
        server.broadcast(&ServerMsg::Presence {
            user_id: uid,
            online: false,
            last_seen,
        });
    }
}

/// Обрабатывает одно сообщение клиента. false — закрыть соединение.
fn handle_msg(
    server: &Arc<Server>,
    user_id: &mut Option<u64>,
    tx: &Sender<ServerMsg>,
    token: u64,
    msg: ClientMsg,
) -> bool {
    match msg {
        ClientMsg::Register {
            username,
            name,
            password,
        } => {
            if user_id.is_some() {
                return true;
            }
            let username = username.trim().to_lowercase();
            let name = name.trim().to_string();
            if !valid_username(&username) {
                let _ = tx.send(ServerMsg::Error {
                    message: "Имя пользователя: 3–24 символа, латиница, цифры, _ или .".into(),
                    fatal: true,
                });
                return false;
            }
            if password.chars().count() < 4 {
                let _ = tx.send(ServerMsg::Error {
                    message: "Пароль должен быть не короче 4 символов".into(),
                    fatal: true,
                });
                return false;
            }
            let display = if name.is_empty() { username.clone() } else { name };
            let id;
            {
                let mut db = server.db.lock().unwrap();
                if db
                    .users
                    .iter()
                    .any(|u| u.username.eq_ignore_ascii_case(&username))
                {
                    let _ = tx.send(ServerMsg::Error {
                        message: "Такое имя пользователя уже занято".into(),
                        fatal: true,
                    });
                    return false;
                }
                db.next_user += 1;
                id = db.next_user;
                let salt = gen_salt();
                let hash = hash_password(&password, &salt);
                db.users.push(UserRec {
                    id,
                    username,
                    name: display,
                    salt,
                    hash,
                    color: (id % 8) as u8,
                    last_seen: unix_ms(),
                    created: unix_ms(),
                });
                server.save(&db);
            }
            auth_done(server, user_id, tx, token, id);
            server.broadcast_chats();
            true
        }
        ClientMsg::Login { username, password } => {
            if user_id.is_some() {
                return true;
            }
            let username = username.trim().to_lowercase();
            let found = {
                let db = server.db.lock().unwrap();
                db.users
                    .iter()
                    .find(|u| u.username.eq_ignore_ascii_case(&username))
                    .map(|u| (u.id, u.salt.clone(), u.hash.clone()))
            };
            match found {
                Some((id, salt, hash)) if hash_password(&password, &salt) == hash => {
                    auth_done(server, user_id, tx, token, id);
                    true
                }
                Some(_) => {
                    let _ = tx.send(ServerMsg::Error {
                        message: "Неверный пароль".into(),
                        fatal: true,
                    });
                    false
                }
                None => {
                    let _ = tx.send(ServerMsg::Error {
                        message: "Пользователь не найден".into(),
                        fatal: true,
                    });
                    false
                }
            }
        }
        ClientMsg::Ping => true,
        _ => {
            let me = match user_id {
                Some(id) => *id,
                None => {
                    let _ = tx.send(ServerMsg::Error {
                        message: "Сначала войдите в аккаунт".into(),
                        fatal: true,
                    });
                    return false;
                }
            };
            match msg {
                ClientMsg::SendMessage { to, text } => {
                    let text = text.trim().to_string();
                    if text.is_empty() {
                        return true;
                    }
                    if text.chars().count() > 4000 {
                        let _ = tx.send(ServerMsg::Error {
                            message: "Сообщение слишком длинное".into(),
                            fatal: false,
                        });
                        return true;
                    }
                    let msg = {
                        let mut db = server.db.lock().unwrap();
                        if !db.users.iter().any(|u| u.id == to) || to == me {
                            return true;
                        }
                        db.next_msg += 1;
                        let m = Message {
                            id: db.next_msg,
                            from: me,
                            to,
                            text,
                            ts: unix_ms(),
                            read: false,
                        };
                        db.messages.push(m.clone());
                        server.save(&db);
                        m
                    };
                    // получателю — непрочитанное, отправителю — эхо
                    let mut incoming = msg.clone();
                    incoming.read = false;
                    server.send_to(to, &ServerMsg::Message { msg: incoming });
                    let read_key = format!("{}:{}", to, me);
                    let read_ts = *server
                        .db
                        .lock()
                        .unwrap()
                        .reads
                        .get(&read_key)
                        .unwrap_or(&0);
                    let mut echo = msg.clone();
                    echo.read = read_ts >= msg.ts;
                    server.send_to(me, &ServerMsg::Message { msg: echo });
                    server.broadcast_chats();
                    true
                }
                ClientMsg::History {
                    with,
                    before_id,
                    limit,
                } => {
                    let limit = limit.clamp(1, 500) as usize;
                    let db = server.db.lock().unwrap();
                    let peer_read = *db
                        .reads
                        .get(&format!("{}:{}", with, me))
                        .unwrap_or(&0);
                    let mut list: Vec<Message> = db
                        .messages
                        .iter()
                        .filter(|m| {
                            (m.from == me && m.to == with) || (m.from == with && m.to == me)
                        })
                        .filter(|m| before_id.map(|b| m.id < b).unwrap_or(true))
                        .cloned()
                        .collect();
                    list.sort_by_key(|m| m.id);
                    let has_more = list.len() > limit;
                    if has_more {
                        list = list.split_off(list.len() - limit);
                    }
                    for m in list.iter_mut() {
                        if m.from == me {
                            m.read = peer_read >= m.ts;
                        }
                    }
                    drop(db);
                    let _ = tx.send(ServerMsg::History {
                        with,
                        messages: list,
                        has_more,
                    });
                    true
                }
                ClientMsg::Typing { to } => {
                    server.send_to(to, &ServerMsg::Typing { from: me });
                    true
                }
                ClientMsg::MarkRead { with } => {
                    let ts = unix_ms();
                    {
                        let mut db = server.db.lock().unwrap();
                        db.reads.insert(format!("{}:{}", me, with), ts);
                        server.save(&db);
                    }
                    server.send_to(with, &ServerMsg::Read { by: me, up_to: ts });
                    let chats = server.chats_for(me);
                    let _ = tx.send(ServerMsg::Chats { chats });
                    true
                }
                _ => true,
            }
        }
    }
}

fn auth_done(
    server: &Arc<Server>,
    user_id: &mut Option<u64>,
    tx: &Sender<ServerMsg>,
    token: u64,
    id: u64,
) {
    *user_id = Some(id);
    {
        let mut conns = server.conns.lock().unwrap();
        conns.entry(id).or_default().push((token, tx.clone()));
    }
    let me = {
        let mut db = server.db.lock().unwrap();
        let now = unix_ms();
        let me = db.users.iter_mut().find(|u| u.id == id).map(|u| {
            u.last_seen = now;
            User {
                id: u.id,
                username: u.username.clone(),
                name: u.name.clone(),
                color: u.color,
                last_seen: now,
            }
        });
        server.save(&db);
        me
    };
    let Some(me) = me else { return };
    let chats = server.chats_for(id);
    let _ = tx.send(ServerMsg::AuthOk {
        me: me.clone(),
        chats,
    });
    server.broadcast(&ServerMsg::Presence {
        user_id: id,
        online: true,
        last_seen: me.last_seen,
    });
    server.broadcast_chats();
}
