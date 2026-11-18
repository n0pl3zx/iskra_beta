//! Сетевой клиент: держит соединение, переподключается, шлёт команды из UI.

use crate::protocol::*;
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum Auth {
    Login {
        username: String,
        password: String,
    },
    Register {
        username: String,
        name: String,
        password: String,
    },
}

impl Auth {
    fn to_msg(&self) -> ClientMsg {
        match self {
            Auth::Login { username, password } => ClientMsg::Login {
                username: username.clone(),
                password: password.clone(),
            },
            Auth::Register {
                username,
                name,
                password,
            } => ClientMsg::Register {
                username: username.clone(),
                name: name.clone(),
                password: password.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnStatus {
    Offline,
    Connecting,
    Online,
}

#[derive(Debug, Clone)]
pub enum NetEvent {
    Status(ConnStatus),
    Notice(String),
    AuthFailed(String),
    Server(ServerMsg),
}

pub enum UiCmd {
    Connect { addr: String, auth: Auth },
    Send(ClientMsg),
    Disconnect,
}

enum SrvEvent {
    Msg(ServerMsg),
    Closed,
}

pub struct NetClient {
    cmd_tx: Sender<UiCmd>,
    ev_rx: Receiver<NetEvent>,
}

impl NetClient {
    pub fn new(ctx: egui::Context) -> Self {
        let (cmd_tx, cmd_rx) = channel::<UiCmd>();
        let (ev_tx, ev_rx) = channel::<NetEvent>();
        thread::Builder::new()
            .name("iskra-net".into())
            .spawn(move || worker(ctx, cmd_rx, ev_tx))
            .ok();
        NetClient { cmd_tx, ev_rx }
    }

    pub fn connect(&self, addr: String, auth: Auth) {
        let _ = self.cmd_tx.send(UiCmd::Connect { addr, auth });
    }

    pub fn send(&self, msg: ClientMsg) {
        let _ = self.cmd_tx.send(UiCmd::Send(msg));
    }

    pub fn disconnect(&self) {
        let _ = self.cmd_tx.send(UiCmd::Disconnect);
    }

    pub fn poll(&self) -> Vec<NetEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.ev_rx.try_recv() {
            out.push(ev);
        }
        out
    }
}

fn io_text(e: &std::io::Error) -> String {
    let s = crate::i18n::s();
    match e.kind() {
        std::io::ErrorKind::ConnectionRefused => s.connection_refused.into(),
        std::io::ErrorKind::TimedOut => s.timeout.into(),
        std::io::ErrorKind::HostUnreachable | std::io::ErrorKind::NetworkUnreachable => {
            s.network_unreachable.into()
        }
        _ => e.to_string(),
    }
}

fn send_line(stream: &mut TcpStream, msg: &ClientMsg) -> std::io::Result<()> {
    let line = serde_json::to_string(msg).unwrap_or_default();
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}

/// Ждёт команду не дольше `ms`, обслуживая пришедшие команды.
fn wait_cmd(cmds: &Receiver<UiCmd>, ms: u64) -> Option<UiCmd> {
    let deadline = Instant::now() + Duration::from_millis(ms);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        match cmds.recv_timeout(left.min(Duration::from_millis(120))) {
            Ok(cmd) => return Some(cmd),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Some(UiCmd::Disconnect),
        }
    }
}

fn worker(ctx: egui::Context, cmds: Receiver<UiCmd>, events: Sender<NetEvent>) {
    let mut target: Option<(String, Auth)> = None;

    'main: loop {
        if target.is_none() {
            match cmds.recv() {
                Ok(UiCmd::Connect { addr, auth }) => target = Some((addr, auth)),
                Ok(_) => continue 'main,
                Err(_) => return,
            }
        }
        let (addr, auth) = target.clone().unwrap();

        let _ = events.send(NetEvent::Status(ConnStatus::Connecting));
        ctx.request_repaint();

        let stream = match TcpStream::connect(&addr) {
            Ok(s) => s,
            Err(e) => {
                let _ = events.send(NetEvent::Status(ConnStatus::Offline));
                let _ = events.send(NetEvent::Notice(
                    crate::i18n::s()
                        .server_unavailable
                        .replacen("{}", &addr, 1)
                        .replacen("{}", &io_text(&e), 1),
                ));
                ctx.request_repaint();
                match wait_cmd(&cmds, 1500) {
                    Some(UiCmd::Connect { addr, auth }) => target = Some((addr, auth)),
                    Some(UiCmd::Disconnect) => target = None,
                    _ => {}
                }
                continue 'main;
            }
        };
        let _ = stream.set_nodelay(true);

        let read_half = match stream.try_clone() {
            Ok(s) => s,
            Err(_) => {
                target = None;
                continue 'main;
            }
        };
        let mut write_half = stream;

        let (srv_tx, srv_rx) = channel::<SrvEvent>();
        {
            let ctx = ctx.clone();
            thread::spawn(move || {
                let reader = BufReader::new(read_half);
                for line in reader.lines() {
                    match line {
                        Ok(line) => {
                            if line.trim().is_empty() {
                                continue;
                            }
                            if let Ok(msg) = serde_json::from_str::<ServerMsg>(&line) {
                                if srv_tx.send(SrvEvent::Msg(msg)).is_err() {
                                    break;
                                }
                                ctx.request_repaint();
                            }
                        }
                        Err(_) => break,
                    }
                }
                let _ = srv_tx.send(SrvEvent::Closed);
                ctx.request_repaint();
            });
        }

        if send_line(&mut write_half, &auth.to_msg()).is_err() {
            let _ = events.send(NetEvent::Status(ConnStatus::Offline));
            let _ = write_half.shutdown(Shutdown::Both);
            match wait_cmd(&cmds, 1500) {
                Some(UiCmd::Connect { addr, auth }) => target = Some((addr, auth)),
                Some(UiCmd::Disconnect) => target = None,
                _ => {}
            }
            continue 'main;
        }

        let mut authenticated = false;
        let mut fatal: Option<String> = None;
        let mut last_ping = Instant::now();

        loop {
            match srv_rx.recv_timeout(Duration::from_millis(15)) {
                Ok(SrvEvent::Msg(m)) => {
                    if matches!(m, ServerMsg::AuthOk { .. }) {
                        authenticated = true;
                        let _ = events.send(NetEvent::Status(ConnStatus::Online));
                    }
                    if let ServerMsg::Error {
                        message,
                        fatal: is_fatal,
                    } = &m
                    {
                        if *is_fatal {
                            fatal = Some(message.clone());
                        }
                    }
                    let _ = events.send(NetEvent::Server(m));
                    ctx.request_repaint();
                }
                Ok(SrvEvent::Closed) => break,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }

            if last_ping.elapsed() > Duration::from_secs(20) {
                let _ = send_line(&mut write_half, &ClientMsg::Ping);
                last_ping = Instant::now();
            }

            match cmds.try_recv() {
                Ok(UiCmd::Send(m)) => {
                    if send_line(&mut write_half, &m).is_err() {
                        break;
                    }
                }
                Ok(UiCmd::Disconnect) => {
                    target = None;
                    let _ = write_half.shutdown(Shutdown::Both);
                    let _ = events.send(NetEvent::Status(ConnStatus::Offline));
                    continue 'main;
                }
                Ok(UiCmd::Connect { addr, auth }) => {
                    target = Some((addr, auth));
                    let _ = write_half.shutdown(Shutdown::Both);
                    continue 'main;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => return,
            }
        }

        let _ = write_half.shutdown(Shutdown::Both);
        drop(write_half);
        let _ = events.send(NetEvent::Status(ConnStatus::Offline));
        ctx.request_repaint();

        if let Some(msg) = fatal {
            let _ = events.send(NetEvent::AuthFailed(msg));
            target = None;
            continue 'main;
        }

        let _ = events.send(NetEvent::Notice(if authenticated {
            crate::i18n::s().connection_lost.into()
        } else {
            crate::i18n::s().server_silent.into()
        }));

        match wait_cmd(&cmds, 1200) {
            Some(UiCmd::Connect { addr, auth }) => target = Some((addr, auth)),
            Some(UiCmd::Disconnect) => target = None,
            _ => {}
        }
        continue 'main;
    }
}
