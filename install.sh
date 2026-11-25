#!/usr/bin/env bash
# Установка Battle City, чтобы запускать игру без IDE и cargo run.
#
#   bash install.sh              — собрать release и установить (повторный запуск = обновление)
#   bash install.sh --uninstall  — убрать команду и ярлык (папку с игрой и рекордами не трогает)
#
# Что делает:
#   ~/.local/share/rust_battle_city/   — бинарник, config.ron, levels/, значок;
#                                        там же рекорд, клавиши, сложность, уровень
#   ~/.local/bin/battle-city           — команда для терминала (ссылка на бинарник)
#   ~/.local/share/applications/battle-city.desktop — ярлык в меню приложений (раздел «Игры»)
#
# Игра ищет config.ron и levels/ рядом с бинарником, поэтому запускается
# откуда угодно. config.ron при обновлении заменяется новым, только если вы
# его не меняли; изменённый остаётся, а новая версия кладётся рядом как
# config.ron.new. Клавиши из меню (keys.ron) из репозитория переносятся,
# если в папке игры своих ещё нет.
set -euo pipefail

APP=rust_battle_city
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/$APP"
BIN_DIR="$HOME/.local/bin"
DESKTOP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
LINK="$BIN_DIR/battle-city"
DESKTOP="$DESKTOP_DIR/battle-city.desktop"

cd "$(dirname "$0")"
[ -f Cargo.toml ] && grep -q "name = \"$APP\"" Cargo.toml || { echo "⛔ Запускайте из репозитория rust_battle_city"; exit 1; }

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$LINK" "$DESKTOP"
    command -v update-desktop-database >/dev/null && update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
    echo "✅ Команда и ярлык удалены. Игра, рекорды и настройки остались в $DEST"
    echo "   (удалить совсем: rm -rf \"$DEST\")"
    exit 0
fi

echo "▶ Сборка release (первый раз — пару минут)…"
cargo build --release

mkdir -p "$DEST/levels" "$BIN_DIR" "$DESKTOP_DIR"

# Бинарник: сначала во временный файл, потом переименование —
# не мешает, даже если старая версия игры сейчас запущена.
cp "target/release/$APP" "$DEST/$APP.tmp"
chmod +x "$DEST/$APP.tmp"
mv -f "$DEST/$APP.tmp" "$DEST/$APP"

# Карты из репозитория (свои карты с другими именами в папке остаются).
cp -f levels/* "$DEST/levels/"

# config.ron: если вы его в папке игры не меняли — обновляем до версии из
# репозитория; свой, изменённый, не затираем. «Не меняли» = файл совпадает
# с какой-то версией config.ron из истории git (то есть его когда-то положил
# сюда install.sh, а вы не правили).
untouched_config() {
    local f="$1" h
    command -v git >/dev/null && [ -d .git ] || return 1
    for h in $(git log --format=%H -- config.ron 2>/dev/null); do
        git show "$h:config.ron" 2>/dev/null | cmp -s - "$f" && return 0
    done
    return 1
}
if [ ! -f "$DEST/config.ron" ]; then
    cp config.ron "$DEST/config.ron"
elif cmp -s config.ron "$DEST/config.ron"; then
    rm -f "$DEST/config.ron.new"
elif untouched_config "$DEST/config.ron"; then
    cp config.ron "$DEST/config.ron"
    rm -f "$DEST/config.ron.new"
    echo "ℹ config.ron в $DEST обновлён до версии из репозитория (вы его не меняли)"
else
    cp config.ron "$DEST/config.ron.new"
    echo "ℹ config.ron в $DEST вы меняли — оставил ваш,"
    echo "  версия из репозитория рядом: config.ron.new (новые настройки и без неё работают по умолчанию)"
fi

# keys.ron — клавиши, назначенные в меню CONTROLS. Если в репозитории он
# есть (вы настраивали клавиши, запуская игру через cargo run), а в папке
# игры нет — переносим, чтобы установленная игра управлялась так же.
if [ -f keys.ron ] && [ ! -f "$DEST/keys.ron" ]; then
    cp keys.ron "$DEST/keys.ron"
    echo "ℹ клавиши из меню (keys.ron) перенесены в $DEST"
fi

cp assets/battle-city.svg "$DEST/battle-city.svg"
ln -sfn "$DEST/$APP" "$LINK"

cat > "$DESKTOP" <<DESK
[Desktop Entry]
Type=Application
Name=Battle City
Name[ru]=Танчики
Comment=Battle City for two players
Comment[ru]=Танчики на двоих
Exec="$DEST/$APP"
Path=$DEST
Icon=$DEST/battle-city.svg
Terminal=false
Categories=Game;ArcadeGame;
DESK
command -v update-desktop-database >/dev/null && update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo "✅ Установлено в $DEST"
echo "   Запуск: «Танчики» в меню приложений или команда battle-city"
case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) echo "ℹ $BIN_DIR нет в PATH — команда battle-city пока не найдётся (fish: fish_add_path ~/.local/bin)";;
esac
