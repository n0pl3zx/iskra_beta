"""Проверка протокола сервера «Искры» без GUI."""
import json
import os
import shutil
import socket
import subprocess
import sys
import time

# На Windows в CI стандартный вывод часто cp1252 — русские сообщения теста
# не печатаются. Явно включаем UTF-8, чтобы проверки не падали из-за локали.
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "testdata")
HOST = "127.0.0.1"
PORT = 0  # выбирается свободный, чтобы не пересечься с запущенным мессенджером


def find_exe():
    """Ищем уже собранную версию: она нужна, чтобы не поднимать сервер из-под cargo."""
    for name in (
        os.path.join(ROOT, "target", "release", "iskra.exe"),
        os.path.join(ROOT, "Iskra.exe"),
        os.path.join(ROOT, "target", "debug", "iskra.exe"),
    ):
        if os.path.exists(name):
            return name
    raise SystemExit("Сборка не найдена — сначала выполните: cargo build --release")


EXE = find_exe()


def free_port():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind((HOST, 0))
        return s.getsockname()[1]


def port_is_busy(port):
    try:
        with socket.create_connection((HOST, port), timeout=0.4):
            return True
    except OSError:
        return False


def ensure_isolated_server(server, port):
    """Проверяем, что тест говорит со своим сервером, а не с приложением.

    Иначе регистрации попадут в настоящую базу %APPDATA%\\Iskra.
    """
    expected = os.path.normcase(os.path.abspath(os.path.join(DATA, "server")))
    for _ in range(4):
        line = server.stdout.readline()
        if not line:
            break
        text = line.decode("utf-8", "replace").strip()
        if text.startswith("Данные:"):
            actual = text.split(":", 1)[1].strip()
            if os.path.normcase(os.path.abspath(actual)) != expected:
                raise AssertionError(f"сервер использует чужую базу: {actual}")
            break
    if server.poll() is not None:
        raise AssertionError(f"тестовый сервер не запустился (порт {port})")
    if not port_is_busy(port):
        raise AssertionError(f"тестовый сервер не слушает порт {port}")


class Client:
    def __init__(self, timeout=4.0):
        self.sock = socket.create_connection((HOST, PORT), timeout=3)
        self.sock.settimeout(0.25)
        self.buf = b""
        self.timeout = timeout
        self.events = []

    def send(self, obj):
        line = json.dumps(obj, ensure_ascii=False).encode("utf-8") + b"\n"
        self.sock.sendall(line)

    def _read_one(self):
        while b"\n" not in self.buf:
            chunk = self.sock.recv(65536)
            if not chunk:
                raise RuntimeError("соединение закрыто")
            self.buf += chunk
        line, self.buf = self.buf.split(b"\n", 1)
        return json.loads(line.decode("utf-8"))

    def wait(self, kind, timeout=None, **fields):
        deadline = time.time() + (timeout or self.timeout)
        for i, ev in enumerate(self.events):
            if ev.get("t") == kind and all(ev.get("d", {}).get(k) == v for k, v in fields.items()):
                return self.events.pop(i)
        while time.time() < deadline:
            try:
                ev = self._read_one()
            except socket.timeout:
                continue
            if ev.get("t") == kind and all(ev.get("d", {}).get(k) == v for k, v in fields.items()):
                return ev
            self.events.append(ev)
        raise AssertionError(f"не дождались {kind} {fields}, накопилось: {self.events}")

    def drain(self, seconds=0.4):
        end = time.time() + seconds
        while time.time() < end:
            try:
                self.events.append(self._read_one())
            except socket.timeout:
                pass

    def close(self):
        try:
            self.sock.close()
        except OSError:
            pass


def ok(msg):
    print("  OK  " + msg)


