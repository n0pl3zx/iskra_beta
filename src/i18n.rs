//! Языки интерфейса. Все подписи хранятся рядом, чтобы легко добавить новый язык.

use std::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Ru,
    En,
    De,
    Es,
}

pub const LANGS: [Lang; 4] = [Lang::Ru, Lang::En, Lang::De, Lang::Es];

impl Lang {
    pub fn id(self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::En => "en",
            Lang::De => "de",
            Lang::Es => "es",
        }
    }

    pub fn from_id(id: &str) -> Option<Lang> {
        match id {
            "ru" => Some(Lang::Ru),
            "en" => Some(Lang::En),
            "de" => Some(Lang::De),
            "es" => Some(Lang::Es),
            _ => None,
        }
    }

    /// Название языка на нём самом.
    pub fn native(self) -> &'static str {
        match self {
            Lang::Ru => "Русский",
            Lang::En => "English",
            Lang::De => "Deutsch",
            Lang::Es => "Español",
        }
    }
}

pub struct Strings {
    pub app_name: &'static str,
    pub tagline: &'static str,

    pub label_server: &'static str,
    pub label_username: &'static str,
    pub label_your_name: &'static str,
    pub label_password: &'static str,
    pub hint_user: &'static str,
    pub hint_name: &'static str,
    pub hint_password: &'static str,

    pub err_username_required: &'static str,
    pub err_password_required: &'static str,
    pub err_username_format: &'static str,
    pub err_password_short: &'static str,

    pub connecting: &'static str,
    pub create_account: &'static str,
    pub sign_in: &'static str,
    pub have_account: &'static str,
    pub no_account: &'static str,
    pub switch_mode: &'static str,

    pub server_running: &'static str,
    pub friends_connect: &'static str,
    pub waiting: &'static str,
    pub connected: &'static str,

    pub search: &'static str,
    pub no_contacts: &'static str,

    pub online: &'static str,
    pub connecting_lower: &'static str,
    pub offline: &'static str,
    pub typing: &'static str,

    pub choose_chat: &'static str,
    pub contacts_hint: &'static str,
    pub empty_chat: &'static str,
    pub write_first: &'static str,
    pub show_older: &'static str,
    pub message_hint: &'static str,
    pub no_connection: &'static str,

    pub profile: &'static str,
    pub id_label: &'static str,
    pub server_label: &'static str,
    pub my_address: &'static str,
    pub logout: &'static str,

    pub settings: &'static str,
    pub settings_sub: &'static str,
    pub theme_section: &'static str,
    pub language_section: &'static str,
    pub interface_section: &'static str,
    pub scale_label: &'static str,
    pub animations_label: &'static str,
    pub animations_hint: &'static str,
    pub done: &'static str,
    pub copy_text: &'static str,

    pub you_prefix: &'static str,
    pub unread_suffix: &'static str,

    pub today: &'static str,
    pub yesterday: &'static str,
    pub yesterday_lower: &'static str,
    pub last_seen_at: &'static str,
    pub last_seen_yesterday: &'static str,
    pub last_seen_on: &'static str,
    pub last_seen_recently: &'static str,

    pub theme_graphite: &'static str,
    pub theme_graphite_hint: &'static str,
    pub theme_light: &'static str,
    pub theme_light_hint: &'static str,
    pub theme_emerald: &'static str,
    pub theme_emerald_hint: &'static str,
    pub theme_indigo: &'static str,
    pub theme_indigo_hint: &'static str,

    pub srv_bad_request: &'static str,
    pub srv_username_format: &'static str,
    pub srv_password_short: &'static str,
    pub srv_name_taken: &'static str,
    pub srv_wrong_password: &'static str,
    pub srv_user_not_found: &'static str,
    pub srv_login_first: &'static str,
    pub srv_too_long: &'static str,

    pub connection_refused: &'static str,
    pub timeout: &'static str,
    pub network_unreachable: &'static str,
    pub server_unavailable: &'static str,
    pub connection_lost: &'static str,
    pub server_silent: &'static str,

    pub months: [&'static str; 12],
}

const RU: Strings = Strings {
    app_name: "Искра",
    tagline: "быстрый мессенджер для своих",

    label_server: "Сервер",
    label_username: "Имя пользователя",
    label_your_name: "Ваше имя",
    label_password: "Пароль",
    hint_user: "например, dima",
    hint_name: "как вас показывать другим",
    hint_password: "не менее 4 символов",

    err_username_required: "Введите имя пользователя",
    err_password_required: "Введите пароль",
    err_username_format: "Имя: 3–24 символа, латиница, цифры, _ или .",
    err_password_short: "Пароль должен быть не короче 4 символов",

    connecting: "Подключение…",
    create_account: "Создать аккаунт",
    sign_in: "Войти",
    have_account: "Уже есть аккаунт?",
    no_account: "Нет аккаунта?",
    switch_mode: "Переключить",

    server_running: "Сервер запущен",
    friends_connect: "Друзья подключаются к вам по этому адресу",
    waiting: "Ожидание подключения",
    connected: "Подключено",

    search: "Поиск",
    no_contacts: "Пока никого нет рядом",

    online: "в сети",
    connecting_lower: "подключение…",
    offline: "нет соединения",
    typing: "печатает…",

    choose_chat: "Выберите, кому написать",
    contacts_hint: "Контакты и переписки — в списке слева",
    empty_chat: "Здесь пока пусто",
    write_first: "Напишите первым — @{}",
    show_older: "Показать более ранние",
    message_hint: "Сообщение",
    no_connection: "Нет соединения с сервером",

    profile: "Профиль",
    id_label: "ID",
    server_label: "Сервер",
    my_address: "Мой адрес",
    logout: "Выйти из аккаунта",

    settings: "Настройки",
    settings_sub: "Оформление и удобство",
    theme_section: "ТЕМА ОФОРМЛЕНИЯ",
    language_section: "ЯЗЫК",
    interface_section: "ИНТЕРФЕЙС",
    scale_label: "Масштаб",
    animations_label: "Плавные анимации",
    animations_hint: "Мягкие переходы и подсветка",
    done: "Готово",
    copy_text: "Копировать текст",

    you_prefix: "Вы: ",
    unread_suffix: "новых",

    today: "Сегодня",
    yesterday: "Вчера",
    yesterday_lower: "вчера",
    last_seen_at: "был(а) в {}",
    last_seen_yesterday: "был(а) вчера",
    last_seen_on: "был(а) {}",
    last_seen_recently: "был(а) недавно",

    theme_graphite: "Графит",
    theme_graphite_hint: "Тёмная монохромная",
    theme_light: "Светлая",
    theme_light_hint: "День, бумага",
    theme_emerald: "Изумруд",
    theme_emerald_hint: "Тёмная с зелёным",
    theme_indigo: "Индиго",
    theme_indigo_hint: "Тёмная с синим",

    srv_bad_request: "Некорректный запрос",
    srv_username_format: "Имя пользователя: 3–24 символа, латиница, цифры, _ или .",
    srv_password_short: "Пароль должен быть не короче 4 символов",
    srv_name_taken: "Такое имя пользователя уже занято",
    srv_wrong_password: "Неверный пароль",
    srv_user_not_found: "Пользователь не найден",
    srv_login_first: "Сначала войдите в аккаунт",
    srv_too_long: "Сообщение слишком длинное",

    connection_refused: "соединение отклонено",
    timeout: "таймаут",
    network_unreachable: "сеть недоступна",
    server_unavailable: "Сервер {} недоступен: {}",
    connection_lost: "Соединение потеряно, переподключаюсь…",
    server_silent: "Сервер не отвечает, переподключаюсь…",

    months: [
        "января",
        "февраля",
        "марта",
        "апреля",
        "мая",
        "июня",
        "июля",
        "августа",
        "сентября",
        "октября",
        "ноября",
        "декабря",
    ],
};

const EN: Strings = Strings {
    app_name: "Iskra",
    tagline: "a fast messenger for friends",

    label_server: "Server",
    label_username: "Username",
    label_your_name: "Your name",
    label_password: "Password",
    hint_user: "for example, alex",
    hint_name: "how others will see you",
    hint_password: "at least 4 characters",

    err_username_required: "Enter a username",
    err_password_required: "Enter a password",
    err_username_format: "Username: 3–24 characters, Latin letters, digits, _ or .",
    err_password_short: "Password must be at least 4 characters",

    connecting: "Connecting…",
    create_account: "Create account",
    sign_in: "Sign in",
    have_account: "Already have an account?",
    no_account: "No account yet?",
    switch_mode: "Switch",

    server_running: "Server running",
    friends_connect: "Friends connect to you at this address",
    waiting: "Waiting for connection",
    connected: "Connected",

    search: "Search",
    no_contacts: "No one around yet",

    online: "online",
    connecting_lower: "connecting…",
    offline: "offline",
    typing: "typing…",

    choose_chat: "Choose someone to write to",
    contacts_hint: "Contacts and chats are in the list on the left",
    empty_chat: "Nothing here yet",
    write_first: "Say hello first — @{}",
    show_older: "Show earlier messages",
    message_hint: "Message",
    no_connection: "No connection to the server",

    profile: "Profile",
    id_label: "ID",
    server_label: "Server",
    my_address: "My address",
    logout: "Sign out",

    settings: "Settings",
    settings_sub: "Appearance and comfort",
    theme_section: "APPEARANCE",
    language_section: "LANGUAGE",
    interface_section: "INTERFACE",
    scale_label: "Scale",
    animations_label: "Smooth animations",
    animations_hint: "Soft transitions and highlights",
    done: "Done",
    copy_text: "Copy text",

    you_prefix: "You: ",
    unread_suffix: "new",

    today: "Today",
    yesterday: "Yesterday",
    yesterday_lower: "yesterday",
    last_seen_at: "last seen at {}",
    last_seen_yesterday: "last seen yesterday",
    last_seen_on: "last seen on {}",
    last_seen_recently: "last seen recently",

    theme_graphite: "Graphite",
    theme_graphite_hint: "Dark monochrome",
    theme_light: "Light",
    theme_light_hint: "Daylight, paper",
    theme_emerald: "Emerald",
    theme_emerald_hint: "Dark with green",
    theme_indigo: "Indigo",
    theme_indigo_hint: "Dark with blue",

    srv_bad_request: "Malformed request",
    srv_username_format: "Username: 3–24 characters, Latin letters, digits, _ or .",
    srv_password_short: "Password must be at least 4 characters",
    srv_name_taken: "That username is already taken",
    srv_wrong_password: "Wrong password",
    srv_user_not_found: "User not found",
    srv_login_first: "Sign in first",
    srv_too_long: "Message is too long",

    connection_refused: "connection refused",
    timeout: "timeout",
    network_unreachable: "network unreachable",
    server_unavailable: "Server {} is unavailable: {}",
    connection_lost: "Connection lost, reconnecting…",
    server_silent: "Server is not responding, reconnecting…",

    months: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
};

const DE: Strings = Strings {
    app_name: "Iskra",
    tagline: "ein schneller Messenger für Freunde",

    label_server: "Server",
    label_username: "Benutzername",
    label_your_name: "Dein Name",
    label_password: "Passwort",
    hint_user: "zum Beispiel, alex",
    hint_name: "so sehen dich andere",
    hint_password: "mindestens 4 Zeichen",

    err_username_required: "Bitte Benutzernamen eingeben",
    err_password_required: "Bitte Passwort eingeben",
    err_username_format: "Name: 3–24 Zeichen, lateinische Buchstaben, Ziffern, _ oder .",
    err_password_short: "Das Passwort muss mindestens 4 Zeichen lang sein",

    connecting: "Verbinden…",
    create_account: "Konto erstellen",
    sign_in: "Anmelden",
    have_account: "Schon ein Konto?",
    no_account: "Noch kein Konto?",
    switch_mode: "Wechseln",

    server_running: "Server läuft",
    friends_connect: "Freunde verbinden sich über diese Adresse",
    waiting: "Warte auf Verbindung",
    connected: "Verbunden",

    search: "Suchen",
    no_contacts: "Noch niemand hier",

    online: "online",
    connecting_lower: "verbinde…",
    offline: "offline",
    typing: "schreibt…",

    choose_chat: "Wähle, wem du schreiben willst",
    contacts_hint: "Kontakte und Chats findest du links",
    empty_chat: "Hier ist noch nichts",
    write_first: "Schreib zuerst — @{}",
    show_older: "Ältere anzeigen",
    message_hint: "Nachricht",
    no_connection: "Keine Verbindung zum Server",

    profile: "Profil",
    id_label: "ID",
    server_label: "Server",
    my_address: "Meine Adresse",
    logout: "Abmelden",

    settings: "Einstellungen",
    settings_sub: "Aussehen und Komfort",
    theme_section: "ERSCHEINUNGSBILD",
    language_section: "SPRACHE",
    interface_section: "OBERFLÄCHE",
    scale_label: "Skalierung",
    animations_label: "Weiche Animationen",
    animations_hint: "Sanfte Übergänge und Effekte",
    done: "Fertig",
    copy_text: "Text kopieren",

    you_prefix: "Du: ",
    unread_suffix: "neu",

    today: "Heute",
    yesterday: "Gestern",
    yesterday_lower: "gestern",
    last_seen_at: "zuletzt online um {}",
    last_seen_yesterday: "gestern online",
    last_seen_on: "zuletzt online am {}",
    last_seen_recently: "zuletzt vor kurzem online",

    theme_graphite: "Graphit",
    theme_graphite_hint: "Dunkel, monochrom",
    theme_light: "Hell",
    theme_light_hint: "Tag, Papier",
    theme_emerald: "Smaragd",
    theme_emerald_hint: "Dunkel mit Grün",
    theme_indigo: "Indigo",
    theme_indigo_hint: "Dunkel mit Blau",

    srv_bad_request: "Ungültige Anfrage",
    srv_username_format: "Benutzername: 3–24 Zeichen, lateinische Buchstaben, Ziffern, _ oder .",
    srv_password_short: "Das Passwort muss mindestens 4 Zeichen lang sein",
    srv_name_taken: "Dieser Benutzername ist bereits vergeben",
    srv_wrong_password: "Falsches Passwort",
    srv_user_not_found: "Benutzer nicht gefunden",
    srv_login_first: "Bitte zuerst anmelden",
    srv_too_long: "Nachricht ist zu lang",

    connection_refused: "Verbindung abgelehnt",
    timeout: "Zeitüberschreitung",
    network_unreachable: "Netzwerk nicht erreichbar",
    server_unavailable: "Server {} ist nicht erreichbar: {}",
    connection_lost: "Verbindung verloren, verbinde neu…",
    server_silent: "Server antwortet nicht, verbinde neu…",

    months: [
        "Januar",
        "Februar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktober",
        "November",
        "Dezember",
    ],
};

const ES: Strings = Strings {
    app_name: "Iskra",
    tagline: "un mensajero rápido para los tuyos",

    label_server: "Servidor",
    label_username: "Usuario",
    label_your_name: "Tu nombre",
    label_password: "Contraseña",
    hint_user: "por ejemplo, alex",
    hint_name: "cómo te verán los demás",
    hint_password: "al menos 4 caracteres",

    err_username_required: "Introduce un usuario",
    err_password_required: "Introduce una contraseña",
    err_username_format: "Usuario: 3–24 caracteres, letras latinas, números, _ o .",
    err_password_short: "La contraseña debe tener al menos 4 caracteres",

    connecting: "Conectando…",
    create_account: "Crear cuenta",
    sign_in: "Entrar",
    have_account: "¿Ya tienes cuenta?",
    no_account: "¿No tienes cuenta?",
    switch_mode: "Cambiar",

    server_running: "Servidor activo",
    friends_connect: "Tus amigos se conectan a esta dirección",
    waiting: "Esperando conexión",
    connected: "Conectado",

    search: "Buscar",
    no_contacts: "Nadie por aquí todavía",

    online: "en línea",
    connecting_lower: "conectando…",
    offline: "sin conexión",
    typing: "escribiendo…",

    choose_chat: "Elige a quién escribir",
    contacts_hint: "Contactos y chats, en la lista de la izquierda",
    empty_chat: "Aquí todavía no hay nada",
    write_first: "Escribe primero — @{}",
    show_older: "Mostrar anteriores",
    message_hint: "Mensaje",
    no_connection: "Sin conexión con el servidor",

    profile: "Perfil",
    id_label: "ID",
    server_label: "Servidor",
    my_address: "Mi dirección",
    logout: "Cerrar sesión",

    settings: "Ajustes",
    settings_sub: "Apariencia y comodidad",
    theme_section: "APARIENCIA",
    language_section: "IDIOMA",
    interface_section: "INTERFAZ",
    scale_label: "Escala",
    animations_label: "Animaciones suaves",
    animations_hint: "Transiciones y efectos suaves",
    done: "Listo",
    copy_text: "Copiar texto",

    you_prefix: "Tú: ",
    unread_suffix: "nuevos",

    today: "Hoy",
    yesterday: "Ayer",
    yesterday_lower: "ayer",
    last_seen_at: "última vez a las {}",
    last_seen_yesterday: "visto ayer",
    last_seen_on: "visto el {}",
    last_seen_recently: "visto recientemente",

    theme_graphite: "Grafito",
    theme_graphite_hint: "Oscuro monocromo",
    theme_light: "Clara",
    theme_light_hint: "Día, papel",
    theme_emerald: "Esmeralda",
    theme_emerald_hint: "Oscuro con verde",
    theme_indigo: "Índigo",
    theme_indigo_hint: "Oscuro con azul",

    srv_bad_request: "Solicitud incorrecta",
    srv_username_format: "Usuario: 3–24 caracteres, letras latinas, números, _ o .",
    srv_password_short: "La contraseña debe tener al menos 4 caracteres",
    srv_name_taken: "Ese usuario ya está ocupado",
    srv_wrong_password: "Contraseña incorrecta",
    srv_user_not_found: "Usuario no encontrado",
    srv_login_first: "Primero inicia sesión",
    srv_too_long: "El mensaje es demasiado largo",

    connection_refused: "conexión rechazada",
    timeout: "tiempo agotado",
    network_unreachable: "red no disponible",
    server_unavailable: "El servidor {} no está disponible: {}",
    connection_lost: "Conexión perdida, reconectando…",
    server_silent: "El servidor no responde, reconectando…",

    months: [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ],
};

static CURRENT: RwLock<Lang> = RwLock::new(Lang::Ru);

pub fn lang() -> Lang {
    *CURRENT.read().unwrap_or_else(|e| e.into_inner())
}

pub fn set(next: Lang) {
    if next != lang() {
        *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = next;
    }
}

/// Подписи текущего языка.
pub fn s() -> &'static Strings {
    match lang() {
        Lang::Ru => &RU,
        Lang::En => &EN,
        Lang::De => &DE,
        Lang::Es => &ES,
    }
}