def main():
    global PORT
    PORT = free_port()
    if port_is_busy(PORT):  # на всякий случай, окно между bind и connect
        print(f"ПРОВАЛ: порт {PORT} занят, повторите запуск")
        sys.exit(2)
    shutil.rmtree(DATA, ignore_errors=True)
    env = dict(os.environ, ISKRA_DATA=DATA, ISKRA_PORT=str(PORT))
    server = subprocess.Popen(
        [EXE, "--server"],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    time.sleep(1.2)
    try:
        ensure_isolated_server(server, PORT)
        a = Client()
        a.send({"t": "Register", "d": {"username": "alice", "name": "Алиса", "password": "1234"}})
        ev = a.wait("AuthOk")
        assert ev["d"]["me"]["username"] == "alice", ev
        assert ev["d"]["chats"] == [], ev
        ok("регистрация первого аккаунта")

        b = Client()
        b.send({"t": "Register", "d": {"username": "bob", "name": "Боб", "password": "1234"}})
        ev = b.wait("AuthOk")
        bob_id = ev["d"]["me"]["id"]
        assert [c["user"]["username"] for c in ev["d"]["chats"]] == ["alice"], ev
        ok("второй аккаунт видит первого в контактах")

        a.drain()
        a.send({"t": "Login", "d": {"username": "alice", "password": "1234"}})
        # повторный вход в том же соединении игнорируется сервером — это нормально
        a2 = Client()
        a2.send({"t": "Login", "d": {"username": "alice", "password": "1234"}})
        me = a2.wait("AuthOk")["d"]["me"]
        alice_id = me["id"]
        ok("вход по паролю")

        b.send({"t": "SendMessage", "d": {"to": alice_id, "text": "Привет, Алиса!"}})
        echo = b.wait("Message")["d"]["msg"]
        assert echo["text"] == "Привет, Алиса!" and echo["from"] == bob_id
        incoming = a2.wait("Message")["d"]["msg"]
        assert incoming["from"] == bob_id and not incoming["read"]
        ok("доставка сообщения в реальном времени")

        a2.send({"t": "History", "d": {"with": bob_id, "before_id": None, "limit": 50}})
        hist = a2.wait("History")
        assert len(hist["d"]["messages"]) == 1, hist
        assert hist["d"]["messages"][0]["text"] == "Привет, Алиса!"
        ok("история переписки")

        b.wait("Chats")
        b.drain()
        a2.send({"t": "MarkRead", "d": {"with": bob_id}})
        read = b.wait("Read")
        assert read["d"]["by"] == alice_id
        ok("отметка о прочтении доходит до автора")

        b.send({"t": "SendMessage", "d": {"to": alice_id, "text": "Второе сообщение"}})
        b.wait("Message")
        a2.wait("Message")
        a2.send({"t": "MarkRead", "d": {"with": bob_id}})
        b.wait("Read")
        b.send({"t": "History", "d": {"with": alice_id, "before_id": None, "limit": 50}})
        hist = b.wait("History")
        assert all(m["read"] for m in hist["d"]["messages"] if m["from"] == bob_id), hist
        ok("галочки прочтения в истории")

        bad = Client()
        bad.send({"t": "Login", "d": {"username": "bob", "password": "wrong"}})
        ev = bad.wait("Error")
        assert ev["d"]["fatal"] is True, ev
        ok("неверный пароль отклоняется")

        a.close()
        bad.close()
        b.close()
        a2.close()

        time.sleep(0.4)
        server.terminate()
        server.wait(timeout=10)

        server = subprocess.Popen(
            [EXE, "--server"], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT
        )
        time.sleep(1.2)
        c = Client()
        c.send({"t": "Login", "d": {"username": "alice", "password": "1234"}})
        c.wait("AuthOk")
        c.send({"t": "History", "d": {"with": bob_id, "before_id": None, "limit": 50}})
        hist = c.wait("History")
        assert len(hist["d"]["messages"]) == 2, hist
        ok("данные сохраняются между запусками сервера")
        c.close()
        print("\nВСЕ ПРОВЕРКИ ПРОЙДЕНЫ")
    finally:
        try:
            server.terminate()
            server.wait(timeout=10)
        except Exception:
            pass


if __name__ == "__main__":
    try:
        main()
    except AssertionError as exc:
        print("ПРОВАЛ:", exc)
        sys.exit(1)