pub fn theme_name(id: &str) -> &'static str {
    let s = s();
    match id {
        "light" => s.theme_light,
        "emerald" => s.theme_emerald,
        "indigo" => s.theme_indigo,
        _ => s.theme_graphite,
    }
}

pub fn theme_hint(id: &str) -> &'static str {
    let s = s();
    match id {
        "light" => s.theme_light_hint,
        "emerald" => s.theme_emerald_hint,
        "indigo" => s.theme_indigo_hint,
        _ => s.theme_graphite_hint,
    }
}

/// Сервер отвечает по-русски; для других языков переводим известные тексты.
pub fn server_text(text: &str) -> String {
    if lang() == Lang::Ru {
        return text.to_string();
    }
    let s = s();
    match text {
        "Некорректный запрос" => s.srv_bad_request.into(),
        "Имя пользователя: 3–24 символа, латиница, цифры, _ или ." => {
            s.srv_username_format.into()
        }
        "Пароль должен быть не короче 4 символов" => s.srv_password_short.into(),
        "Такое имя пользователя уже занято" => s.srv_name_taken.into(),
        "Неверный пароль" => s.srv_wrong_password.into(),
        "Пользователь не найден" => s.srv_user_not_found.into(),
        "Сначала войдите в аккаунт" => s.srv_login_first.into(),
        "Сообщение слишком длинное" => s.srv_too_long.into(),
        other => other.to_string(),
    }
}
